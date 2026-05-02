//! Wasm sandbox - Isolated plugin execution environment

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

use super::host::SoloHostFunctions;
use super::manifest::PluginManifest;

/// Sandbox state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxState {
    /// Sandbox is ready
    Ready,
    /// Sandbox is running a plugin
    Running,
    /// Sandbox is terminated
    Terminated,
}

/// Sandbox for isolated plugin execution
pub struct WasmSandbox {
    engine: Engine,
    wasi_ctx: wasmtime_wasi::WasiCtx,
    state: SandboxState,
}

impl WasmSandbox {
    /// Create a new sandbox
    pub fn new() -> Result<Self, String> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.wasm_multi_memory(true);

        let engine = Engine::new(&config).map_err(|e| format!("Failed to create engine: {}", e))?;

        let wasi_ctx = WasiCtxBuilder::new().build();

        Ok(Self {
            engine,
            wasi_ctx,
            state: SandboxState::Ready,
        })
    }

    /// Load a plugin module with handshake verification
    pub fn load_plugin(
        &self,
        wasm_bytes: &[u8],
        manifest: &PluginManifest,
        whitelist: &HashMap<String, HashMap<String, String>>,
    ) -> Result<Module, String> {
        // Verify plugin hash against whitelist
        if let Some(version_hashes) = whitelist.get(&manifest.plugin_id) {
            if let Some(expected_hash) = version_hashes.get(&manifest.version) {
                let computed_hash = Self::compute_hash(wasm_bytes);
                if computed_hash != *expected_hash {
                    return Err("Plugin hash verification failed".to_string());
                }
            }
        }

        // Compile module
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| format!("Failed to compile module: {}", e))?;

        Ok(module)
    }

    /// Compute SHA-256 hash of wasm bytes
    fn compute_hash(wasm_bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(wasm_bytes);
        format!("{:x}", hasher.finalize())
    }

    /// Create a store with host functions
    pub fn create_store(&self, host_functions: SoloHostFunctions) -> Store<SoloHostFunctions> {
        Store::new(&self.engine, host_functions)
    }

    /// Terminate the sandbox
    pub fn terminate(&mut self) {
        self.state = SandboxState::Terminated;
    }
}

impl Default for WasmSandbox {
    fn default() -> Self {
        Self::new().expect("Failed to create default sandbox")
    }
}
