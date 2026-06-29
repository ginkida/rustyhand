//! PeerNode — TCP server and client for the RustyHand Wire Protocol.
//!
//! A [`PeerNode`] binds a local TCP listener and accepts incoming connections
//! from other RustyHand kernels. It also connects outward to known peers. Each
//! connection performs a handshake to exchange identity and agent lists, then
//! enters a message dispatch loop.
//!
//! The [`PeerHandle`] trait abstracts the kernel's ability to respond to
//! remote requests (agent messages, discovery, etc.).

use crate::message::*;
use crate::registry::{PeerEntry, PeerRegistry, PeerState};

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

type HmacSha256 = Hmac<Sha256>;

/// Maximum allowed clock skew between a handshake's signed timestamp and the
/// receiver's clock. Handshakes outside this window are rejected as stale.
const HANDSHAKE_FRESHNESS_SECS: i64 = 60;
/// How long a seen handshake nonce is remembered for replay rejection. Must be
/// >= 2 * HANDSHAKE_FRESHNESS_SECS so no replay can slip past the freshness gate.
const NONCE_CACHE_TTL: Duration = Duration::from_secs(150);
/// Pre-authentication frame size cap (1 MB). A handshake is tiny; this bounds
/// memory an unauthenticated peer can force us to buffer before it proves auth.
const HANDSHAKE_MAX_SIZE: u32 = 1024 * 1024;
/// Per-read timeout for untrusted/outbound reads, so a stalling or malicious
/// peer cannot wedge the caller or pin a connection indefinitely.
const NETWORK_TIMEOUT: Duration = Duration::from_secs(30);
/// TCP connect timeout for outbound peer connections.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Cap on concurrently-handled inbound connections (slowloris / task-exhaustion
/// backstop). Excess connections wait for a permit rather than each spawning a
/// task that can be pinned forever pre-auth.
const MAX_INBOUND_CONNECTIONS: usize = 256;
/// Body read chunk size — bytes are appended as they arrive so we never commit
/// the full attacker-declared frame length up front.
const BODY_READ_CHUNK: usize = 64 * 1024;

/// Build the length-framed payload that the handshake HMAC signs.
///
/// Length-prefixing each field (rather than `format!("{nonce}{node_id}")`)
/// removes any ambiguity at field boundaries, and binding the timestamp ties
/// the signature to a point in time for replay rejection.
fn handshake_signed_data(nonce: &str, node_id: &str, timestamp: i64) -> Vec<u8> {
    let mut data = Vec::with_capacity(nonce.len() + node_id.len() + 16);
    for field in [nonce.as_bytes(), node_id.as_bytes()] {
        data.extend_from_slice(&(field.len() as u32).to_be_bytes());
        data.extend_from_slice(field);
    }
    data.extend_from_slice(&timestamp.to_be_bytes());
    data
}

/// Whether a handshake timestamp is within the accepted freshness window.
fn timestamp_is_fresh(timestamp: i64) -> bool {
    let now = chrono::Utc::now().timestamp();
    (now - timestamp).abs() <= HANDSHAKE_FRESHNESS_SECS
}

/// Generate HMAC-SHA256 signature for message authentication.
fn hmac_sign(secret: &str, data: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(data);
    hex::encode(mac.finalize().into_bytes())
}

/// Verify HMAC-SHA256 signature using constant-time comparison.
fn hmac_verify(secret: &str, data: &[u8], signature: &str) -> bool {
    let expected = hmac_sign(secret, data);
    subtle::ConstantTimeEq::ct_eq(expected.as_bytes(), signature.as_bytes()).into()
}

/// Errors from the wire protocol layer.
#[derive(Debug, Error)]
pub enum WireError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Message too large: {size} bytes (max {max})")]
    MessageTooLarge { size: u32, max: u32 },
    #[error("Protocol version mismatch: local={local}, remote={remote}")]
    VersionMismatch { local: u32, remote: u32 },
}

/// Maximum single message size (16 MB).
pub const MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

/// Configuration for a PeerNode.
#[derive(Debug, Clone)]
pub struct PeerConfig {
    /// Address to bind the listener on.
    pub listen_addr: SocketAddr,
    /// This node's unique ID.
    pub node_id: String,
    /// This node's human-readable name.
    pub node_name: String,
    /// Pre-shared key for HMAC-SHA256 authentication.
    /// Required — RHP refuses to start without it.
    pub shared_secret: String,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: uuid::Uuid::new_v4().to_string(),
            node_name: "rusty-hand-node".to_string(),
            shared_secret: String::new(),
        }
    }
}

/// Trait for the kernel to handle incoming remote requests.
///
/// The PeerNode calls these methods when it receives requests from
/// remote peers. The kernel implements this to route messages to
/// local agents.
#[async_trait]
pub trait PeerHandle: Send + Sync + 'static {
    /// List local agents as RemoteAgentInfo (for handshake and discovery).
    fn local_agents(&self) -> Vec<RemoteAgentInfo>;

    /// Send a message to a local agent and get the response.
    async fn handle_agent_message(
        &self,
        agent: &str,
        message: &str,
        sender: Option<&str>,
    ) -> Result<String, String>;

    /// Find local agents matching a query.
    fn discover_agents(&self, query: &str) -> Vec<RemoteAgentInfo>;

    /// Return the uptime of the local node in seconds.
    fn uptime_secs(&self) -> u64;
}

/// The local network node — listens for connections and connects to peers.
pub struct PeerNode {
    config: PeerConfig,
    registry: PeerRegistry,
    /// Actual bound address (useful when binding to port 0).
    local_addr: SocketAddr,
    /// Recently-seen handshake nonces, for replay rejection on inbound auth.
    nonce_cache: Mutex<HashMap<String, Instant>>,
    /// Limits concurrent inbound connections (DoS backstop).
    inbound_limit: Arc<Semaphore>,
}

impl PeerNode {
    /// Create and start listening on the configured address.
    pub async fn start(
        config: PeerConfig,
        registry: PeerRegistry,
        handle: Arc<dyn PeerHandle>,
    ) -> Result<(Arc<Self>, tokio::task::JoinHandle<()>), WireError> {
        // SECURITY: Require shared_secret for RHP
        if config.shared_secret.is_empty() {
            return Err(WireError::HandshakeFailed(
                "RHP requires shared_secret. Set [network] shared_secret in config.toml".into(),
            ));
        }

        let listener = TcpListener::bind(config.listen_addr).await?;
        let local_addr = listener.local_addr()?;

        info!(
            "RHP: listening on {} (node_id={})",
            local_addr, config.node_id
        );

        let node = Arc::new(Self {
            config,
            registry: registry.clone(),
            local_addr,
            nonce_cache: Mutex::new(HashMap::new()),
            inbound_limit: Arc::new(Semaphore::new(MAX_INBOUND_CONNECTIONS)),
        });

        let node_clone = Arc::clone(&node);
        let accept_handle = tokio::spawn(async move {
            Self::accept_loop(listener, node_clone, registry, handle).await;
        });

        Ok((node, accept_handle))
    }

    /// Get the actual bound address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Get the node ID.
    pub fn node_id(&self) -> &str {
        &self.config.node_id
    }

    /// Get a reference to the peer registry.
    pub fn registry(&self) -> &PeerRegistry {
        &self.registry
    }

    /// Record a handshake nonce, returning `true` if it is fresh (not seen
    /// within [`NONCE_CACHE_TTL`]) and `false` if it is a replay. Expired
    /// entries are evicted on each call to keep the cache bounded.
    fn register_nonce(&self, nonce: &str) -> bool {
        let mut cache = self
            .nonce_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();
        cache.retain(|_, seen| now.duration_since(*seen) < NONCE_CACHE_TTL);
        if cache.contains_key(nonce) {
            return false;
        }
        cache.insert(nonce.to_string(), now);
        true
    }

    /// Connect to a remote peer and perform the handshake.
    pub async fn connect_to_peer(
        &self,
        addr: SocketAddr,
        handle: Arc<dyn PeerHandle>,
    ) -> Result<(), WireError> {
        info!("RHP: connecting to peer at {}", addr);
        let stream = match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
            Ok(r) => r?,
            Err(_) => {
                return Err(WireError::HandshakeFailed(format!(
                    "connect to {addr} timed out"
                )))
            }
        };
        let (mut reader, mut writer) = stream.into_split();

        // Send our handshake with HMAC authentication
        let nonce = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp();
        let auth_data = handshake_signed_data(&nonce, &self.config.node_id, timestamp);
        let auth_hmac = hmac_sign(&self.config.shared_secret, &auth_data);

        let handshake = WireMessage {
            id: uuid::Uuid::new_v4().to_string(),
            kind: WireMessageKind::Request(WireRequest::Handshake {
                node_id: self.config.node_id.clone(),
                node_name: self.config.node_name.clone(),
                protocol_version: PROTOCOL_VERSION,
                agents: handle.local_agents(),
                nonce,
                timestamp,
                auth_hmac,
            }),
        };
        write_message(&mut writer, &handshake).await?;

        // Read their handshake ack (bounded + timed out — peer may stall)
        let response =
            read_message_bounded(&mut reader, HANDSHAKE_MAX_SIZE, NETWORK_TIMEOUT).await?;
        match &response.kind {
            WireMessageKind::Response(WireResponse::HandshakeAck {
                node_id,
                node_name,
                protocol_version,
                agents,
                nonce: ack_nonce,
                timestamp: ack_ts,
                auth_hmac: ack_hmac,
            }) => {
                if *protocol_version != PROTOCOL_VERSION {
                    return Err(WireError::VersionMismatch {
                        local: PROTOCOL_VERSION,
                        remote: *protocol_version,
                    });
                }

                // SECURITY: Verify the ack HMAC over the length-framed payload
                let expected_data = handshake_signed_data(ack_nonce, node_id, *ack_ts);
                if !hmac_verify(&self.config.shared_secret, &expected_data, ack_hmac) {
                    return Err(WireError::HandshakeFailed(
                        "HMAC verification failed on HandshakeAck".into(),
                    ));
                }

                // SECURITY: Reject a stale ack (replay / clock skew).
                if !timestamp_is_fresh(*ack_ts) {
                    return Err(WireError::HandshakeFailed(
                        "HandshakeAck timestamp outside freshness window".into(),
                    ));
                }

                info!(
                    "RHP: handshake complete with {} ({}) — {} agents",
                    node_name,
                    node_id,
                    agents.len()
                );
                self.registry.add_peer(PeerEntry {
                    node_id: node_id.clone(),
                    node_name: node_name.clone(),
                    address: addr,
                    agents: agents.clone(),
                    state: PeerState::Connected,
                    connected_at: chrono::Utc::now(),
                    protocol_version: *protocol_version,
                });
            }
            WireMessageKind::Response(WireResponse::Error { code, message }) => {
                return Err(WireError::HandshakeFailed(format!(
                    "Remote error {code}: {message}"
                )));
            }
            _ => {
                return Err(WireError::HandshakeFailed(
                    "Unexpected response to handshake".to_string(),
                ));
            }
        }

        // Extract the peer node_id for the connection loop
        let peer_node_id = match &response.kind {
            WireMessageKind::Response(WireResponse::HandshakeAck { node_id, .. }) => {
                node_id.clone()
            }
            _ => {
                return Err(WireError::HandshakeFailed(
                    "Unexpected response kind after handshake validation".to_string(),
                ));
            }
        };

        // Spawn a task to handle ongoing communication
        let registry = self.registry.clone();
        tokio::spawn(async move {
            if let Err(e) =
                connection_loop(&mut reader, &mut writer, &peer_node_id, &registry, &*handle).await
            {
                debug!("RHP: connection to {} ended: {}", peer_node_id, e);
            }
            registry.mark_disconnected(&peer_node_id);
        });

        Ok(())
    }

    /// Send a message to a specific peer and await the response.
    ///
    /// SECURITY: Opens a new connection to the peer, performs a full HMAC
    /// handshake, sends the agent message, and reads the response.
    pub async fn send_to_peer(
        &self,
        node_id: &str,
        agent: &str,
        message: &str,
        sender: Option<&str>,
        handle: Arc<dyn PeerHandle>,
    ) -> Result<String, WireError> {
        let peer = self
            .registry
            .get_peer(node_id)
            .ok_or_else(|| WireError::HandshakeFailed(format!("Unknown peer: {node_id}")))?;

        let stream =
            match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(peer.address)).await {
                Ok(r) => r?,
                Err(_) => {
                    return Err(WireError::HandshakeFailed(format!(
                        "connect to {} timed out",
                        peer.address
                    )))
                }
            };
        let (mut reader, mut writer) = stream.into_split();

        // SECURITY: Perform HMAC handshake before sending any data
        let nonce = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp();
        let auth_data = handshake_signed_data(&nonce, &self.config.node_id, timestamp);
        let auth_hmac = hmac_sign(&self.config.shared_secret, &auth_data);

        let handshake = WireMessage {
            id: uuid::Uuid::new_v4().to_string(),
            kind: WireMessageKind::Request(WireRequest::Handshake {
                node_id: self.config.node_id.clone(),
                node_name: self.config.node_name.clone(),
                protocol_version: PROTOCOL_VERSION,
                agents: handle.local_agents(),
                nonce,
                timestamp,
                auth_hmac,
            }),
        };
        write_message(&mut writer, &handshake).await?;

        // Verify handshake ack (bounded + timed out)
        let ack = read_message_bounded(&mut reader, HANDSHAKE_MAX_SIZE, NETWORK_TIMEOUT).await?;
        match &ack.kind {
            WireMessageKind::Response(WireResponse::HandshakeAck {
                node_id: ack_node_id,
                nonce: ack_nonce,
                timestamp: ack_ts,
                auth_hmac: ack_hmac,
                protocol_version,
                ..
            }) => {
                if *protocol_version != PROTOCOL_VERSION {
                    return Err(WireError::VersionMismatch {
                        local: PROTOCOL_VERSION,
                        remote: *protocol_version,
                    });
                }
                let expected_data = handshake_signed_data(ack_nonce, ack_node_id, *ack_ts);
                if !hmac_verify(&self.config.shared_secret, &expected_data, ack_hmac) {
                    return Err(WireError::HandshakeFailed(
                        "HMAC verification failed on HandshakeAck".into(),
                    ));
                }
                if !timestamp_is_fresh(*ack_ts) {
                    return Err(WireError::HandshakeFailed(
                        "HandshakeAck timestamp outside freshness window".into(),
                    ));
                }
            }
            WireMessageKind::Response(WireResponse::Error { code, message }) => {
                return Err(WireError::HandshakeFailed(format!(
                    "Remote error {code}: {message}"
                )));
            }
            _ => {
                return Err(WireError::HandshakeFailed(
                    "Unexpected response to handshake".to_string(),
                ));
            }
        }

        // Now send the actual agent message over the authenticated connection
        let msg = WireMessage {
            id: uuid::Uuid::new_v4().to_string(),
            kind: WireMessageKind::Request(WireRequest::AgentMessage {
                agent: agent.to_string(),
                message: message.to_string(),
                sender: sender.map(|s| s.to_string()),
            }),
        };
        write_message(&mut writer, &msg).await?;

        let response = read_message_bounded(&mut reader, MAX_MESSAGE_SIZE, NETWORK_TIMEOUT).await?;
        match response.kind {
            WireMessageKind::Response(WireResponse::AgentResponse { text }) => Ok(text),
            WireMessageKind::Response(WireResponse::Error { code, message }) => Err(
                WireError::HandshakeFailed(format!("Remote error {code}: {message}")),
            ),
            _ => Err(WireError::HandshakeFailed(
                "Unexpected response type".to_string(),
            )),
        }
    }

    /// Internal accept loop — runs in a spawned task.
    async fn accept_loop(
        listener: TcpListener,
        node: Arc<PeerNode>,
        registry: PeerRegistry,
        handle: Arc<dyn PeerHandle>,
    ) {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("RHP: accepted connection from {}", addr);
                    // SECURITY: bound concurrent inbound connections so an
                    // attacker cannot exhaust tasks/sockets by opening many
                    // connections and never authenticating.
                    let permit = match Arc::clone(&node.inbound_limit).acquire_owned().await {
                        Ok(p) => p,
                        Err(_) => return, // semaphore closed — node shutting down
                    };
                    let node = Arc::clone(&node);
                    let registry = registry.clone();
                    let handle = Arc::clone(&handle);
                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_inbound(stream, addr, &node, &registry, &*handle).await
                        {
                            debug!("RHP: inbound connection from {} ended: {}", addr, e);
                        }
                        drop(permit);
                    });
                }
                Err(e) => {
                    error!("RHP: accept error: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Handle a single inbound connection: perform handshake, then enter message loop.
    async fn handle_inbound(
        stream: TcpStream,
        addr: SocketAddr,
        node: &PeerNode,
        registry: &PeerRegistry,
        handle: &dyn PeerHandle,
    ) -> Result<(), WireError> {
        let (mut reader, mut writer) = stream.into_split();

        // Read the incoming handshake request. SECURITY: this is pre-auth, so
        // it is bounded to a small frame size and a read timeout — an
        // unauthenticated peer cannot force a large allocation or pin the task.
        let msg = read_message_bounded(&mut reader, HANDSHAKE_MAX_SIZE, NETWORK_TIMEOUT).await?;
        let peer_node_id = match &msg.kind {
            WireMessageKind::Request(WireRequest::Handshake {
                node_id,
                node_name,
                protocol_version,
                agents,
                nonce,
                timestamp,
                auth_hmac,
            }) => {
                if *protocol_version != PROTOCOL_VERSION {
                    let err_resp = WireMessage {
                        id: msg.id.clone(),
                        kind: WireMessageKind::Response(WireResponse::Error {
                            code: 1,
                            message: format!(
                                "Protocol version mismatch: expected {}, got {}",
                                PROTOCOL_VERSION, protocol_version
                            ),
                        }),
                    };
                    write_message(&mut writer, &err_resp).await?;
                    return Err(WireError::VersionMismatch {
                        local: PROTOCOL_VERSION,
                        remote: *protocol_version,
                    });
                }

                // SECURITY: Verify the incoming HMAC over the length-framed
                // (nonce, node_id, timestamp) payload.
                let expected_data = handshake_signed_data(nonce, node_id, *timestamp);
                if !hmac_verify(&node.config.shared_secret, &expected_data, auth_hmac) {
                    let err_resp = WireMessage {
                        id: msg.id.clone(),
                        kind: WireMessageKind::Response(WireResponse::Error {
                            code: 403,
                            message: "HMAC authentication failed".to_string(),
                        }),
                    };
                    write_message(&mut writer, &err_resp).await?;
                    return Err(WireError::HandshakeFailed(
                        "HMAC verification failed on incoming Handshake".into(),
                    ));
                }

                // SECURITY: Reject replays — the handshake must be recent and
                // its nonce must not have been seen before. Without this, a
                // captured handshake frame could be replayed verbatim over the
                // plaintext transport to gain an authenticated session.
                if !timestamp_is_fresh(*timestamp) || !node.register_nonce(nonce) {
                    let err_resp = WireMessage {
                        id: msg.id.clone(),
                        kind: WireMessageKind::Response(WireResponse::Error {
                            code: 403,
                            message: "Stale or replayed handshake".to_string(),
                        }),
                    };
                    write_message(&mut writer, &err_resp).await?;
                    return Err(WireError::HandshakeFailed(
                        "Rejected stale/replayed handshake".into(),
                    ));
                }

                // Send handshake ack with our own fresh, timestamped HMAC
                let ack_nonce = uuid::Uuid::new_v4().to_string();
                let ack_ts = chrono::Utc::now().timestamp();
                let ack_auth_data = handshake_signed_data(&ack_nonce, &node.config.node_id, ack_ts);
                let ack_hmac = hmac_sign(&node.config.shared_secret, &ack_auth_data);

                let ack = WireMessage {
                    id: msg.id.clone(),
                    kind: WireMessageKind::Response(WireResponse::HandshakeAck {
                        node_id: node.config.node_id.clone(),
                        node_name: node.config.node_name.clone(),
                        protocol_version: PROTOCOL_VERSION,
                        agents: handle.local_agents(),
                        nonce: ack_nonce,
                        timestamp: ack_ts,
                        auth_hmac: ack_hmac,
                    }),
                };
                write_message(&mut writer, &ack).await?;

                info!(
                    "RHP: handshake with {} ({}) from {} — {} agents",
                    node_name,
                    node_id,
                    addr,
                    agents.len()
                );

                // Register the peer
                registry.add_peer(PeerEntry {
                    node_id: node_id.clone(),
                    node_name: node_name.clone(),
                    address: addr,
                    agents: agents.clone(),
                    state: PeerState::Connected,
                    connected_at: chrono::Utc::now(),
                    protocol_version: *protocol_version,
                });

                node_id.clone()
            }
            // SECURITY: Reject all non-Handshake initial messages.
            // Clients MUST complete HMAC-authenticated handshake before sending
            // any requests (AgentMessage, Ping, Discover, etc.).
            _ => {
                warn!(
                    "RHP: rejected unauthenticated message from {} — handshake required",
                    addr
                );
                let err_resp = WireMessage {
                    id: msg.id.clone(),
                    kind: WireMessageKind::Response(WireResponse::Error {
                        code: 401,
                        message: "Authentication required: complete HMAC handshake first"
                            .to_string(),
                    }),
                };
                write_message(&mut writer, &err_resp).await?;
                return Err(WireError::HandshakeFailed(
                    "Rejected unauthenticated request — handshake required".into(),
                ));
            }
        };

        // Enter the message dispatch loop
        if let Err(e) =
            connection_loop(&mut reader, &mut writer, &peer_node_id, registry, handle).await
        {
            debug!("RHP: connection with {} ended: {}", peer_node_id, e);
        }
        registry.mark_disconnected(&peer_node_id);

        Ok(())
    }
}

/// Read/write message loop for an established connection.
async fn connection_loop(
    reader: &mut tokio::net::tcp::OwnedReadHalf,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    peer_node_id: &str,
    registry: &PeerRegistry,
    handle: &dyn PeerHandle,
) -> Result<(), WireError> {
    loop {
        let msg = match read_message(reader).await {
            Ok(m) => m,
            Err(WireError::ConnectionClosed) => return Ok(()),
            Err(e) => return Err(e),
        };

        match &msg.kind {
            // Handle notifications (no response needed)
            WireMessageKind::Notification(notif) => {
                handle_notification(peer_node_id, notif, registry);
            }
            // Handle requests (produce response)
            WireMessageKind::Request(_) => {
                // We need the node for uptime; create a minimal shim
                let response = handle_request_in_loop(&msg, handle).await;
                write_message(writer, &response).await?;
            }
            // We don't expect to receive responses in the connection loop
            WireMessageKind::Response(_) => {
                warn!(
                    "RHP: unexpected response message from {}: {:?}",
                    peer_node_id, msg.id
                );
            }
        }
    }
}

/// Handle request inside the connection loop (no PeerNode reference needed for most ops).
async fn handle_request_in_loop(msg: &WireMessage, handle: &dyn PeerHandle) -> WireMessage {
    let kind = match &msg.kind {
        WireMessageKind::Request(WireRequest::Ping) => {
            WireMessageKind::Response(WireResponse::Pong {
                uptime_secs: handle.uptime_secs(),
            })
        }
        WireMessageKind::Request(WireRequest::Discover { query }) => {
            let agents = handle.discover_agents(query);
            WireMessageKind::Response(WireResponse::DiscoverResult { agents })
        }
        WireMessageKind::Request(WireRequest::AgentMessage {
            agent,
            message,
            sender,
        }) => match handle
            .handle_agent_message(agent, message, sender.as_deref())
            .await
        {
            Ok(text) => WireMessageKind::Response(WireResponse::AgentResponse { text }),
            Err(e) => WireMessageKind::Response(WireResponse::Error {
                code: 500,
                message: e,
            }),
        },
        _ => WireMessageKind::Response(WireResponse::Error {
            code: 400,
            message: "Unexpected request in connection loop".to_string(),
        }),
    };

    WireMessage {
        id: msg.id.clone(),
        kind,
    }
}

/// Process an incoming notification.
fn handle_notification(peer_node_id: &str, notif: &WireNotification, registry: &PeerRegistry) {
    match notif {
        WireNotification::AgentSpawned { agent } => {
            info!(
                "RHP: peer {} spawned agent {} ({})",
                peer_node_id, agent.name, agent.id
            );
            registry.add_agent(peer_node_id, agent.clone());
        }
        WireNotification::AgentTerminated { agent_id } => {
            info!("RHP: peer {} terminated agent {}", peer_node_id, agent_id);
            registry.remove_agent(peer_node_id, agent_id);
        }
        WireNotification::ShuttingDown => {
            info!("RHP: peer {} is shutting down", peer_node_id);
            registry.mark_disconnected(peer_node_id);
        }
    }
}

/// Write a framed message (4-byte length + JSON) to a TCP stream.
pub async fn write_message(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    msg: &WireMessage,
) -> Result<(), WireError> {
    let bytes = encode_message(msg)?;
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}

/// Read a framed message (4-byte length + JSON) from a TCP stream.
///
/// Used for established (post-handshake) connections: the full
/// [`MAX_MESSAGE_SIZE`] cap applies and there is no idle timeout, so a quiet
/// authenticated peer is not disconnected. The body is read in chunks so the
/// full declared length is never pre-allocated before the bytes arrive.
pub async fn read_message(
    reader: &mut tokio::net::tcp::OwnedReadHalf,
) -> Result<WireMessage, WireError> {
    read_framed(reader, MAX_MESSAGE_SIZE, None).await
}

/// Read a framed message with an explicit size cap and a per-read timeout.
///
/// Use this for untrusted (pre-authentication) and outbound request/response
/// reads: a stalling or malicious peer hits the timeout instead of pinning the
/// caller forever, and an oversized declared length is rejected before any
/// large allocation. SECURITY-relevant — see [`read_message`] for the trusted
/// post-handshake variant.
pub async fn read_message_bounded(
    reader: &mut tokio::net::tcp::OwnedReadHalf,
    max_size: u32,
    timeout: Duration,
) -> Result<WireMessage, WireError> {
    read_framed(reader, max_size, Some(timeout)).await
}

/// Shared framed-read implementation. Reads the 4-byte length header, enforces
/// `max_size`, then reads the body in [`BODY_READ_CHUNK`]-sized reads, growing
/// the buffer as bytes actually arrive (never committing the full declared
/// length up front). When `timeout` is `Some`, each underlying read is bounded.
async fn read_framed(
    reader: &mut tokio::net::tcp::OwnedReadHalf,
    max_size: u32,
    timeout: Option<Duration>,
) -> Result<WireMessage, WireError> {
    let mut header = [0u8; 4];
    let header_res = match timeout {
        Some(d) => match tokio::time::timeout(d, reader.read_exact(&mut header)).await {
            Ok(r) => r,
            Err(_) => return Err(WireError::ConnectionClosed),
        },
        None => reader.read_exact(&mut header).await,
    };
    match header_res {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(WireError::ConnectionClosed);
        }
        Err(e) => return Err(WireError::Io(e)),
    }

    let len = decode_length(&header);
    if len > max_size {
        return Err(WireError::MessageTooLarge {
            size: len,
            max: max_size,
        });
    }

    let mut body = Vec::new();
    let mut remaining = len as usize;
    let mut buf = vec![0u8; BODY_READ_CHUNK];
    while remaining > 0 {
        let want = remaining.min(buf.len());
        let n = match timeout {
            Some(d) => match tokio::time::timeout(d, reader.read(&mut buf[..want])).await {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => return Err(WireError::Io(e)),
                Err(_) => return Err(WireError::ConnectionClosed),
            },
            None => reader.read(&mut buf[..want]).await.map_err(WireError::Io)?,
        };
        if n == 0 {
            return Err(WireError::ConnectionClosed); // EOF mid-frame
        }
        body.extend_from_slice(&buf[..n]);
        remaining -= n;
    }

    let msg = decode_message(&body)?;
    Ok(msg)
}

/// Broadcast a notification to all connected peers.
pub async fn broadcast_notification(
    registry: &PeerRegistry,
    notification: WireNotification,
) -> Vec<(String, WireError)> {
    let peers = registry.connected_peers();
    let mut errors = Vec::new();

    for peer in peers {
        let msg = WireMessage {
            id: uuid::Uuid::new_v4().to_string(),
            kind: WireMessageKind::Notification(notification.clone()),
        };

        match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(peer.address)).await {
            Ok(Ok(stream)) => {
                let (_, mut writer) = stream.into_split();
                if let Err(e) = write_message(&mut writer, &msg).await {
                    errors.push((peer.node_id.clone(), e));
                }
            }
            Ok(Err(e)) => {
                errors.push((peer.node_id.clone(), WireError::Io(e)));
            }
            Err(_) => {
                errors.push((
                    peer.node_id.clone(),
                    WireError::HandshakeFailed(format!("connect to {} timed out", peer.address)),
                ));
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Minimal PeerHandle for testing.
    struct TestHandle {
        agents: Vec<RemoteAgentInfo>,
        uptime: AtomicU64,
    }

    impl TestHandle {
        fn new() -> Self {
            Self {
                agents: vec![RemoteAgentInfo {
                    id: "test-agent-1".to_string(),
                    name: "echo".to_string(),
                    description: "Echo agent".to_string(),
                    tags: vec!["test".to_string()],
                    tools: vec![],
                    state: "running".to_string(),
                }],
                uptime: AtomicU64::new(42),
            }
        }
    }

    async fn start_test_node(
        config: PeerConfig,
        registry: PeerRegistry,
        handle: Arc<dyn PeerHandle>,
    ) -> Option<(Arc<PeerNode>, tokio::task::JoinHandle<()>)> {
        match PeerNode::start(config, registry, handle).await {
            Ok(v) => Some(v),
            Err(WireError::Io(e)) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!("Skipping peer network test: local TCP bind denied: {e}");
                None
            }
            Err(e) => panic!("Failed to start peer node: {e}"),
        }
    }

    async fn connect_stream(addr: SocketAddr) -> Option<TcpStream> {
        match TcpStream::connect(addr).await {
            Ok(stream) => Some(stream),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!("Skipping peer network test: TCP connect denied: {e}");
                None
            }
            Err(e) => panic!("Failed to connect to peer test server: {e}"),
        }
    }

    macro_rules! require_network {
        ($expr:expr) => {
            match $expr {
                Some(v) => v,
                None => return,
            }
        };
    }

    #[async_trait]
    impl PeerHandle for TestHandle {
        fn local_agents(&self) -> Vec<RemoteAgentInfo> {
            self.agents.clone()
        }

        async fn handle_agent_message(
            &self,
            agent: &str,
            message: &str,
            _sender: Option<&str>,
        ) -> Result<String, String> {
            Ok(format!("Echo from {agent}: {message}"))
        }

        fn discover_agents(&self, query: &str) -> Vec<RemoteAgentInfo> {
            let q = query.to_lowercase();
            self.agents
                .iter()
                .filter(|a| a.name.to_lowercase().contains(&q))
                .cloned()
                .collect()
        }

        fn uptime_secs(&self) -> u64 {
            self.uptime.load(Ordering::Relaxed)
        }
    }

    #[tokio::test]
    async fn test_peer_start_and_connect() {
        let registry1 = PeerRegistry::new();
        let handle1 = Arc::new(TestHandle::new());

        let config1 = PeerConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: "node-1".to_string(),
            node_name: "kernel-1".to_string(),
            shared_secret: "test-secret-for-unit-tests".to_string(),
        };
        let (node1, _task1) =
            require_network!(start_test_node(config1, registry1.clone(), handle1.clone()).await);

        // Start a second node and connect to the first
        let registry2 = PeerRegistry::new();
        let handle2 = Arc::new(TestHandle::new());
        let config2 = PeerConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: "node-2".to_string(),
            node_name: "kernel-2".to_string(),
            shared_secret: "test-secret-for-unit-tests".to_string(),
        };
        let (node2, _task2) =
            require_network!(start_test_node(config2, registry2.clone(), handle2.clone()).await);

        // Node2 connects to Node1
        if let Err(err) = node2.connect_to_peer(node1.local_addr(), handle2).await {
            if matches!(&err, WireError::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied)
            {
                eprintln!("Skipping peer network test: outbound connect denied: {err}");
                return;
            }
            panic!("Failed to connect peer nodes: {err}");
        }

        // Registry2 should now have node-1 as a connected peer
        assert_eq!(registry2.connected_count(), 1);
        let peer = registry2.get_peer("node-1").unwrap();
        assert_eq!(peer.node_name, "kernel-1");
        assert_eq!(peer.agents.len(), 1);
        assert_eq!(peer.agents[0].name, "echo");

        // Registry1 should have node-2 (from inbound handshake)
        // Give the accept loop a moment to process
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(registry1.connected_count(), 1);
    }

    #[tokio::test]
    async fn test_unauthenticated_agent_message_rejected() {
        let registry = PeerRegistry::new();
        let handle = Arc::new(TestHandle::new());

        let config = PeerConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: "server".to_string(),
            node_name: "server-node".to_string(),
            shared_secret: "test-secret-for-unit-tests".to_string(),
        };
        let (node, _task) =
            require_network!(start_test_node(config, registry.clone(), handle.clone()).await);

        // SECURITY TEST: Sending an AgentMessage without handshake must be rejected
        let addr = node.local_addr();
        let stream = require_network!(connect_stream(addr).await);
        let (mut reader, mut writer) = stream.into_split();

        let msg = WireMessage {
            id: "req-1".to_string(),
            kind: WireMessageKind::Request(WireRequest::AgentMessage {
                agent: "echo".to_string(),
                message: "Hello, world!".to_string(),
                sender: Some("client".to_string()),
            }),
        };
        write_message(&mut writer, &msg).await.unwrap();

        let response = read_message(&mut reader).await.unwrap();
        assert_eq!(response.id, "req-1");
        match response.kind {
            WireMessageKind::Response(WireResponse::Error { code, message }) => {
                assert_eq!(code, 401);
                assert!(
                    message.contains("handshake"),
                    "Expected handshake-required error, got: {message}"
                );
            }
            other => panic!("Expected Error(401), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_unauthenticated_ping_rejected() {
        let registry = PeerRegistry::new();
        let handle = Arc::new(TestHandle::new());

        let config = PeerConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: "server".to_string(),
            node_name: "server-node".to_string(),
            shared_secret: "test-secret-for-unit-tests".to_string(),
        };
        let (node, _task) = require_network!(start_test_node(config, registry, handle).await);

        // SECURITY TEST: Sending a Ping without handshake must be rejected
        let stream = require_network!(connect_stream(node.local_addr()).await);
        let (mut reader, mut writer) = stream.into_split();

        let msg = WireMessage {
            id: "ping-1".to_string(),
            kind: WireMessageKind::Request(WireRequest::Ping),
        };
        write_message(&mut writer, &msg).await.unwrap();

        let response = read_message(&mut reader).await.unwrap();
        match response.kind {
            WireMessageKind::Response(WireResponse::Error { code, .. }) => {
                assert_eq!(code, 401);
            }
            other => panic!("Expected Error(401), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_unauthenticated_discover_rejected() {
        let registry = PeerRegistry::new();
        let handle = Arc::new(TestHandle::new());

        let config = PeerConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: "server".to_string(),
            node_name: "server-node".to_string(),
            shared_secret: "test-secret-for-unit-tests".to_string(),
        };
        let (node, _task) = require_network!(start_test_node(config, registry, handle).await);

        // SECURITY TEST: Sending a Discover without handshake must be rejected
        let stream = require_network!(connect_stream(node.local_addr()).await);
        let (mut reader, mut writer) = stream.into_split();

        let msg = WireMessage {
            id: "disc-1".to_string(),
            kind: WireMessageKind::Request(WireRequest::Discover {
                query: "echo".to_string(),
            }),
        };
        write_message(&mut writer, &msg).await.unwrap();

        let response = read_message(&mut reader).await.unwrap();
        match response.kind {
            WireMessageKind::Response(WireResponse::Error { code, .. }) => {
                assert_eq!(code, 401);
            }
            other => panic!("Expected Error(401), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handshake_and_message_loop() {
        let registry1 = PeerRegistry::new();
        let handle1 = Arc::new(TestHandle::new());

        let config1 = PeerConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: "node-a".to_string(),
            node_name: "kernel-a".to_string(),
            shared_secret: "test-secret-for-unit-tests".to_string(),
        };
        let (node1, _task1) =
            require_network!(start_test_node(config1, registry1.clone(), handle1.clone()).await);

        let registry2 = PeerRegistry::new();
        let handle2 = Arc::new(TestHandle::new());
        let config2 = PeerConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: "node-b".to_string(),
            node_name: "kernel-b".to_string(),
            shared_secret: "test-secret-for-unit-tests".to_string(),
        };
        let (node2, _task2) =
            require_network!(start_test_node(config2, registry2.clone(), handle2.clone()).await);

        // Connect node2 → node1
        if let Err(err) = node2.connect_to_peer(node1.local_addr(), handle2).await {
            if matches!(&err, WireError::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied)
            {
                eprintln!("Skipping peer network test: outbound connect denied: {err}");
                return;
            }
            panic!("Failed to connect peer nodes: {err}");
        }

        // Both should see each other
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(registry2.connected_count(), 1);
        assert_eq!(registry1.connected_count(), 1);

        // Verify agent discovery across the wire
        let remote_agents = registry2.find_agents("echo");
        assert_eq!(remote_agents.len(), 1);
        assert_eq!(remote_agents[0].peer_node_id, "node-a");
    }

    #[test]
    fn test_peer_config_default() {
        let config = PeerConfig::default();
        assert_eq!(config.node_name, "rusty-hand-node");
        assert!(!config.node_id.is_empty());
    }

    /// Build a fully-valid handshake frame for `secret` with the given nonce
    /// and timestamp (used by the replay/freshness security tests).
    fn signed_handshake(secret: &str, nonce: &str, timestamp: i64) -> WireMessage {
        let auth_hmac = hmac_sign(secret, &handshake_signed_data(nonce, "client", timestamp));
        WireMessage {
            id: "hs".to_string(),
            kind: WireMessageKind::Request(WireRequest::Handshake {
                node_id: "client".to_string(),
                node_name: "client-node".to_string(),
                protocol_version: PROTOCOL_VERSION,
                agents: vec![],
                nonce: nonce.to_string(),
                timestamp,
                auth_hmac,
            }),
        }
    }

    #[tokio::test]
    async fn test_handshake_replay_rejected() {
        let secret = "test-secret-for-unit-tests";
        let config = PeerConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: "server".to_string(),
            node_name: "server-node".to_string(),
            shared_secret: secret.to_string(),
        };
        let (node, _task) = require_network!(
            start_test_node(config, PeerRegistry::new(), Arc::new(TestHandle::new())).await
        );
        let addr = node.local_addr();

        // A single valid handshake (fresh timestamp, correct HMAC).
        let handshake = signed_handshake(secret, "replay-nonce-1", chrono::Utc::now().timestamp());

        // First use is accepted.
        let stream = require_network!(connect_stream(addr).await);
        let (mut reader, mut writer) = stream.into_split();
        write_message(&mut writer, &handshake).await.unwrap();
        let resp = read_message(&mut reader).await.unwrap();
        assert!(
            matches!(
                resp.kind,
                WireMessageKind::Response(WireResponse::HandshakeAck { .. })
            ),
            "first handshake should be accepted"
        );

        // Replaying the identical frame on a fresh connection is rejected.
        let stream = require_network!(connect_stream(addr).await);
        let (mut reader, mut writer) = stream.into_split();
        write_message(&mut writer, &handshake).await.unwrap();
        let resp = read_message(&mut reader).await.unwrap();
        match resp.kind {
            WireMessageKind::Response(WireResponse::Error { code, .. }) => assert_eq!(code, 403),
            other => panic!("Expected Error(403) on replay, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_stale_handshake_rejected() {
        let secret = "test-secret-for-unit-tests";
        let config = PeerConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            node_id: "server".to_string(),
            node_name: "server-node".to_string(),
            shared_secret: secret.to_string(),
        };
        let (node, _task) = require_network!(
            start_test_node(config, PeerRegistry::new(), Arc::new(TestHandle::new())).await
        );

        // A handshake whose HMAC is valid but whose timestamp is far in the
        // past must be rejected (outside the freshness window).
        let stale_ts = chrono::Utc::now().timestamp() - 3600;
        let handshake = signed_handshake(secret, "stale-nonce-1", stale_ts);

        let stream = require_network!(connect_stream(node.local_addr()).await);
        let (mut reader, mut writer) = stream.into_split();
        write_message(&mut writer, &handshake).await.unwrap();
        let resp = read_message(&mut reader).await.unwrap();
        match resp.kind {
            WireMessageKind::Response(WireResponse::Error { code, .. }) => assert_eq!(code, 403),
            other => panic!("Expected Error(403) on stale handshake, got {other:?}"),
        }
    }
}
