//! WASM sandbox for secure skill/plugin execution.
//!
//! Uses Wasmtime to execute untrusted WASM modules with deny-by-default
//! capability-based permissions. No filesystem, network, or credential
//! access unless explicitly granted.
//!
//! # Guest ABI
//!
//! WASM modules must export:
//! - `memory` — linear memory
//! - `alloc(size: i32) -> i32` — allocate `size` bytes, return pointer
//! - `execute(input_ptr: i32, input_len: i32) -> i64` — main entry point
//!
//! The `execute` function receives JSON input bytes and returns a packed
//! `i64` value: `(result_ptr << 32) | result_len`. The result is JSON bytes.
//!
//! # Host ABI
//!
//! The host provides (in the `"rustyhand"` import module):
//! - `host_call(request_ptr: i32, request_len: i32) -> i64` — RPC dispatch
//! - `host_log(level: i32, msg_ptr: i32, msg_len: i32)` — logging
//!
//! `host_call` reads a JSON request `{"method": "...", "params": {...}}`
//! and returns a packed pointer to JSON `{"ok": ...}` or `{"error": "..."}`.

use crate::host_functions;
use crate::kernel_handle::KernelHandle;
use rusty_hand_types::capability::Capability;
use rusty_hand_types::config::ExecPolicy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::debug;
use wasmtime::*;

/// Resolve a guest `(ptr, len)` pair into a checked byte range within
/// `data_len`. Rejects negative `i32` values (which `as usize` would
/// sign-extend into a huge offset) and guards the addition against overflow
/// before any slice, so a malicious guest can't bypass the bounds check.
fn guest_range(ptr: i32, len: i32, data_len: usize) -> Result<std::ops::Range<usize>> {
    let start = usize::try_from(ptr).map_err(|_| format_err!("negative guest pointer"))?;
    let n = usize::try_from(len).map_err(|_| format_err!("negative guest length"))?;
    let end = start
        .checked_add(n)
        .ok_or_else(|| format_err!("guest range overflow"))?;
    if end > data_len {
        bail!("guest range out of bounds");
    }
    Ok(start..end)
}

/// Same as [`guest_range`] but for a `usize` length the host already knows to
/// be non-negative (e.g. `input_bytes.len()`), so only the pointer needs the
/// negative-value rejection.
fn guest_range_host_len(ptr: i32, len: usize, data_len: usize) -> Result<std::ops::Range<usize>> {
    let start = usize::try_from(ptr).map_err(|_| format_err!("negative guest pointer"))?;
    let end = start
        .checked_add(len)
        .ok_or_else(|| format_err!("guest range overflow"))?;
    if end > data_len {
        bail!("guest range out of bounds");
    }
    Ok(start..end)
}

/// Configuration for a WASM sandbox instance.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum fuel (CPU instruction budget). 0 = unlimited.
    pub fuel_limit: u64,
    /// Maximum WASM linear memory in bytes. Enforced via `StoreLimits` — a
    /// `memory.grow` past this traps. Also budgets the guest's table elements
    /// (`max_memory_bytes / 8`), since tables are host allocations that the
    /// linear-memory cap does not cover.
    pub max_memory_bytes: usize,
    /// Capabilities granted to this sandbox instance.
    pub capabilities: Vec<Capability>,
    /// Wall-clock timeout in seconds for epoch-based interruption.
    /// Defaults to 30 seconds if None.
    pub timeout_secs: Option<u64>,
    /// Exec policy enforced on the WASM `shell_exec` host call. When set, the
    /// operator's ExecSecurityMode (deny/allowlist) gates shell execution from
    /// guest code the same way it gates the builtin shell tool. `None` = no
    /// extra gate beyond the per-command capability check.
    pub exec_policy: Option<ExecPolicy>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            fuel_limit: 1_000_000,
            max_memory_bytes: 16 * 1024 * 1024,
            capabilities: Vec::new(),
            timeout_secs: None,
            exec_policy: None,
        }
    }
}

/// State carried in each WASM Store, accessible by host functions.
pub struct GuestState {
    /// Capabilities granted to this guest — checked before every host call.
    pub capabilities: Vec<Capability>,
    /// Handle to kernel for inter-agent operations.
    pub kernel: Option<Arc<dyn KernelHandle>>,
    /// Agent ID of the calling agent.
    pub agent_id: String,
    /// Tokio runtime handle for async operations in sync host functions.
    pub tokio_handle: tokio::runtime::Handle,
    /// Exec policy gating the `shell_exec` host call (see [`SandboxConfig::exec_policy`]).
    pub exec_policy: Option<ExecPolicy>,
    /// Wasmtime resource limits — caps guest linear memory growth.
    pub limits: StoreLimits,
}

/// Result of executing a WASM module.
#[derive(Debug)]
pub struct ExecutionResult {
    /// JSON output from the guest's `execute` function.
    pub output: serde_json::Value,
    /// Number of fuel units consumed.
    pub fuel_consumed: u64,
}

/// Errors from sandbox operations.
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("WASM compilation failed: {0}")]
    Compilation(String),
    #[error("WASM instantiation failed: {0}")]
    Instantiation(String),
    #[error("WASM execution failed: {0}")]
    Execution(String),
    #[error("Fuel exhausted: skill exceeded CPU budget")]
    FuelExhausted,
    #[error("Guest ABI violation: {0}")]
    AbiError(String),
}

/// The WASM sandbox engine.
///
/// Create one per kernel, reuse across skill invocations. The `Engine`
/// is expensive to create but can compile/instantiate many modules.
pub struct WasmSandbox {
    engine: Engine,
    /// Signals the engine-wide epoch ticker thread to stop (set on drop).
    epoch_stop: Arc<AtomicBool>,
}

impl Drop for WasmSandbox {
    fn drop(&mut self) {
        self.epoch_stop.store(true, Ordering::Relaxed);
    }
}

impl WasmSandbox {
    /// Create a new sandbox engine with fuel metering enabled.
    pub fn new() -> Result<Self, SandboxError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        // Turn off proposals this ABI never uses. Multi-memory is ON by default
        // in wasmtime 47, and `StoreLimits::memory_size` is documented as
        // per-memory — so a multi-memory guest could multiply its linear-memory
        // budget by the store's memory count. The ABI mandates a single exported
        // `memory`, so refuse the proposal outright rather than rely on the count
        // limit alone. Threads are likewise unused and would add shared-memory
        // growth paths outside the limiter.
        config.wasm_multi_memory(false);
        config.wasm_threads(false);
        let engine = Engine::new(&config).map_err(|e| SandboxError::Compilation(e.to_string()))?;

        // A SINGLE monotonic epoch ticker for this engine (one tick per second).
        // Each execution sets its deadline N ticks ahead (see execute_sync), so
        // each gets an independent ~N-second wall-clock timeout. The previous
        // per-execution watchdog incremented the SHARED engine epoch, which
        // cross-interrupted other concurrent executions (a finished/short run's
        // increment tripped a longer run's deadline early).
        let epoch_stop = Arc::new(AtomicBool::new(false));
        {
            let engine = engine.clone();
            let stop = Arc::clone(&epoch_stop);
            std::thread::Builder::new()
                .name("wasm-epoch-ticker".into())
                .spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        engine.increment_epoch();
                    }
                })
                .map_err(|e| SandboxError::Compilation(e.to_string()))?;
        }

        Ok(Self { engine, epoch_stop })
    }

    /// Execute a WASM module with the given JSON input.
    ///
    /// All host calls from within the module are subject to capability checks.
    /// Execution is offloaded to a blocking thread (CPU-bound WASM should not
    /// run on the Tokio executor).
    pub async fn execute(
        &self,
        wasm_bytes: &[u8],
        input: serde_json::Value,
        config: SandboxConfig,
        kernel: Option<Arc<dyn KernelHandle>>,
        agent_id: &str,
    ) -> Result<ExecutionResult, SandboxError> {
        let engine = self.engine.clone();
        let wasm_bytes = wasm_bytes.to_vec();
        let agent_id = agent_id.to_string();
        let handle = tokio::runtime::Handle::current();

        tokio::task::spawn_blocking(move || {
            Self::execute_sync(
                &engine,
                &wasm_bytes,
                input,
                &config,
                kernel,
                &agent_id,
                handle,
            )
        })
        .await
        .map_err(|e| SandboxError::Execution(format!("spawn_blocking join failed: {e}")))?
    }

    /// Synchronous inner execution — runs on a blocking thread.
    fn execute_sync(
        engine: &Engine,
        wasm_bytes: &[u8],
        input: serde_json::Value,
        config: &SandboxConfig,
        kernel: Option<Arc<dyn KernelHandle>>,
        agent_id: &str,
        tokio_handle: tokio::runtime::Handle,
    ) -> Result<ExecutionResult, SandboxError> {
        // Compile the module (accepts both .wasm binary and .wat text)
        let module = Module::new(engine, wasm_bytes)
            .map_err(|e| SandboxError::Compilation(e.to_string()))?;

        // Create store with guest state
        let mut store = Store::new(
            engine,
            GuestState {
                capabilities: config.capabilities.clone(),
                kernel,
                agent_id: agent_id.to_string(),
                tokio_handle,
                exec_policy: config.exec_policy.clone(),
                limits: StoreLimitsBuilder::new()
                    .memory_size(config.max_memory_bytes)
                    // `memory_size` alone is NOT a memory budget: wasmtime's
                    // defaults leave `table_elements` UNLIMITED, so a module
                    // declaring `(table 200000000 funcref)` allocates ~1 GiB of
                    // host RAM while nominally under a 1 MiB memory cap. Budget
                    // tables against the same allowance (8 bytes per element is
                    // the funcref slot size on 64-bit hosts).
                    .table_elements(config.max_memory_bytes / 8)
                    // Pin the per-store counts too — the defaults are 10_000
                    // each, which would multiply every limit above by 10_000.
                    // This sandbox instantiates exactly one module with one
                    // linear memory.
                    .instances(1)
                    .memories(1)
                    .tables(4)
                    .build(),
            },
        );

        // SECURITY: cap guest linear memory at the manifest's max_memory_bytes.
        // Fuel/epoch bound CPU and wall-clock but NOT memory, so without this a
        // wasm32 guest could grow to the 4 GiB architectural max and OOM the host.
        store.limiter(|state| &mut state.limits);

        // Set fuel budget (deterministic metering)
        if config.fuel_limit > 0 {
            store
                .set_fuel(config.fuel_limit)
                .map_err(|e| SandboxError::Execution(e.to_string()))?;
        }

        // Set epoch deadline, counted in ticks of the engine's monotonic 1s epoch
        // ticker (WasmSandbox::new). The deadline is relative to the CURRENT epoch,
        // so it is independent per execution — a concurrent execution's deadline
        // can't trip this store early.
        //
        // `+ 1` because the ticker runs on its own cadence, unsynchronized with
        // store creation: a store created 1ms before a tick would otherwise get
        // `timeout - 1` seconds of real budget. With the extra tick the guest is
        // guaranteed at least `timeout` seconds and interrupted within
        // `timeout + 1` — i.e. the timeout is a floor, as callers expect.
        let timeout = config.timeout_secs.unwrap_or(30).max(1);
        store.set_epoch_deadline(timeout + 1);

        // Build linker with host function imports
        let mut linker = Linker::new(engine);
        Self::register_host_functions(&mut linker)?;

        // Instantiate — links host functions, no WASI
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| SandboxError::Instantiation(e.to_string()))?;

        // Retrieve required guest exports
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| SandboxError::AbiError("Module must export 'memory'".into()))?;

        let alloc_fn = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|e| {
                SandboxError::AbiError(format!("Module must export 'alloc(i32)->i32': {e}"))
            })?;

        let execute_fn = instance
            .get_typed_func::<(i32, i32), i64>(&mut store, "execute")
            .map_err(|e| {
                SandboxError::AbiError(format!("Module must export 'execute(i32,i32)->i64': {e}"))
            })?;

        // Serialize input JSON → bytes
        let input_bytes = serde_json::to_vec(&input)
            .map_err(|e| SandboxError::Execution(format!("JSON serialize failed: {e}")))?;

        // Allocate space in guest memory for input
        let input_ptr = alloc_fn
            .call(&mut store, input_bytes.len() as i32)
            .map_err(|e| SandboxError::AbiError(format!("alloc call failed: {e}")))?;

        // Write input into guest memory.
        //
        // SECURITY: `alloc` is guest code, so `input_ptr` is guest-controlled and
        // may be negative. A raw `input_ptr as usize` sign-extends to ~u64::MAX and
        // the following `+ len` then WRAPS in a release build (overflow-checks are
        // off), sliding past the `end > mem_data.len()` guard and panicking the
        // blocking thread on `mem_data[start..end]` (start > end). Route it through
        // the same checked helper the host_call/host_log ABI sites use.
        let mem_data = memory.data_mut(&mut store);
        let range = guest_range_host_len(input_ptr, input_bytes.len(), mem_data.len())
            .map_err(|e| SandboxError::AbiError(format!("Input exceeds memory bounds: {e}")))?;
        mem_data[range].copy_from_slice(&input_bytes);

        // Call guest execute
        let packed = match execute_fn.call(&mut store, (input_ptr, input_bytes.len() as i32)) {
            Ok(v) => v,
            Err(e) => {
                // Check for fuel exhaustion via trap code
                if let Some(Trap::OutOfFuel) = e.downcast_ref::<Trap>() {
                    return Err(SandboxError::FuelExhausted);
                }
                // Check for epoch deadline (wall-clock timeout)
                if let Some(Trap::Interrupt) = e.downcast_ref::<Trap>() {
                    return Err(SandboxError::Execution(format!(
                        "WASM execution timed out after {}s (epoch interrupt)",
                        timeout
                    )));
                }
                return Err(SandboxError::Execution(e.to_string()));
            }
        };

        // Unpack result: high 32 bits = ptr, low 32 bits = len.
        //
        // SECURITY: `packed` is whatever the guest's `execute` returned. Shift it
        // as UNSIGNED — `packed >> 32` on an i64 is an *arithmetic* shift, so a
        // guest returning e.g. 0xFFFF_FFFF_0000_0001 yields -1, and `as usize`
        // sign-extends that to u64::MAX. The subsequent `result_ptr + result_len`
        // then wraps in release and slips past the bounds check, panicking the
        // blocking thread on the slice. Masking to u32 keeps both halves in the
        // 32-bit range the ABI actually defines.
        let packed = packed as u64;
        let result_ptr = ((packed >> 32) & 0xFFFF_FFFF) as usize;
        let result_len = (packed & 0xFFFF_FFFF) as usize;

        // Read output JSON from guest memory
        let mem_data = memory.data(&store);
        let end = result_ptr.checked_add(result_len).ok_or_else(|| {
            SandboxError::AbiError("Result pointer out of bounds (overflow)".into())
        })?;
        if end > mem_data.len() {
            return Err(SandboxError::AbiError(
                "Result pointer out of bounds".into(),
            ));
        }
        let output_bytes = &mem_data[result_ptr..end];

        let output: serde_json::Value = serde_json::from_slice(output_bytes)
            .map_err(|e| SandboxError::AbiError(format!("Invalid JSON output from guest: {e}")))?;

        // Calculate fuel consumed
        let fuel_remaining = store.get_fuel().unwrap_or(0);
        let fuel_consumed = config.fuel_limit.saturating_sub(fuel_remaining);

        debug!(agent = agent_id, fuel_consumed, "WASM execution complete");

        Ok(ExecutionResult {
            output,
            fuel_consumed,
        })
    }

    /// Register host function imports in the linker ("rustyhand" module).
    fn register_host_functions(linker: &mut Linker<GuestState>) -> Result<(), SandboxError> {
        // host_call: single dispatch for all capability-checked operations.
        // Request: JSON bytes in guest memory → {"method": "...", "params": {...}}
        // Response: packed (ptr, len) pointing to JSON in guest memory.
        linker
            .func_wrap(
                "rustyhand",
                "host_call",
                |mut caller: Caller<'_, GuestState>,
                 request_ptr: i32,
                 request_len: i32|
                 -> Result<i64> {
                    // Read request from guest memory
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| format_err!("no memory export"))?;

                    let data = memory.data(&caller);
                    let range = guest_range(request_ptr, request_len, data.len())?;
                    let request_bytes = data[range].to_vec();

                    // Parse request
                    let request: serde_json::Value =
                        serde_json::from_slice(&request_bytes).map_err(|e| format_err!("{e}"))?;
                    let method = request
                        .get("method")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_string();
                    let params = request
                        .get("params")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    // Dispatch to capability-checked handler
                    let response = host_functions::dispatch(caller.data(), &method, &params);

                    // Serialize response JSON
                    let response_bytes =
                        serde_json::to_vec(&response).map_err(|e| format_err!("{e}"))?;
                    let len = response_bytes.len() as i32;

                    // Allocate space in guest for response
                    let alloc_fn = caller
                        .get_export("alloc")
                        .and_then(|e| e.into_func())
                        .ok_or_else(|| format_err!("no alloc export"))?;
                    let alloc_typed = alloc_fn.typed::<i32, i32>(&caller)?;
                    let ptr = alloc_typed.call(&mut caller, len)?;

                    // Write response into guest memory
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| format_err!("no memory export"))?;
                    let mem_data = memory.data_mut(&mut caller);
                    let dest_start =
                        usize::try_from(ptr).map_err(|_| format_err!("negative alloc pointer"))?;
                    let dest_end = dest_start
                        .checked_add(response_bytes.len())
                        .ok_or_else(|| format_err!("response range overflow"))?;
                    if dest_end > mem_data.len() {
                        bail!("host_call: response exceeds memory bounds");
                    }
                    mem_data[dest_start..dest_end].copy_from_slice(&response_bytes);

                    // Pack (ptr, len) into i64
                    Ok(((ptr as i64) << 32) | (len as i64))
                },
            )
            .map_err(|e| SandboxError::Compilation(e.to_string()))?;

        // host_log: lightweight logging — no capability check required.
        linker
            .func_wrap(
                "rustyhand",
                "host_log",
                |mut caller: Caller<'_, GuestState>,
                 level: i32,
                 msg_ptr: i32,
                 msg_len: i32|
                 -> Result<()> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| format_err!("no memory export"))?;

                    let data = memory.data(&caller);
                    let range = guest_range(msg_ptr, msg_len, data.len())?;
                    let msg = std::str::from_utf8(&data[range]).unwrap_or("<invalid utf8>");
                    let agent_id = &caller.data().agent_id;

                    match level {
                        0 => tracing::trace!(agent = %agent_id, "[wasm] {msg}"),
                        1 => tracing::debug!(agent = %agent_id, "[wasm] {msg}"),
                        2 => tracing::info!(agent = %agent_id, "[wasm] {msg}"),
                        3 => tracing::warn!(agent = %agent_id, "[wasm] {msg}"),
                        _ => tracing::error!(agent = %agent_id, "[wasm] {msg}"),
                    }
                    Ok(())
                },
            )
            .map_err(|e| SandboxError::Compilation(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal echo module: returns input JSON unchanged.
    const ECHO_WAT: &str = r#"
        (module
            (memory (export "memory") 1)
            (global $bump (mut i32) (i32.const 1024))

            (func (export "alloc") (param $size i32) (result i32)
                (local $ptr i32)
                (local.set $ptr (global.get $bump))
                (global.set $bump (i32.add (global.get $bump) (local.get $size)))
                (local.get $ptr)
            )

            (func (export "execute") (param $ptr i32) (param $len i32) (result i64)
                ;; Echo: return the input as-is
                (i64.or
                    (i64.shl
                        (i64.extend_i32_u (local.get $ptr))
                        (i64.const 32)
                    )
                    (i64.extend_i32_u (local.get $len))
                )
            )
        )
    "#;

    /// Module with infinite loop to test fuel exhaustion.
    const INFINITE_LOOP_WAT: &str = r#"
        (module
            (memory (export "memory") 1)
            (global $bump (mut i32) (i32.const 1024))

            (func (export "alloc") (param $size i32) (result i32)
                (local $ptr i32)
                (local.set $ptr (global.get $bump))
                (global.set $bump (i32.add (global.get $bump) (local.get $size)))
                (local.get $ptr)
            )

            (func (export "execute") (param $ptr i32) (param $len i32) (result i64)
                (loop $inf
                    (br $inf)
                )
                (i64.const 0)
            )
        )
    "#;

    /// Proxy module: forwards input to host_call and returns the response.
    const HOST_CALL_PROXY_WAT: &str = r#"
        (module
            (import "rustyhand" "host_call" (func $host_call (param i32 i32) (result i64)))
            (memory (export "memory") 2)
            (global $bump (mut i32) (i32.const 1024))

            (func (export "alloc") (param $size i32) (result i32)
                (local $ptr i32)
                (local.set $ptr (global.get $bump))
                (global.set $bump (i32.add (global.get $bump) (local.get $size)))
                (local.get $ptr)
            )

            (func (export "execute") (param $input_ptr i32) (param $input_len i32) (result i64)
                (call $host_call (local.get $input_ptr) (local.get $input_len))
            )
        )
    "#;

    /// `alloc` returns -1. The host then computes `input_ptr as usize`, which
    /// sign-extends to ~u64::MAX before the bounds check.
    const EVIL_ALLOC_WAT: &str = r#"
        (module
            (memory (export "memory") 1)
            (func (export "alloc") (param $size i32) (result i32)
                (i32.const -1)
            )
            (func (export "execute") (param $ptr i32) (param $len i32) (result i64)
                (i64.const 0)
            )
        )
    "#;

    /// `execute` returns a packed value whose high 32 bits are 0xFFFFFFFF, so an
    /// arithmetic `>> 32` yields -1 and `as usize` sign-extends it.
    const EVIL_RESULT_WAT: &str = r#"
        (module
            (memory (export "memory") 1)
            (global $bump (mut i32) (i32.const 1024))
            (func (export "alloc") (param $size i32) (result i32)
                (local $ptr i32)
                (local.set $ptr (global.get $bump))
                (global.set $bump (i32.add (global.get $bump) (local.get $size)))
                (local.get $ptr)
            )
            (func (export "execute") (param $ptr i32) (param $len i32) (result i64)
                (i64.const 0xFFFFFFFF00000001)
            )
        )
    "#;

    /// Declares a table far larger than any memory budget. Tables are host
    /// allocations that the linear-memory cap does not cover.
    const HUGE_TABLE_WAT: &str = r#"
        (module
            (memory (export "memory") 1)
            (table 200000000 funcref)
            (func (export "alloc") (param $size i32) (result i32) (i32.const 1024))
            (func (export "execute") (param $ptr i32) (param $len i32) (result i64)
                (i64.const 0)
            )
        )
    "#;

    // ── guest_range ─────────────────────────────────────────────────────────

    #[test]
    fn guest_range_accepts_in_bounds() {
        assert_eq!(guest_range(0, 4, 16).unwrap(), 0..4);
        assert_eq!(guest_range(12, 4, 16).unwrap(), 12..16);
        assert_eq!(guest_range(16, 0, 16).unwrap(), 16..16);
    }

    /// The bug the helper exists for: `as usize` on a negative i32 sign-extends
    /// to a huge offset, and the subsequent `+ len` wraps in a release build —
    /// sliding past a naive `end > len` check.
    #[test]
    fn guest_range_rejects_negative_and_overflow() {
        assert!(guest_range(-1, 4, 16).is_err(), "negative ptr");
        assert!(guest_range(4, -1, 16).is_err(), "negative len");
        assert!(guest_range(i32::MIN, 1, 16).is_err(), "i32::MIN ptr");
        assert!(guest_range(8, 16, 16).is_err(), "past the end");
        assert!(guest_range(17, 0, 16).is_err(), "start past the end");
    }

    #[test]
    fn guest_range_host_len_rejects_negative_pointer() {
        assert!(guest_range_host_len(-1, 4, 16).is_err());
        assert!(guest_range_host_len(4, usize::MAX, 16).is_err(), "overflow");
        assert_eq!(guest_range_host_len(4, 4, 16).unwrap(), 4..8);
    }

    // ── guest-controlled ABI values ─────────────────────────────────────────

    /// Regression: a guest whose `alloc` returns -1 used to panic the host's
    /// blocking thread on `mem_data[start..end]`. It must now come back as a
    /// clean AbiError.
    #[tokio::test]
    async fn malicious_alloc_pointer_is_rejected_not_panicked() {
        let sandbox = WasmSandbox::new().unwrap();
        let err = sandbox
            .execute(
                EVIL_ALLOC_WAT.as_bytes(),
                serde_json::json!({"hello": "world"}),
                SandboxConfig::default(),
                None,
                "test-agent",
            )
            .await
            .expect_err("a negative alloc pointer must be rejected");
        assert!(
            matches!(err, SandboxError::AbiError(_)),
            "expected AbiError, got {err:?}"
        );
    }

    /// Regression: `(packed >> 32) as usize` sign-extended a guest-chosen
    /// negative i64 and panicked on the slice.
    #[tokio::test]
    async fn malicious_packed_result_is_rejected_not_panicked() {
        let sandbox = WasmSandbox::new().unwrap();
        let err = sandbox
            .execute(
                EVIL_RESULT_WAT.as_bytes(),
                serde_json::json!({}),
                SandboxConfig::default(),
                None,
                "test-agent",
            )
            .await
            .expect_err("an out-of-range packed result must be rejected");
        assert!(
            matches!(err, SandboxError::AbiError(_)),
            "expected AbiError, got {err:?}"
        );
    }

    // ── resource limits ─────────────────────────────────────────────────────

    /// `memory_size` alone is not a memory budget — wasmtime leaves
    /// `table_elements` unlimited by default, so this module allocated ~1 GiB of
    /// host RAM while nominally capped at 1 MiB.
    #[tokio::test]
    async fn oversized_table_is_refused() {
        let sandbox = WasmSandbox::new().unwrap();
        let config = SandboxConfig {
            max_memory_bytes: 1024 * 1024,
            ..Default::default()
        };
        let err = sandbox
            .execute(
                HUGE_TABLE_WAT.as_bytes(),
                serde_json::json!({}),
                config,
                None,
                "test-agent",
            )
            .await
            .expect_err("a table far past the budget must not instantiate");
        assert!(
            matches!(err, SandboxError::Instantiation(_)),
            "expected Instantiation error, got {err:?}"
        );
    }

    /// A module whose declared initial memory exceeds the budget must be
    /// refused rather than silently granted.
    #[tokio::test]
    async fn oversized_initial_memory_is_refused() {
        let sandbox = WasmSandbox::new().unwrap();
        // 100 pages = 6.4 MiB, against a 1 MiB budget.
        let wat = r#"
            (module
                (memory (export "memory") 100)
                (func (export "alloc") (param i32) (result i32) (i32.const 0))
                (func (export "execute") (param i32) (param i32) (result i64) (i64.const 0))
            )
        "#;
        let config = SandboxConfig {
            max_memory_bytes: 1024 * 1024,
            ..Default::default()
        };
        let err = sandbox
            .execute(
                wat.as_bytes(),
                serde_json::json!({}),
                config,
                None,
                "test-agent",
            )
            .await
            .expect_err("initial memory past the budget must not instantiate");
        assert!(
            matches!(err, SandboxError::Instantiation(_)),
            "expected Instantiation error, got {err:?}"
        );
    }

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.fuel_limit, 1_000_000);
        assert_eq!(config.max_memory_bytes, 16 * 1024 * 1024);
        assert!(config.capabilities.is_empty());
    }

    #[test]
    fn test_sandbox_engine_creation() {
        let sandbox = WasmSandbox::new().unwrap();
        // Engine should be created successfully
        drop(sandbox);
    }

    #[tokio::test]
    async fn test_echo_module() {
        let sandbox = WasmSandbox::new().unwrap();
        let input = serde_json::json!({"hello": "world", "num": 42});
        let config = SandboxConfig::default();

        let result = sandbox
            .execute(
                ECHO_WAT.as_bytes(),
                input.clone(),
                config,
                None,
                "test-agent",
            )
            .await
            .unwrap();

        assert_eq!(result.output, input);
        assert!(result.fuel_consumed > 0);
    }

    #[tokio::test]
    async fn test_fuel_exhaustion() {
        let sandbox = WasmSandbox::new().unwrap();
        let input = serde_json::json!({});
        let config = SandboxConfig {
            fuel_limit: 10_000,
            ..Default::default()
        };

        let err = sandbox
            .execute(
                INFINITE_LOOP_WAT.as_bytes(),
                input,
                config,
                None,
                "test-agent",
            )
            .await
            .unwrap_err();

        assert!(
            matches!(err, SandboxError::FuelExhausted),
            "Expected FuelExhausted, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_host_call_time_now() {
        let sandbox = WasmSandbox::new().unwrap();
        // time_now requires no capabilities
        let input = serde_json::json!({"method": "time_now", "params": {}});
        let config = SandboxConfig::default();

        let result = sandbox
            .execute(
                HOST_CALL_PROXY_WAT.as_bytes(),
                input,
                config,
                None,
                "test-agent",
            )
            .await
            .unwrap();

        // Response should be {"ok": <timestamp>}
        assert!(
            result.output.get("ok").is_some(),
            "Expected ok field: {:?}",
            result.output
        );
        let ts = result.output["ok"].as_u64().unwrap();
        assert!(ts > 1_700_000_000, "Timestamp looks too small: {ts}");
    }

    #[tokio::test]
    async fn test_host_call_capability_denied() {
        let sandbox = WasmSandbox::new().unwrap();
        // Try fs_read with no capabilities → denied
        let input = serde_json::json!({
            "method": "fs_read",
            "params": {"path": "/etc/passwd"}
        });
        let config = SandboxConfig {
            capabilities: vec![], // No capabilities!
            ..Default::default()
        };

        let result = sandbox
            .execute(
                HOST_CALL_PROXY_WAT.as_bytes(),
                input,
                config,
                None,
                "test-agent",
            )
            .await
            .unwrap();

        // Response should contain "error" with "denied"
        let err_msg = result.output["error"].as_str().unwrap_or("");
        assert!(
            err_msg.contains("denied"),
            "Expected capability denied, got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_host_call_unknown_method() {
        let sandbox = WasmSandbox::new().unwrap();
        let input = serde_json::json!({"method": "nonexistent_method", "params": {}});
        let config = SandboxConfig::default();

        let result = sandbox
            .execute(
                HOST_CALL_PROXY_WAT.as_bytes(),
                input,
                config,
                None,
                "test-agent",
            )
            .await
            .unwrap();

        let err_msg = result.output["error"].as_str().unwrap_or("");
        assert!(
            err_msg.contains("Unknown"),
            "Expected unknown method error, got: {err_msg}"
        );
    }
}
