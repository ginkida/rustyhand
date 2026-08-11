//! MCP (Model Context Protocol) client — connect to external MCP servers.
//!
//! MCP uses JSON-RPC 2.0 over stdio or HTTP+SSE. This module lets RustyHand
//! agents use tools from any MCP server (100+ available: GitHub, filesystem,
//! databases, APIs, etc.).
//!
//! All MCP tools are namespaced with `mcp_{server}_{tool}` to prevent collisions.

use rusty_hand_types::tool::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Configuration types
// ---------------------------------------------------------------------------

/// Configuration for an MCP server connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Display name for this server (used in tool namespacing).
    pub name: String,
    /// Transport configuration.
    pub transport: McpTransport,
    /// Request timeout in seconds (default: 30).
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Environment variables to pass through to the subprocess (sandboxed).
    #[serde(default)]
    pub env: Vec<String>,
}

fn default_timeout() -> u64 {
    30
}

/// Transport type for MCP server connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransport {
    /// Subprocess with JSON-RPC over stdin/stdout.
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// HTTP Server-Sent Events.
    Sse { url: String },
}

// ---------------------------------------------------------------------------
// Connection types
// ---------------------------------------------------------------------------

/// An active connection to an MCP server.
pub struct McpConnection {
    /// Configuration for this connection.
    config: McpServerConfig,
    /// Tools discovered from the server via tools/list.
    tools: Vec<ToolDefinition>,
    /// Transport handle for sending requests.
    transport: McpTransportHandle,
    /// Next JSON-RPC request ID.
    next_id: u64,
}

/// Transport handle — abstraction over stdio subprocess or HTTP.
enum McpTransportHandle {
    Stdio {
        /// Child process handle. Wrapped in `Option` so `Drop` can `take()`
        /// and move it into a spawned task that performs proper async reaping.
        child: Option<Box<tokio::process::Child>>,
        stdin: tokio::process::ChildStdin,
        stdout: BufReader<tokio::process::ChildStdout>,
    },
    Sse {
        client: reqwest::Client,
        url: String,
    },
}

/// JSON-RPC 2.0 request.
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response.
#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
    /// Present only on server-initiated **requests**/notifications, never on a
    /// response. MCP servers may call back to the client (`sampling/createMessage`,
    /// `roots/list`, `ping`), and those use the SERVER's id space — which also
    /// starts at 1 and therefore collides with ours. Without this field a
    /// server request carrying id 3 would be consumed as the answer to our
    /// request 3, yielding `Ok(None)` and desyncing the stream.
    #[serde(default)]
    method: Option<String>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[allow(dead_code)]
    pub data: Option<serde_json::Value>,
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

/// Maximum non-JSON-RPC lines (blank, log noise, banner output) we'll
/// skip over while waiting for the next JSON-RPC message on stdio.
/// Real-world MCP servers (especially Python ones using the official
/// SDK) sometimes leak warnings or stray prints to stdout before the
/// first response; without this skip the request fails with a useless
/// "Invalid MCP JSON-RPC response: expected value at line 1 column 1".
const MAX_NOISE_LINES: usize = 32;

/// Hard cap on the size of a single JSON-RPC line / response body read from
/// an MCP server. Without this, a malicious or buggy server that never emits
/// a newline (or returns a multi-gigabyte body) would make us buffer the whole
/// thing into memory and OOM. 16 MiB is far larger than any legitimate
/// JSON-RPC message yet bounds the blast radius.
const MAX_MCP_LINE_BYTES: usize = 16 * 1024 * 1024;

/// Read the next plausibly JSON-RPC line from an MCP server's stdout,
/// skipping blank lines and obvious log/banner noise. Returns the raw
/// line (untrimmed) so the caller can re-trim and parse.
///
/// Honors the supplied timeout for the *entire* read attempt — not per
/// noise line — so a chatty server can't keep us waiting forever even
/// if it stays under the per-line read budget.
async fn read_next_jsonrpc_line<R>(
    reader: &mut R,
    timeout: tokio::time::Duration,
) -> Result<String, String>
where
    R: AsyncBufReadExt + Unpin,
{
    let deadline = tokio::time::Instant::now() + timeout;

    for _ in 0..MAX_NOISE_LINES {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Err("MCP request timed out".to_string());
        }
        let remaining = deadline - now;

        // Size-bounded read: cap the bytes we'll buffer for a single line so a
        // server that never emits a newline can't make us OOM. `take` limits
        // the underlying reader to MAX_MCP_LINE_BYTES; `read_until` then reads
        // up to (and including) the newline or until that cap is hit.
        let mut line_buf: Vec<u8> = Vec::new();
        {
            let mut limited = (&mut *reader).take(MAX_MCP_LINE_BYTES as u64);
            match tokio::time::timeout(remaining, limited.read_until(b'\n', &mut line_buf)).await {
                Ok(Ok(0)) => return Err("MCP server closed connection".to_string()),
                Ok(Ok(_)) => {}
                Ok(Err(e)) => return Err(format!("Failed to read MCP response: {e}")),
                Err(_) => return Err("MCP request timed out".to_string()),
            }
        }

        // If we filled the entire budget without a terminating newline, the
        // server sent an oversized (or unterminated) line — reject it.
        if line_buf.len() >= MAX_MCP_LINE_BYTES {
            return Err(format!(
                "MCP server response exceeded {MAX_MCP_LINE_BYTES}-byte line cap"
            ));
        }

        let line = String::from_utf8(line_buf)
            .map_err(|e| format!("MCP response was not valid UTF-8: {e}"))?;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // JSON-RPC responses are always JSON objects. Anything else
        // (Python warnings to stdout, ANSI banners, dotfile messages)
        // gets silently skipped at debug level so noisy servers don't
        // break the protocol.
        if !trimmed.starts_with('{') {
            debug!(noise = %trimmed, "Skipping non-JSON-RPC line from MCP server stdout");
            continue;
        }
        return Ok(line);
    }

    Err(format!(
        "MCP server emitted {MAX_NOISE_LINES} non-JSON-RPC lines before any valid response"
    ))
}

/// Maximum number of unrelated frames tolerated while waiting for our response.
const MAX_SKIPPED_FRAMES: usize = 64;

/// Read frames from `reader` until the JSON-RPC **response** carrying `id`
/// arrives, discarding unrelated ones.
///
/// Why the skipping is needed: the stdio stream is persistent and shared across
/// every call on the connection, so a previous request that timed out can leave
/// its late response sitting in the buffer. Returning that stale frame would
/// answer the current call with the previous call's payload and desync every
/// subsequent request.
///
/// Two bounds keep the skipping from becoming a hang. `deadline` is computed
/// ONCE by the caller and each read gets only the remaining time — passing the
/// full `timeout` per iteration would re-arm it forever, so a server emitting a
/// frame slightly faster than `timeout` could hold the call indefinitely (the
/// pre-drain code was hard-bounded by `timeout`, so that would have been a
/// regression). `MAX_SKIPPED_FRAMES` additionally caps a fast flood.
///
/// Free function generic over the reader so it is unit-testable against
/// in-memory byte slices, without a live MCP child process.
async fn read_response_for_id<R>(
    reader: &mut R,
    id: u64,
    deadline: tokio::time::Instant,
) -> Result<Option<serde_json::Value>, String>
where
    R: AsyncBufReadExt + Unpin,
{
    for _ in 0..MAX_SKIPPED_FRAMES {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("MCP request timed out".to_string());
        }
        let line = read_next_jsonrpc_line(reader, remaining).await?;

        // A JSON object that isn't a JSON-RPC frame is noise, not a fatal error:
        // `read_next_jsonrpc_line` already filters non-`{` lines, but a server can
        // still print a structured log object. Aborting here would fail the call
        // even though the real response is on the very next line.
        let Ok(response) = serde_json::from_str::<JsonRpcResponse>(line.trim()) else {
            debug!(noise = %line.trim(), "Skipping non-JSON-RPC object on MCP stdout");
            continue;
        };

        // Server-initiated request/notification — uses the server's id space.
        if let Some(method) = &response.method {
            debug!(method = %method, "Skipping server-initiated MCP frame");
            continue;
        }

        match response.id {
            Some(rid) if rid == id => {
                if let Some(err) = response.error {
                    return Err(format!("{err}"));
                }
                return Ok(response.result);
            }
            // JSON-RPC 2.0 mandates `id: null` when the server cannot associate
            // an error with a request (e.g. our line failed to parse on its
            // side). That error is terminal for this call, so surface it rather
            // than skipping it and stalling until the deadline with a generic
            // timeout message.
            None if response.error.is_some() => {
                return Err(format!("{}", response.error.expect("checked is_some")));
            }
            other => {
                debug!(
                    expected = id,
                    got = ?other,
                    "Skipping MCP response with non-matching id (stale/notification)"
                );
            }
        }
    }
    Err(format!(
        "MCP server sent {MAX_SKIPPED_FRAMES} frames without answering request id {id}"
    ))
}

// ---------------------------------------------------------------------------
// McpConnection implementation
// ---------------------------------------------------------------------------

impl McpConnection {
    /// Connect to an MCP server, perform handshake, and discover tools.
    pub async fn connect(config: McpServerConfig) -> Result<Self, String> {
        let transport = match &config.transport {
            McpTransport::Stdio { command, args } => {
                Self::connect_stdio(command, args, &config.env).await?
            }
            McpTransport::Sse { url } => {
                // SSRF check: reject private/localhost URLs unless explicitly configured
                Self::connect_sse(url).await?
            }
        };

        let mut conn = Self {
            config,
            tools: Vec::new(),
            transport,
            next_id: 1,
        };

        // Initialize handshake
        conn.initialize().await?;

        // Discover tools
        conn.discover_tools().await?;

        info!(
            server = %conn.config.name,
            tools = conn.tools.len(),
            "MCP server connected"
        );

        Ok(conn)
    }

    /// Send the MCP `initialize` handshake.
    async fn initialize(&mut self) -> Result<(), String> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "rustyhand",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let response = self.send_request("initialize", Some(params)).await?;

        if let Some(result) = response {
            debug!(
                server = %self.config.name,
                server_info = %result,
                "MCP initialize response"
            );
        }

        // Send initialized notification (no response expected)
        self.send_notification("notifications/initialized", None)
            .await?;

        Ok(())
    }

    /// Discover available tools via `tools/list`.
    async fn discover_tools(&mut self) -> Result<(), String> {
        let response = self.send_request("tools/list", None).await?;

        if let Some(result) = response {
            if let Some(tools_array) = result.get("tools").and_then(|t| t.as_array()) {
                let server_name = &self.config.name;
                for tool in tools_array {
                    let raw_name = tool["name"].as_str().unwrap_or("unnamed");
                    let description = tool["description"].as_str().unwrap_or("");
                    let input_schema = tool
                        .get("inputSchema")
                        .cloned()
                        .unwrap_or(serde_json::json!({"type": "object"}));

                    // Namespace: mcp_{server}_{tool}
                    let namespaced = format_mcp_tool_name(server_name, raw_name);

                    self.tools.push(ToolDefinition {
                        name: namespaced,
                        description: format!("[MCP:{server_name}] {description}"),
                        input_schema,
                    });
                }
            }
        }

        Ok(())
    }

    /// Call a tool on the MCP server.
    ///
    /// `name` should be the namespaced name (mcp_{server}_{tool}).
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        // Strip the namespace prefix to get the original tool name
        let raw_name = strip_mcp_prefix(&self.config.name, name).unwrap_or(name);

        let params = serde_json::json!({
            "name": raw_name,
            "arguments": arguments,
        });

        let response = self.send_request("tools/call", Some(params)).await?;

        match response {
            Some(result) => {
                // Per the MCP spec, a failed tool call is reported as a normal
                // result with `isError: true` and the error text carried in
                // `content` — NOT via the JSON-RPC `error` object. Detect that
                // here so failures surface as Err to the agent loop instead of
                // masquerading as a successful result.
                let is_error = result.get("isError").and_then(|v| v.as_bool()) == Some(true);

                // Extract text content from the response
                let text = if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                    let texts: Vec<&str> = content
                        .iter()
                        .filter_map(|item| {
                            if item["type"].as_str() == Some("text") {
                                item["text"].as_str()
                            } else {
                                None
                            }
                        })
                        .collect();
                    texts.join("\n")
                } else {
                    result.to_string()
                };

                if is_error {
                    return Err(text);
                }
                Ok(text)
            }
            None => Err("No result from MCP tools/call".to_string()),
        }
    }

    /// Get the discovered tool definitions.
    pub fn tools(&self) -> &[ToolDefinition] {
        &self.tools
    }

    /// Get the server name.
    pub fn name(&self) -> &str {
        &self.config.name
    }

    // --- Transport helpers ---

    async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<Option<serde_json::Value>, String> {
        let id = self.next_id;
        self.next_id += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to serialize request: {e}"))?;

        debug!(method, id, "MCP request");

        match &mut self.transport {
            McpTransportHandle::Stdio { stdin, stdout, .. } => {
                // Write request + newline
                stdin
                    .write_all(request_json.as_bytes())
                    .await
                    .map_err(|e| format!("Failed to write to MCP stdin: {e}"))?;
                stdin
                    .write_all(b"\n")
                    .await
                    .map_err(|e| format!("Failed to write newline: {e}"))?;
                stdin
                    .flush()
                    .await
                    .map_err(|e| format!("Failed to flush stdin: {e}"))?;

                // ONE deadline for the whole exchange, including any frames we
                // skip along the way — see read_response_for_id.
                let deadline = tokio::time::Instant::now()
                    + tokio::time::Duration::from_secs(self.config.timeout_secs);
                read_response_for_id(stdout, id, deadline).await
            }
            McpTransportHandle::Sse { client, url } => {
                let mut response = client
                    .post(url.as_str())
                    .json(&request)
                    .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
                    .send()
                    .await
                    .map_err(|e| format!("MCP SSE request failed: {e}"))?;

                if !response.status().is_success() {
                    return Err(format!("MCP SSE returned {}", response.status()));
                }

                // Reject up front if the server advertises an oversized body.
                if let Some(len) = response.content_length() {
                    if len > MAX_MCP_LINE_BYTES as u64 {
                        return Err(format!(
                            "MCP SSE response too large: {len} bytes exceeds {MAX_MCP_LINE_BYTES}-byte cap"
                        ));
                    }
                }

                // Stream the body with a running-total cap so a server that
                // lies about (or omits) Content-Length can't OOM us either.
                let mut buf: Vec<u8> = Vec::new();
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|e| format!("Failed to read SSE response: {e}"))?
                {
                    if buf.len() + chunk.len() > MAX_MCP_LINE_BYTES {
                        return Err(format!(
                            "MCP SSE response exceeded {MAX_MCP_LINE_BYTES}-byte cap"
                        ));
                    }
                    buf.extend_from_slice(&chunk);
                }

                let body = String::from_utf8(buf)
                    .map_err(|e| format!("MCP SSE response was not valid UTF-8: {e}"))?;

                let rpc_response: JsonRpcResponse = serde_json::from_str(&body)
                    .map_err(|e| format!("Invalid MCP SSE JSON-RPC response: {e}"))?;

                if let Some(err) = rpc_response.error {
                    return Err(format!("{err}"));
                }

                Ok(rpc_response.result)
            }
        }
    }

    async fn send_notification(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(serde_json::json!({})),
        });

        let json = serde_json::to_string(&notification)
            .map_err(|e| format!("Failed to serialize notification: {e}"))?;

        match &mut self.transport {
            McpTransportHandle::Stdio { stdin, .. } => {
                stdin
                    .write_all(json.as_bytes())
                    .await
                    .map_err(|e| format!("Write notification: {e}"))?;
                stdin
                    .write_all(b"\n")
                    .await
                    .map_err(|e| format!("Write newline: {e}"))?;
                stdin.flush().await.map_err(|e| format!("Flush: {e}"))?;
            }
            McpTransportHandle::Sse { client, url } => {
                let _ = client.post(url.as_str()).json(&notification).send().await;
            }
        }

        Ok(())
    }

    async fn connect_stdio(
        command: &str,
        args: &[String],
        env_whitelist: &[String],
    ) -> Result<McpTransportHandle, String> {
        // Validate command path (no path traversal)
        if command.contains("..") {
            return Err("MCP command path contains '..': rejected".to_string());
        }

        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Sandbox: clear environment, only pass whitelisted vars
        cmd.env_clear();
        for var_name in env_whitelist {
            if let Ok(val) = std::env::var(var_name) {
                cmd.env(var_name, val);
            }
        }
        // Always pass PATH for binary resolution
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn MCP server '{command}': {e}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or("Failed to capture MCP server stdin")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to capture MCP server stdout")?;

        Ok(McpTransportHandle::Stdio {
            child: Some(Box::new(child)),
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    async fn connect_sse(url: &str) -> Result<McpTransportHandle, String> {
        // Basic SSRF check: reject obviously private URLs
        let lower = url.to_lowercase();
        if lower.contains("169.254.169.254") || lower.contains("metadata.google") {
            return Err("SSRF: MCP SSE URL targets metadata endpoint".to_string());
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

        Ok(McpTransportHandle::Sse {
            client,
            url: url.to_string(),
        })
    }
}

impl Drop for McpConnection {
    fn drop(&mut self) {
        if let McpTransportHandle::Stdio { ref mut child, .. } = self.transport {
            // Signal the child immediately (sync) and, if a tokio runtime
            // is available, spawn a task to reap the zombie via wait().
            // Without the reap, the subprocess stays as a zombie until the
            // whole rustyhand process exits.
            if let Some(mut taken) = child.take() {
                let _ = taken.start_kill();
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        let _ = taken.wait().await;
                    });
                }
                // If no runtime is available (e.g. dropped from sync code
                // outside tokio), start_kill will still terminate the child
                // and it will be reaped by the OS on rustyhand exit.
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tool namespacing helpers
// ---------------------------------------------------------------------------

/// Format a namespaced MCP tool name: `mcp_{server}_{tool}`.
pub fn format_mcp_tool_name(server: &str, tool: &str) -> String {
    format!("mcp_{}_{}", normalize_name(server), normalize_name(tool))
}

/// Check if a tool name is an MCP-namespaced tool.
pub fn is_mcp_tool(name: &str) -> bool {
    name.starts_with("mcp_")
}

/// Extract server name from an MCP tool name.
pub fn extract_mcp_server(tool_name: &str) -> Option<&str> {
    if !tool_name.starts_with("mcp_") {
        return None;
    }
    let rest = &tool_name[4..];
    rest.find('_').map(|pos| &rest[..pos])
}

/// Strip the MCP namespace prefix from a tool name.
fn strip_mcp_prefix<'a>(server: &str, tool_name: &'a str) -> Option<&'a str> {
    let prefix = format!("mcp_{}_", normalize_name(server));
    tool_name.strip_prefix(&prefix)
}

/// Normalize a name for use in tool namespacing (lowercase, replace hyphens).
pub fn normalize_name(name: &str) -> String {
    name.to_lowercase().replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── read_response_for_id ────────────────────────────────────────────────

    fn deadline(secs: u64) -> tokio::time::Instant {
        tokio::time::Instant::now() + std::time::Duration::from_secs(secs)
    }

    /// The reason id-matching exists: a request that timed out earlier can leave
    /// its late response buffered on the persistent stdio stream. Returning that
    /// frame would answer this call with the previous call's payload and desync
    /// every request after it.
    #[tokio::test]
    async fn skips_stale_response_and_returns_the_matching_one() {
        let stream = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"stale\"}\n\
            {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":\"mine\"}\n";
        let mut reader = BufReader::new(&stream[..]);
        let got = read_response_for_id(&mut reader, 2, deadline(2))
            .await
            .expect("must find id 2");
        assert_eq!(got, Some(serde_json::json!("mine")));
    }

    /// Notifications carry no id and must not be mistaken for our answer.
    #[tokio::test]
    async fn skips_notifications() {
        let stream = b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\",\"params\":{}}\n\
            {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"mine\"}\n";
        let mut reader = BufReader::new(&stream[..]);
        let got = read_response_for_id(&mut reader, 1, deadline(2))
            .await
            .expect("must skip the notification");
        assert_eq!(got, Some(serde_json::json!("mine")));
    }

    /// Server-initiated REQUESTS use the server's own id space, which also
    /// starts at 1. Matching on id alone would consume this frame as the answer
    /// to our request 1 and return Ok(None).
    #[tokio::test]
    async fn does_not_match_a_server_request_with_a_colliding_id() {
        let stream = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n\
            {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"mine\"}\n";
        let mut reader = BufReader::new(&stream[..]);
        let got = read_response_for_id(&mut reader, 1, deadline(2))
            .await
            .expect("must skip the server-initiated request");
        assert_eq!(got, Some(serde_json::json!("mine")));
    }

    /// A structured log object on stdout is noise, not a fatal error — the real
    /// response is on the next line.
    #[tokio::test]
    async fn skips_json_objects_that_are_not_jsonrpc_frames() {
        let stream = b"{\"level\":\"info\",\"msg\":\"starting\"}\n\
            {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"mine\"}\n";
        let mut reader = BufReader::new(&stream[..]);
        let got = read_response_for_id(&mut reader, 1, deadline(2))
            .await
            .expect("must skip the non-JSON-RPC object");
        assert_eq!(got, Some(serde_json::json!("mine")));
    }

    #[tokio::test]
    async fn surfaces_an_error_for_our_id() {
        let stream =
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32601,\"message\":\"nope\"}}\n";
        let mut reader = BufReader::new(&stream[..]);
        let err = read_response_for_id(&mut reader, 1, deadline(2))
            .await
            .expect_err("an error frame must surface");
        assert!(err.contains("nope"), "got: {err}");
    }

    /// JSON-RPC 2.0 mandates `id: null` when the server cannot associate an
    /// error with a request. Skipping it would stall the caller until the
    /// deadline and report a generic timeout instead of the server's message.
    #[tokio::test]
    async fn surfaces_a_null_id_error_instead_of_stalling() {
        let stream =
            b"{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32700,\"message\":\"parse error\"}}\n";
        let mut reader = BufReader::new(&stream[..]);
        let err = read_response_for_id(&mut reader, 1, deadline(2))
            .await
            .expect_err("a null-id error frame must surface");
        assert!(err.contains("parse error"), "got: {err}");
    }

    /// A server that streams unrelated frames must not hold the call open
    /// forever. Before the frame cap and the single shared deadline, each read
    /// re-armed its own timeout, so a steady trickle was unbounded.
    #[tokio::test]
    async fn gives_up_after_too_many_unrelated_frames() {
        let mut stream = Vec::new();
        for i in 0..(MAX_SKIPPED_FRAMES + 10) {
            stream.extend_from_slice(
                format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":null}}\n",
                    9000 + i
                )
                .as_bytes(),
            );
        }
        let mut reader = BufReader::new(&stream[..]);
        let err = read_response_for_id(&mut reader, 1, deadline(5))
            .await
            .expect_err("must give up rather than spin");
        assert!(err.contains("without answering"), "got: {err}");
    }

    /// An already-elapsed deadline must fail immediately, not read.
    #[tokio::test]
    async fn respects_an_expired_deadline() {
        let stream = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"mine\"}\n";
        let mut reader = BufReader::new(&stream[..]);
        let err = read_response_for_id(&mut reader, 1, tokio::time::Instant::now())
            .await
            .expect_err("expired deadline must error");
        assert!(err.contains("timed out"), "got: {err}");
    }

    #[test]
    fn test_mcp_tool_namespacing() {
        assert_eq!(
            format_mcp_tool_name("github", "create_issue"),
            "mcp_github_create_issue"
        );
        assert_eq!(
            format_mcp_tool_name("my-server", "do_thing"),
            "mcp_my_server_do_thing"
        );
    }

    #[test]
    fn test_is_mcp_tool() {
        assert!(is_mcp_tool("mcp_github_create_issue"));
        assert!(!is_mcp_tool("file_read"));
        assert!(!is_mcp_tool(""));
    }

    #[test]
    fn test_extract_mcp_server() {
        assert_eq!(
            extract_mcp_server("mcp_github_create_issue"),
            Some("github")
        );
        assert_eq!(extract_mcp_server("file_read"), None);
    }

    #[test]
    fn test_mcp_jsonrpc_initialize() {
        // Verify the initialize request structure
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "rustyhand",
                    "version": "0.1.0"
                }
            })),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("initialize"));
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("rustyhand"));
    }

    #[test]
    fn test_mcp_jsonrpc_tools_list() {
        // Simulate a tools/list response
        let response_json = r#"{
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": [
                    {
                        "name": "create_issue",
                        "description": "Create a GitHub issue",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "title": {"type": "string"},
                                "body": {"type": "string"}
                            },
                            "required": ["title"]
                        }
                    }
                ]
            }
        }"#;

        let response: JsonRpcResponse = serde_json::from_str(response_json).unwrap();
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"].as_str().unwrap(), "create_issue");
    }

    #[test]
    fn test_mcp_transport_config_serde() {
        let config = McpServerConfig {
            name: "github".to_string(),
            transport: McpTransport::Stdio {
                command: "npx".to_string(),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-github".to_string(),
                ],
            },
            timeout_secs: 30,
            env: vec!["GITHUB_PERSONAL_ACCESS_TOKEN".to_string()],
        };

        let json = serde_json::to_string(&config).unwrap();
        let back: McpServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "github");
        assert_eq!(back.timeout_secs, 30);
        assert_eq!(back.env, vec!["GITHUB_PERSONAL_ACCESS_TOKEN"]);

        match back.transport {
            McpTransport::Stdio { command, args } => {
                assert_eq!(command, "npx");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Stdio transport"),
        }

        // SSE variant
        let sse_config = McpServerConfig {
            name: "test".to_string(),
            transport: McpTransport::Sse {
                url: "https://example.com/mcp".to_string(),
            },
            timeout_secs: 60,
            env: vec![],
        };
        let json = serde_json::to_string(&sse_config).unwrap();
        let back: McpServerConfig = serde_json::from_str(&json).unwrap();
        match back.transport {
            McpTransport::Sse { url } => assert_eq!(url, "https://example.com/mcp"),
            _ => panic!("Expected SSE transport"),
        }
    }

    // ── read_next_jsonrpc_line: noise tolerance ──────────────────────────
    //
    // These exercise the helper directly using `tokio::io::BufReader<&[u8]>`
    // so we can simulate Python MCP servers that leak warnings or blank
    // lines to stdout before the first JSON-RPC response.

    use tokio::io::BufReader;

    #[tokio::test]
    async fn read_next_jsonrpc_line_skips_blank_lines() {
        let stream = b"\n\n   \n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n";
        let mut reader = BufReader::new(&stream[..]);
        let line = read_next_jsonrpc_line(&mut reader, std::time::Duration::from_secs(2))
            .await
            .expect("must skip blank lines and return JSON line");
        assert!(line.contains("\"jsonrpc\""));
        assert!(line.contains("\"result\""));
    }

    #[tokio::test]
    async fn read_next_jsonrpc_line_skips_log_noise() {
        // Realistic Python MCP server: deprecation warning + banner + valid response.
        let stream = b"DeprecationWarning: foo\n\
            [INFO] starting up\n\
            Warning: PYTHONUNBUFFERED is unset\n\
            {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}\n";
        let mut reader = BufReader::new(&stream[..]);
        let line = read_next_jsonrpc_line(&mut reader, std::time::Duration::from_secs(2))
            .await
            .expect("must skip non-JSON-RPC noise lines");
        let parsed: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(parsed["id"], 1);
    }

    #[tokio::test]
    async fn read_next_jsonrpc_line_returns_first_json_line() {
        // Two JSON-RPC lines back-to-back: should return the first.
        let stream = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"first\"}\n\
            {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":\"second\"}\n";
        let mut reader = BufReader::new(&stream[..]);
        let line = read_next_jsonrpc_line(&mut reader, std::time::Duration::from_secs(2))
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(parsed["result"], "first");
    }

    #[tokio::test]
    async fn read_next_jsonrpc_line_errors_when_only_noise() {
        // Pure log noise, no JSON-RPC, then EOF — must surface a clear error.
        let stream = b"WARNING\nINFO foo\nERROR bar\n";
        let mut reader = BufReader::new(&stream[..]);
        let result = read_next_jsonrpc_line(&mut reader, std::time::Duration::from_secs(2)).await;
        // EOF after only noise lines → "MCP server closed connection" (since
        // read_line returns Ok(0) at EOF after we drained the buffered noise).
        let err = result.expect_err("must error when only noise + EOF");
        assert!(
            err.contains("closed connection") || err.contains("noise"),
            "expected closed/noise error, got: {err}"
        );
    }

    #[tokio::test]
    async fn read_next_jsonrpc_line_propagates_eof_immediately() {
        let stream: &[u8] = b"";
        let mut reader = BufReader::new(stream);
        let err = read_next_jsonrpc_line(&mut reader, std::time::Duration::from_secs(2))
            .await
            .expect_err("empty stream must error");
        assert!(
            err.contains("closed connection"),
            "empty stream must produce closed-connection error, got: {err}"
        );
    }
}
