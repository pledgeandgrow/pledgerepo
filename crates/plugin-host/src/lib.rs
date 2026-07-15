// WASM plugin host: runs framework compilers and custom transforms
// in a sandboxed WebAssembly runtime
//
// Architecture:
//   1. Load .wasm plugin files
//   2. Call `transform(source, path, options) -> { code, map, deps }`
//   3. Shared linear memory for zero-copy data passing
//
// Plugin protocol (WIT):
//   interface plugin {
//     transform: func(source: string, path: string) -> result<transform-result>;
//   }
//
// Two-tier system:
//   Tier 1: Native Rust transforms (SWC, Lightning CSS) — zero overhead
//   Tier 2: WASM plugins (Svelte, Vue, custom) — sandboxed, near-native

pub mod vite_compat;
pub use vite_compat::{VitePlugin, VitePluginHost, RollupPluginHost, Enforce, Apply};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};
use wasmtime::*;

/// Plugin transform result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTransformResult {
    pub code: String,
    pub source_map: Option<String>,
    pub deps: Vec<String>,
}

/// A loaded WASM plugin
pub struct WasmPlugin {
    name: String,
    #[allow(dead_code)]
    module: Module,
    store: Store<()>,
    /// Transform function: (ptr: i32, len: i32, path_ptr: i32, path_len: i32) -> i32 (result_ptr)
    transform_func: Option<Func>,
    /// WASM linear memory for data passing
    memory: Option<Memory>,
    /// Allocate function: (len: i32) -> i32 (ptr)
    alloc_func: Option<Func>,
}

/// The plugin host manages all loaded plugins
pub struct PluginHost {
    engine: Engine,
    plugins: Vec<Arc<RwLock<WasmPlugin>>>,
}

use std::sync::RwLock;

impl PluginHost {
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        Ok(Self {
            engine,
            plugins: Vec::new(),
        })
    }

    /// Load a WASM plugin from a file
    pub fn load_plugin(&mut self, path: &PathBuf) -> Result<()> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        info!("Loading WASM plugin: {} from {:?}", name, path);

        let module = Module::from_file(&self.engine, path)?;
        let mut store = Store::new(&self.engine, ());

        let instance = Instance::new(&mut store, &module, &[])?;

        // Look for exports
        let transform_func = instance.get_func(&mut store, "transform");
        let memory = instance.get_memory(&mut store, "memory");
        let alloc_func = instance.get_func(&mut store, "alloc");

        let plugin = WasmPlugin {
            name: name.clone(),
            module,
            store,
            transform_func,
            memory,
            alloc_func,
        };

        self.plugins.push(Arc::new(RwLock::new(plugin)));
        debug!("Plugin {} loaded successfully", name);

        Ok(())
    }

    /// Run transform on all plugins that match the file type
    pub fn transform(
        &self,
        source: &str,
        file_path: &str,
    ) -> Result<Option<PluginTransformResult>> {
        for plugin in &self.plugins {
            let result = self.call_plugin(plugin, source, file_path)?;
            if let Some(result) = result {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    fn call_plugin(
        &self,
        plugin: &Arc<RwLock<WasmPlugin>>,
        source: &str,
        file_path: &str,
    ) -> Result<Option<PluginTransformResult>> {
        let mut p = plugin.write().unwrap();

        let func = match &p.transform_func {
            Some(f) => f.clone(),
            None => return Ok(None),
        };

        let memory = match &p.memory {
            Some(m) => m.clone(),
            None => return Ok(None),
        };

        let alloc = match &p.alloc_func {
            Some(f) => f.clone(),
            None => return Ok(None),
        };

        // 1. Allocate memory in WASM for source string
        let source_bytes = source.as_bytes();
        let source_ptr = self.wasm_alloc(&mut p.store, &alloc, source_bytes.len() as i32)?;
        self.wasm_write(&mut p.store, &memory, source_ptr, source_bytes)?;

        // 2. Allocate memory for file path
        let path_bytes = file_path.as_bytes();
        let path_ptr = self.wasm_alloc(&mut p.store, &alloc, path_bytes.len() as i32)?;
        self.wasm_write(&mut p.store, &memory, path_ptr, path_bytes)?;

        // 3. Call transform(source_ptr, source_len, path_ptr, path_len) → result_ptr
        let mut result_ptr = [Val::I32(0)];
        func.call(
            &mut p.store,
            &[
                Val::I32(source_ptr),
                Val::I32(source_bytes.len() as i32),
                Val::I32(path_ptr),
                Val::I32(path_bytes.len() as i32),
            ],
            &mut result_ptr,
        )?;

        let result_ptr = match result_ptr[0] {
            Val::I32(p) => p,
            _ => return Ok(None),
        };

        if result_ptr == 0 {
            return Ok(None);
        }

        // 4. Read result from WASM memory
        // Result format: [result_len: i32 (4 bytes)] [json data: result_len bytes]
        let len_bytes = self.wasm_read(&mut p.store, &memory, result_ptr, 4)?;
        let result_len = i32::from_le_bytes([
            len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3],
        ]) as usize;

        if result_len == 0 {
            return Ok(None);
        }

        let json_bytes = self.wasm_read(&mut p.store, &memory, result_ptr + 4, result_len)?;
        let json_str = String::from_utf8_lossy(&json_bytes);

        let result: PluginTransformResult = serde_json::from_str(&json_str)?;
        Ok(Some(result))
    }

    /// Allocate memory in WASM linear memory
    fn wasm_alloc(&self, store: &mut Store<()>, alloc: &Func, len: i32) -> Result<i32> {
        let mut result = [Val::I32(0)];
        alloc.call(store, &[Val::I32(len)], &mut result)?;
        match result[0] {
            Val::I32(ptr) => Ok(ptr),
            _ => anyhow::bail!("alloc returned non-i32"),
        }
    }

    /// Write bytes into WASM linear memory
    fn wasm_write(&self, store: &mut Store<()>, memory: &Memory, ptr: i32, data: &[u8]) -> Result<()> {
        memory.write(store, ptr as usize, data)?;
        Ok(())
    }

    /// Read bytes from WASM linear memory
    fn wasm_read(&self, store: &mut Store<()>, memory: &Memory, ptr: i32, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        memory.read(store, ptr as usize, &mut buf)?;
        Ok(buf)
    }

    /// List loaded plugins
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins
            .iter()
            .map(|p| p.read().unwrap().name.clone())
            .collect()
    }
}

/// Compile a JavaScript Vite/Rollup plugin to WASM using javy.
/// This shells out to the javy CLI (ByteDance's JS-to-WASM compiler).
/// The resulting WASM file can be loaded by the PluginHost.
pub fn compile_js_plugin_to_wasm(js_path: &PathBuf) -> Result<PathBuf> {
    let wasm_path = js_path.with_extension("wasm");

    // Try to run javy compile
    let output = std::process::Command::new("javy")
        .arg("compile")
        .arg(js_path)
        .arg("-o")
        .arg(&wasm_path)
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                info!("Compiled JS plugin to WASM: {:?} → {:?}", js_path, wasm_path);
                Ok(wasm_path)
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                anyhow::bail!(
                    "javy compile failed: {}. Make sure javy is installed: npm install -g @bytecodealliance/javy",
                    stderr
                )
            }
        }
        Err(_) => {
            // javy not found — fall back to using the JS plugin host instead
            warn!(
                "javy CLI not found. JS plugin {:?} will be run via the embedded JS runtime instead of WASM.",
                js_path
            );
            anyhow::bail!(
                "javy CLI not installed. Install it with: npm install -g @bytecodealliance/javy\n\
                 Alternatively, JS plugins can be run directly via the embedded JS runtime."
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_host_creation() {
        let host = PluginHost::new();
        assert!(host.is_ok());
    }
}
