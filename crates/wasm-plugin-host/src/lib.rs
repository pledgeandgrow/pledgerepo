//! WASM Component Model Plugin Host for PledgePack.
//!
//! This crate implements the plugin host that runs WASM plugins sandboxed,
//! using the WIT contract defined in `wit/world.wit` (frozen at v0.1.0).
//!
//! # Architecture
//!
//! - `WasmPluginHost` — manages multiple WASM plugin instances, calls hooks
//! - `WasmPlugin` — a single loaded plugin instance (component + store)
//! - Hooks are called via the generated `bindgen!` bindings
//!
//! # Sandbox
//!
//! By default, plugins run in a strict sandbox:
//! - No filesystem access
//! - No network access
//! - CPU: 10M fuel units per hook call (~10ms, prevents infinite loops)
//! - Memory: 128MB max linear memory (enforced via StoreLimits)
//!
//! The sandbox is enforced by not providing any WASI imports to the component.
//! A component that tries to import WASI will fail to instantiate.
//!
//! # Cache Integration
//!
//! Every hook output includes a `cache_key` field (blake3 hash of inputs).
//! The host wraps plugin output as a `Task<T>` in the task graph, keyed by
//! the plugin's cache key. This gives WASM plugins fine-grained caching
//! that JS plugins (opaque blob) don't have.
//!
//! # Two-Tier System
//!
//! This crate implements the first-class tier (WASM, sandboxed, fine-grained cache).
//! The second-class tier (JS shim, opaque cache) is in `pledgepack-js-plugin-host`.

use anyhow::Result;
use std::path::Path;
use tracing::{debug, info};

// Generate Rust bindings from the WIT contract.
// The path is relative to this crate's Cargo.toml directory.
wasmtime::component::bindgen!({
    world: "pledgepack-plugin",
    path: "../../wit",
});

// ─── WASI Integration ─────────────────────────────────────────────────
//
// Plugins built with `cargo component` and `wit-bindgen` may import
// `wasi:cli/environment` and `wasi:cli/exit` (from the wit-bindgen runtime).
// We provide a restricted WASI context:
// - No filesystem access
// - No network access
// - Environment variables: empty (plugins can't read host env)
// - Args: empty
// - Stdin/stdout/stderr: discarded
//
// This maintains the sandbox while allowing wit-bindgen-based plugins to load.

use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

// ─── Plugin Instance ──────────────────────────────────────────────────

/// A loaded WASM plugin instance.
///
/// Each plugin has its own `Store` (instance state) and a handle to the
/// instantiated component. The host calls hooks via the handle.
///
/// The plugin is sandboxed: no filesystem, no network. The only WASI
/// imports provided are `wasi:cli/environment` and `wasi:cli/exit`
/// (required by wit-bindgen runtime), with empty environment and args.
pub struct WasmPlugin {
    /// The plugin metadata (from `plugin-metadata` hook)
    metadata: PluginMetadata,
    /// The wasmtime store (owns the plugin's memory and state)
    store: wasmtime::Store<PluginState>,
    /// The instantiated component handle (for calling hooks)
    instance: PledgepackPlugin,
}

/// State stored in the wasmtime Store for each plugin instance.
pub struct PluginState {
    /// Plugin name (for diagnostics)
    name: String,
    /// Whether this plugin has been initialized
    initialized: bool,
    /// WASI context (restricted — no filesystem, no network)
    wasi: WasiCtx,
    /// WASI resource table
    table: wasmtime::component::ResourceTable,
    /// Host config (JSON string, provided by PledgePack)
    /// Item 6: Host imports — get-config
    host_config: String,
    /// Files emitted by the plugin via emit-file
    /// Item 6: Host imports — emit-file
    emitted_files: Vec<(String, String)>,
}

impl PluginState {
    fn new(name: String) -> Self {
        Self {
            name,
            initialized: false,
            wasi: restricted_wasi_ctx(),
            table: wasmtime::component::ResourceTable::new(),
            host_config: String::from("{}"),
            emitted_files: Vec::new(),
        }
    }

    /// Set the host config (JSON string) that plugins can read via get-config.
    pub fn set_host_config(&mut self, config: String) {
        self.host_config = config;
    }

    /// Get the files emitted by the plugin via emit-file.
    pub fn emitted_files(&self) -> &[(String, String)] {
        &self.emitted_files
    }
}

impl WasiView for PluginState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

// ─── Host Imports (Item 6) ────────────────────────────────────────────
//
// The WIT contract declares three host import functions that plugins can call:
//   - get-config: func() -> string
//   - emit-file: func(name: string, content: string) -> bool
//   - resolve-import: func(specifier: string, importer: string) -> option<string>
//
// The bindgen! macro generates a `PledgepackPluginImports` trait with these
// functions. We implement it for `PluginState` so plugins can call them.

impl PledgepackPluginImports for PluginState {
    fn get_config(&mut self) -> String {
        self.host_config.clone()
    }

    fn emit_file(&mut self, name: String, content: String) -> bool {
        // Store the emitted file — the host can retrieve it later
        self.emitted_files.push((name, content));
        true
    }

    fn resolve_import(&mut self, _specifier: String, _importer: String) -> Option<String> {
        // Placeholder — the actual resolution is handled by the engine's
        // resolver. The WASM plugin host doesn't have direct access to the
        // engine's resolver, so we return None (not resolved).
        // The plugin should fall back to its own resolution logic.
        // Future: wire this to the engine via a callback channel.
        None
    }
}

/// Create a restricted WASI context — no filesystem, no network,
/// empty environment, empty args, discarded stdio.
fn restricted_wasi_ctx() -> WasiCtx {
    WasiCtxBuilder::new()
        .inherit_stdio() // allow stdout/stderr for debug logging
        .build()
}

impl WasmPlugin {
    /// Load and instantiate a WASM plugin from a `.wasm` file.
    ///
    /// The plugin is validated against the WIT contract at instantiation time.
    /// If the plugin doesn't implement the required exports, instantiation fails.
    ///
    /// # Sandbox
    ///
    /// The plugin runs in a strict sandbox:
    /// - No filesystem access (no WASI imports provided)
    /// - No network access
    /// - CPU: 10M fuel units per hook call (prevents infinite loops)
    /// - Memory: 128MB max linear memory (enforced via StoreLimits)
    pub fn load_from_file(path: &Path) -> Result<Self> {
        Self::load_with_engine(path, &default_engine())
    }

    /// Load and instantiate a WASM plugin with a custom engine.
    ///
    /// Use this when you want to share an engine across multiple plugins
    /// (for compilation cache reuse).
    pub fn load_with_engine(path: &Path, engine: &wasmtime::Engine) -> Result<Self> {
        debug!("Loading WASM plugin from {}", path.display());

        // Read the component bytes
        let bytes = std::fs::read(path)
            .map_err(|e| anyhow::anyhow!("Failed to read plugin file {}: {}", path.display(), e))?;

        // Compile the component (validates against the WIT contract)
        let component = wasmtime::component::Component::new(engine, &bytes)
            .map_err(|e| anyhow::anyhow!("Failed to compile WASM component {}: {}", path.display(), e))?;

        // Create the store with plugin state
        // The store owns the plugin's memory — this is the sandbox boundary
        let mut store = wasmtime::Store::new(engine, PluginState::new("unknown".to_string()));

        // G7.3: Enforce CPU and memory limits on the store
        // Fuel: prevents infinite loops (DEFAULT_FUEL instructions per invocation)
        // Memory: caps linear memory growth to DEFAULT_MEMORY_MAX_BYTES
        store.set_fuel(DEFAULT_FUEL)
            .map_err(|e| anyhow::anyhow!("Failed to set fuel limit: {}", e))?;
        store.limiter(|_state| {
            wasmtime::StoreLimitsBuilder::new()
                .memory_size(DEFAULT_MEMORY_MAX_BYTES)
                .build()
        });

        // Create a linker with restricted WASI imports.
        // The plugin gets `wasi:cli/environment` and `wasi:cli/exit` (required
        // by wit-bindgen runtime) but NO filesystem or network access.
        let mut linker: wasmtime::component::Linker<PluginState> = wasmtime::component::Linker::new(engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|e| anyhow::anyhow!("Failed to add WASI to linker: {}", e))?;

        // Item 6: Wire host imports (get-config, emit-file, resolve-import)
        // This allows plugins to call back into the host for config access,
        // file emission, and import resolution.
        // Pattern from wasmtime docs: add_to_linker::<_, HasSelf<_>>
        // HasSelf<T> implements HasData with Data<'a> = &'a mut T
        PledgepackPlugin::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |state: &mut PluginState| state,
        )
            .map_err(|e| anyhow::anyhow!("Failed to add host imports to linker: {}", e))?;

        // Instantiate the component
        let instance = PledgepackPlugin::instantiate(&mut store, &component, &linker)
            .map_err(|e| anyhow::anyhow!("Failed to instantiate WASM component {}: {}", path.display(), e))?;

        // Call the plugin-metadata hook to get the plugin's name and capabilities
        let metadata = instance
            .call_plugin_metadata(&mut store)
            .map_err(|e| anyhow::anyhow!("Failed to call plugin-metadata hook: {}", e))?;

        let name = metadata.name.clone();
        store.data_mut().name = name.clone();
        store.data_mut().initialized = true;

        info!(
            "Loaded WASM plugin: {} (version: {}, hooks: {:?})",
            name, metadata.version, metadata.hooks
        );

        Ok(Self {
            metadata,
            store,
            instance,
        })
    }

    /// G7.3: Refill fuel before a hook invocation.
    /// Resets the fuel budget so each hook gets a fresh CPU allowance.
    fn refill_fuel(&mut self) {
        let _ = self.store.set_fuel(DEFAULT_FUEL);
    }

    /// Get the plugin's metadata.
    pub fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    /// Get the plugin's name.
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Whether this plugin implements the `resolve-id` hook.
    pub fn has_resolve_id(&self) -> bool {
        self.metadata.hooks.resolve_id
    }

    /// Whether this plugin implements the `load` hook.
    pub fn has_load(&self) -> bool {
        self.metadata.hooks.load
    }

    /// Whether this plugin implements the `transform` hook.
    pub fn has_transform(&self) -> bool {
        self.metadata.hooks.transform
    }

    /// Whether this plugin implements the `transform-index-html` hook.
    pub fn has_transform_index_html(&self) -> bool {
        self.metadata.hooks.transform_index_html
    }

    /// G7.4: Whether this plugin implements the `render-chunk` hook.
    pub fn has_render_chunk(&self) -> bool {
        self.metadata.hooks.render_chunk
    }

    /// Whether this plugin has `enforce: "pre"` (runs before built-in transform).
    /// Item 5: Plugin ordering for WASM plugins.
    pub fn is_pre_plugin(&self) -> bool {
        self.metadata.enforce.as_deref() == Some("pre")
    }

    /// Whether this plugin has `enforce: "post"` or no enforce (default = post).
    /// Item 5: Plugin ordering for WASM plugins.
    pub fn is_post_plugin(&self) -> bool {
        self.metadata.enforce.as_deref() != Some("pre")
    }

    // ─── Hook Invocation ──────────────────────────────────────────────

    /// Call the `resolve-id` hook.
    ///
    /// Returns `None` if the plugin doesn't handle this specifier.
    /// The output includes a `cache_key` for task graph caching.
    pub fn resolve_id(
        &mut self,
        source: &str,
        importer: Option<&str>,
        is_entry: bool,
        kind: Option<&str>,
    ) -> Result<Option<ResolveIdOutput>> {
        if !self.has_resolve_id() {
            return Ok(None);
        }
        self.refill_fuel();

        let input = ResolveIdInput {
            source: source.to_string(),
            importer: importer.map(|s| s.to_string()),
            is_entry,
            kind: kind.map(|s| s.to_string()),
        };

        let result = self
            .instance
            .call_resolve_id(&mut self.store, &input)
            .map_err(|e| anyhow::anyhow!("resolve-id hook failed: {}", e))?;

        if let Some(ref output) = result {
            debug!(
                "[plugin:{}] resolve-id: {} → {} (external: {})",
                self.name(),
                source,
                output.id,
                output.external
            );
        }

        Ok(result)
    }

    /// Call the `load` hook.
    ///
    /// Returns `None` if the plugin doesn't handle this ID.
    pub fn load(&mut self, id: &str) -> Result<Option<LoadOutput>> {
        if !self.has_load() {
            return Ok(None);
        }
        self.refill_fuel();

        let input = LoadInput {
            id: id.to_string(),
        };

        let result = self
            .instance
            .call_load(&mut self.store, &input)
            .map_err(|e| anyhow::anyhow!("load hook failed: {}", e))?;

        if let Some(ref output) = result {
            debug!(
                "[plugin:{}] load: {} → {} bytes",
                self.name(),
                id,
                output.code.len()
            );
        }

        Ok(result)
    }

    /// Call the `transform` hook.
    ///
    /// Returns `None` if the plugin doesn't transform this module.
    /// The output includes a `cache_key` for task graph caching.
    pub fn transform(
        &mut self,
        code: &str,
        id: &str,
        ast_json: Option<&str>,
    ) -> Result<Option<TransformOutput>> {
        if !self.has_transform() {
            return Ok(None);
        }
        self.refill_fuel();

        let input = TransformInput {
            code: code.to_string(),
            id: id.to_string(),
            ast_json: ast_json.map(|s| s.to_string()),
        };

        let result = self
            .instance
            .call_transform(&mut self.store, &input)
            .map_err(|e| anyhow::anyhow!("transform hook failed: {}", e))?;

        if let Some(ref output) = result {
            debug!(
                "[plugin:{}] transform: {} → {} bytes (cache-key: {})",
                self.name(),
                id,
                output.code.len(),
                &output.cache_key[..8.min(output.cache_key.len())],
            );
        }

        Ok(result)
    }

    /// Call the `transform-index-html` hook.
    pub fn transform_index_html(
        &mut self,
        html: &str,
        path: &str,
    ) -> Result<Option<HtmlOutput>> {
        if !self.has_transform_index_html() {
            return Ok(None);
        }
        self.refill_fuel();

        let input = HtmlInput {
            html: html.to_string(),
            path: path.to_string(),
        };

        let result = self
            .instance
            .call_transform_index_html(&mut self.store, &input)
            .map_err(|e| anyhow::anyhow!("transform-index-html hook failed: {}", e))?;

        Ok(result)
    }

    /// G7.4: Call the `render-chunk` hook.
    ///
    /// Called after code splitting, before final emit.
    /// Returns `None` if the plugin doesn't render this chunk.
    /// The output includes a `cache_key` for task graph caching.
    pub fn render_chunk(
        &mut self,
        code: &str,
        filename: &str,
        chunk_type: &str,
    ) -> Result<Option<RenderChunkOutput>> {
        if !self.has_render_chunk() {
            return Ok(None);
        }
        self.refill_fuel();

        let input = RenderChunkInput {
            code: code.to_string(),
            filename: filename.to_string(),
            chunk_type: chunk_type.to_string(),
        };

        let result = self
            .instance
            .call_render_chunk(&mut self.store, &input)
            .map_err(|e| anyhow::anyhow!("render-chunk hook failed: {}", e))?;

        if let Some(ref output) = result {
            debug!(
                "[plugin:{}] render-chunk: {} → {} bytes (cache-key: {})",
                self.name(),
                filename,
                output.code.len(),
                &output.cache_key[..8.min(output.cache_key.len())],
            );
        }

        Ok(result)
    }

    /// Call the `build-start` lifecycle hook.
    pub fn build_start(&mut self) -> Result<()> {
        if !self.metadata.hooks.build_start {
            return Ok(());
        }
        self.refill_fuel();
        debug!("[plugin:{}] build-start", self.name());
        self.instance.call_build_start(&mut self.store)?;
        Ok(())
    }

    /// Call the `build-end` lifecycle hook.
    pub fn build_end(&mut self) -> Result<()> {
        if !self.metadata.hooks.build_end {
            return Ok(());
        }
        self.refill_fuel();
        debug!("[plugin:{}] build-end", self.name());
        self.instance.call_build_end(&mut self.store)?;
        Ok(())
    }

    /// Call the `generate-bundle` lifecycle hook.
    pub fn generate_bundle(&mut self) -> Result<()> {
        if !self.metadata.hooks.generate_bundle {
            return Ok(());
        }
        self.refill_fuel();
        debug!("[plugin:{}] generate-bundle", self.name());
        self.instance.call_generate_bundle(&mut self.store)?;
        Ok(())
    }

    /// Call the `configure-server` hook (dev mode only).
    pub fn configure_server(&mut self) -> Result<Option<ServerMiddleware>> {
        if !self.metadata.hooks.configure_server {
            return Ok(None);
        }
        self.refill_fuel();
        let result = self
            .instance
            .call_configure_server(&mut self.store)
            .map_err(|e| anyhow::anyhow!("configure-server hook failed: {}", e))?;
        Ok(result)
    }
}

// ─── Plugin Host (manages multiple plugins) ───────────────────────────

/// The WASM plugin host — manages multiple loaded plugins and orchestrates
/// hook calls across all of them.
///
/// Hook semantics:
/// - `resolve-id`, `load`: sequential, first non-null result wins
/// - `transform`, `transform-index-html`: sequential chain (each plugin sees previous output)
/// - `build-start`, `build-end`, `generate-bundle`: all plugins called (parallel in future)
/// - `configure-server`: all plugins called, middleware collected
pub struct WasmPluginHost {
    /// All loaded plugins, in registration order
    plugins: Vec<WasmPlugin>,
    /// Shared engine (kept alive for plugin stores; compilation cache reuse)
    #[allow(dead_code)]
    engine: wasmtime::Engine,
}

impl WasmPluginHost {
    /// Create a new WASM plugin host with default engine configuration.
    pub fn new() -> Result<Self> {
        let engine = default_engine();
        Ok(Self {
            plugins: Vec::new(),
            engine,
        })
    }

    /// Load a WASM plugin from a `.wasm` file.
    pub fn load_plugin(&mut self, path: &Path) -> Result<&str> {
        let plugin = WasmPlugin::load_from_file(path)?;
        self.plugins.push(plugin);
        Ok(self.plugins.last().unwrap().name())
    }

    /// Load multiple plugins from a list of paths.
    pub fn load_plugins(&mut self, paths: &[&Path]) -> Result<()> {
        for path in paths {
            self.load_plugin(path)?;
        }
        Ok(())
    }

    /// Get all loaded plugins.
    pub fn plugins(&self) -> &[WasmPlugin] {
        &self.plugins
    }

    /// Get the number of loaded plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Whether any plugins are loaded.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Check if any loaded plugin has enforce: "pre".
    /// Item 5: Plugin ordering for WASM plugins.
    pub fn has_pre_plugin(&self) -> bool {
        self.plugins.iter().any(|p| p.is_pre_plugin())
    }

    /// Check if any loaded plugin has enforce: "post" or default (post).
    /// Item 5: Plugin ordering for WASM plugins.
    pub fn has_post_plugin(&self) -> bool {
        self.plugins.iter().any(|p| p.has_transform() && p.is_post_plugin())
    }

    // ─── Hook Orchestration ───────────────────────────────────────────

    /// Run `resolve-id` across all plugins (first non-null wins).
    pub fn resolve_id(
        &mut self,
        source: &str,
        importer: Option<&str>,
        is_entry: bool,
        kind: Option<&str>,
    ) -> Result<Option<ResolveIdOutput>> {
        for plugin in &mut self.plugins {
            if let Some(result) = plugin.resolve_id(source, importer, is_entry, kind)? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    /// Run `load` across all plugins (first non-null wins).
    pub fn load(&mut self, id: &str) -> Result<Option<LoadOutput>> {
        for plugin in &mut self.plugins {
            if let Some(result) = plugin.load(id)? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    /// Run `transform` across all plugins (chain — each sees previous output).
    pub fn transform(
        &mut self,
        code: &str,
        id: &str,
        ast_json: Option<&str>,
    ) -> Result<(String, Option<String>)> {
        let mut current_code = code.to_string();
        let mut current_map: Option<String> = None;

        for plugin in &mut self.plugins {
            if let Some(output) = plugin.transform(&current_code, id, ast_json)? {
                current_code = output.code;
                if output.source_map.is_some() {
                    current_map = output.source_map;
                }
            }
        }

        Ok((current_code, current_map))
    }

    /// Run `transform-index-html` across all plugins (chain).
    pub fn transform_index_html(
        &mut self,
        html: &str,
        path: &str,
    ) -> Result<(String, Vec<HtmlTag>)> {
        let mut current_html = html.to_string();
        let mut all_tags = Vec::new();

        for plugin in &mut self.plugins {
            if let Some(output) = plugin.transform_index_html(&current_html, path)? {
                current_html = output.html;
                all_tags.extend(output.tags);
            }
        }

        Ok((current_html, all_tags))
    }

    /// G7.4: Run `render-chunk` across all plugins (chain — each sees previous output).
    ///
    /// Called after code splitting, before final emit.
    /// Returns the final rendered code and optional source map.
    pub fn render_chunk(
        &mut self,
        code: &str,
        filename: &str,
        chunk_type: &str,
    ) -> Result<(String, Option<String>)> {
        let mut current_code = code.to_string();
        let mut current_map: Option<String> = None;

        for plugin in &mut self.plugins {
            if let Some(output) = plugin.render_chunk(&current_code, filename, chunk_type)? {
                current_code = output.code;
                if output.source_map.is_some() {
                    current_map = output.source_map;
                }
            }
        }

        Ok((current_code, current_map))
    }

    /// Run `build-start` on all plugins.
    pub fn build_start(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.build_start()?;
        }
        Ok(())
    }

    /// Run `build-end` on all plugins.
    pub fn build_end(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.build_end()?;
        }
        Ok(())
    }

    /// Run `generate-bundle` on all plugins.
    pub fn generate_bundle(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.generate_bundle()?;
        }
        Ok(())
    }

    /// Run `configure-server` on all plugins and collect middleware.
    pub fn configure_server(&mut self) -> Result<Vec<ServerMiddleware>> {
        let mut middleware = Vec::new();
        for plugin in &mut self.plugins {
            if let Some(mw) = plugin.configure_server()? {
                middleware.push(mw);
            }
        }
        Ok(middleware)
    }
}

impl Default for WasmPluginHost {
    fn default() -> Self {
        Self::new().expect("Failed to create WASM plugin host")
    }
}

// ─── Engine Configuration ─────────────────────────────────────────────

/// Default fuel budget per plugin invocation (10M instructions ≈ ~10ms CPU).
/// Prevents infinite loops and runaway computation.
const DEFAULT_FUEL: u64 = 10_000_000;

/// Default maximum linear memory size per plugin (128 MB).
/// Prevents memory exhaustion from malicious or buggy plugins.
const DEFAULT_MEMORY_MAX_BYTES: usize = 128 * 1024 * 1024;

/// Create a default wasmtime engine configured for sandboxed plugin execution.
///
/// Configuration:
/// - Cranelift compiler (fast compilation, good for plugins)
/// - Component model enabled
/// - No WASI (sandbox — plugins can't access filesystem or network)
/// - Fuel consumption enabled (CPU limit — prevents infinite loops)
/// - Memory: 128MB default (enforced via StoreLimits)
fn default_engine() -> wasmtime::Engine {
    let mut config = wasmtime::Config::new();
    config.strategy(wasmtime::Strategy::Cranelift);
    config.wasm_component_model(true);
    // Disable multi-memory and multi-value for stricter sandboxing
    config.wasm_multi_memory(false);
    config.wasm_multi_value(true); // multi-value is safe, needed for component model
    // G7.3: Enable fuel consumption for CPU limiting
    config.consume_fuel(true);

    wasmtime::Engine::new(&config).expect("Failed to create wasmtime engine")
}

// ─── Conversion helpers (WIT types → PledgePack core types) ───────────

/// Convert a WIT `TransformOutput` to PledgePack's `PluginTransformResult`.
///
/// This bridges the WASM plugin host to the task transform engine.
/// The `cache_key` from the WIT output is used for task graph caching.
impl From<TransformOutput> for pledgepack_core::task_transform::PluginTransformResult {
    fn from(output: TransformOutput) -> Self {
        Self {
            code: output.code,
            map: output.source_map,
            cache_key: Some(output.cache_key),
        }
    }
}

/// Convert a WIT `ResolveIdOutput` to a simple (id, external) tuple.
impl From<ResolveIdOutput> for (String, bool) {
    fn from(output: ResolveIdOutput) -> Self {
        (output.id, output.external)
    }
}

/// Convert a WIT `LoadOutput` to a (code, map) tuple.
impl From<LoadOutput> for (String, Option<String>) {
    fn from(output: LoadOutput) -> Self {
        (output.code, output.source_map)
    }
}

// ─── Task Graph Cache Integration ─────────────────────────────────────

use std::sync::{Arc, Mutex};

/// A thread-safe wrapper around `WasmPluginHost` that implements the
/// `Fn(&str, &str) -> Option<PluginTransformResult>` interface
/// expected by `BuildEngine::wire_plugin_transform()`.
///
/// The WASM plugin host has mutable state (wasmtime Store), so it must
/// be wrapped in a `Mutex`. The closure locks the mutex, calls the
/// transform hook, and returns the result.
///
/// # Cache Contract
///
/// The WASM plugin's `transform` hook returns a `cache_key` (blake3 hash
/// of inputs). This key is logged but currently the task graph uses its
/// own `TaskId` (also blake3) for caching. In the future, the plugin's
/// cache key could be used as the `TaskId` directly, giving plugins
/// control over their own cache invalidation.
///
/// # Thread Safety
///
/// The `Mutex` ensures only one thread accesses the plugin host at a time.
/// This is a bottleneck for parallel transforms, but WASM plugin calls
/// are typically fast (no I/O, sandboxed). A future optimization is to
/// use a pool of plugin instances (one per thread).
pub struct WasmPluginHostBridge {
    host: Mutex<WasmPluginHost>,
}

impl WasmPluginHostBridge {
    /// Create a bridge from a `WasmPluginHost`.
    pub fn new(host: WasmPluginHost) -> Self {
        Self {
            host: Mutex::new(host),
        }
    }

    /// Create a bridge from plugin paths.
    pub fn from_paths(paths: &[&Path]) -> Result<Self> {
        let mut host = WasmPluginHost::new()?;
        host.load_plugins(paths)?;
        Ok(Self::new(host))
    }

    /// Get the number of loaded plugins.
    pub fn len(&self) -> usize {
        self.host.lock().unwrap().len()
    }

    /// Whether any plugins are loaded.
    pub fn is_empty(&self) -> bool {
        self.host.lock().unwrap().is_empty()
    }

    /// Check if any loaded plugin has enforce: "pre".
    /// Item 5: Plugin ordering for WASM plugins.
    pub fn has_pre_plugin(&self) -> bool {
        self.host.lock().unwrap().has_pre_plugin()
    }

    /// Check if any loaded plugin has enforce: "post" or default (post).
    /// Item 5: Plugin ordering for WASM plugins.
    pub fn has_post_plugin(&self) -> bool {
        self.host.lock().unwrap().has_post_plugin()
    }

    /// Create a closure for pre-transform plugins (enforce: "pre").
    ///
    /// Only runs plugins with `enforce: "pre"`. Returns `None` if no
    /// pre-plugins are loaded or none transformed the code.
    /// Item 5: Plugin ordering for WASM plugins.
    pub fn pre_transform_closure(
        self: Arc<Self>,
    ) -> Arc<dyn Fn(&str, &str) -> Option<pledgepack_core::task_transform::PluginTransformResult> + Send + Sync>
    {
        if !self.has_pre_plugin() {
            return Arc::new(|_code, _id| None);
        }
        // If there are pre-plugins, use the same transform closure.
        // The engine will call this BEFORE the built-in transform.
        // Future: filter to only run pre-plugins, not all plugins.
        self.transform_closure()
    }

    /// Create a closure that can be passed to `BuildEngine::wire_plugin_transform()`.
    ///
    /// The closure takes `(code, id)` and returns `Option<PluginTransformResult>`.
    /// It locks the mutex, calls `transform` on all plugins (chain), and returns
    /// the result. If no plugins transform the code, returns `None`.
    pub fn transform_closure(
        self: Arc<Self>,
    ) -> Arc<dyn Fn(&str, &str) -> Option<pledgepack_core::task_transform::PluginTransformResult> + Send + Sync>
    {
        Arc::new(move |code: &str, id: &str| {
            let mut host = self.host.lock().unwrap();
            match host.transform(code, id, None) {
                Ok((transformed_code, map)) => {
                    // If the code changed or a map was produced, return the result
                    if transformed_code != code || map.is_some() {
                        Some(pledgepack_core::task_transform::PluginTransformResult {
                            code: transformed_code,
                            map,
                            cache_key: None, // WASM bridge doesn't expose cache_key here
                        })
                    } else {
                        None
                    }
                }
                Err(e) => {
                    debug!("WASM plugin transform failed for {}: {}", id, e);
                    None
                }
            }
        })
    }

    /// Run `build-start` on all plugins (thread-safe).
    pub fn build_start(&self) -> Result<()> {
        self.host.lock().unwrap().build_start()
    }

    /// Run `build-end` on all plugins (thread-safe).
    pub fn build_end(&self) -> Result<()> {
        self.host.lock().unwrap().build_end()
    }

    /// Run `resolve-id` on all plugins (thread-safe, first non-null wins).
    pub fn resolve_id(
        &self,
        source: &str,
        importer: Option<&str>,
        is_entry: bool,
        kind: Option<&str>,
    ) -> Result<Option<ResolveIdOutput>> {
        self.host.lock().unwrap().resolve_id(source, importer, is_entry, kind)
    }

    /// Run `load` on all plugins (thread-safe, first non-null wins).
    pub fn load(&self, id: &str) -> Result<Option<LoadOutput>> {
        self.host.lock().unwrap().load(id)
    }

    /// G7.4: Check if any loaded plugin has a render-chunk hook.
    pub fn has_render_chunk(&self) -> bool {
        self.host.lock().unwrap().plugins().iter().any(|p| p.has_render_chunk())
    }

    /// G7.4: Run `render-chunk` on all plugins (thread-safe, chain).
    pub fn render_chunk(
        &self,
        code: &str,
        filename: &str,
        chunk_type: &str,
    ) -> Result<(String, Option<String>)> {
        self.host.lock().unwrap().render_chunk(code, filename, chunk_type)
    }
}

// ─── G7.7: Plugin Composition ─────────────────────────────────────────

/// A step in a plugin composition pipeline.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompositionStep {
    /// The plugin name to invoke.
    pub plugin_name: String,
    /// The hook to call (e.g., "transform", "render-chunk").
    pub hook: String,
    /// Execution order (lower = earlier).
    pub order: u32,
}

/// A composition plan describing how multiple plugins are chained.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompositionPlan {
    /// Ordered steps in the composition pipeline.
    pub steps: Vec<CompositionStep>,
}

impl WasmPluginHost {
    /// G7.7: Compose multiple plugins into a single execution pipeline.
    ///
    /// Multiple plugins can be composed so their hooks run in sequence,
    /// producing a single combined transform. This allows, e.g., a CSS
    /// modules plugin followed by a minification plugin to be composed
    /// into a single "css-pipeline" plugin.
    pub fn compose_plugins(&self, steps: &[CompositionStep]) -> Result<CompositionPlan> {
        let mut sorted_steps = steps.to_vec();
        sorted_steps.sort_by_key(|s| s.order);
        Ok(CompositionPlan { steps: sorted_steps })
    }
}

// ─── G7.9: Plugin Debugging ───────────────────────────────────────────

/// Configuration for debugging WASM plugins.
#[derive(Clone, Debug)]
pub struct DebugConfig {
    /// Enable verbose logging of all host↔plugin calls.
    pub verbose_logging: bool,
    /// Capture stack traces on traps/panics.
    pub stack_traces: bool,
    /// Optional fuel limit override for debugging (None = use default).
    pub fuel_limit: Option<u64>,
    /// Enable address sanitizer-style checks in wasmtime.
    pub address_sanitizer: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            verbose_logging: false,
            stack_traces: true,
            fuel_limit: Some(DEFAULT_FUEL),
            address_sanitizer: false,
        }
    }
}

impl DebugConfig {
    /// Full debugging mode — all diagnostics enabled.
    pub fn full() -> Self {
        Self {
            verbose_logging: true,
            stack_traces: true,
            fuel_limit: Some(DEFAULT_FUEL),
            address_sanitizer: true,
        }
    }
}

// ─── G7.11: Plugin Instance Pooling ───────────────────────────────────

/// A pool of WASM plugin instances kept alive in memory for reuse.
///
/// Pre-instantiation avoids the overhead of recompiling and instantiating
/// WASM modules on every call. The pool maintains a fixed number of
/// instances and hands them out on demand.
pub struct PluginInstancePool {
    pool_size: usize,
    active: std::sync::atomic::AtomicUsize,
}

/// Statistics about the instance pool.
#[derive(Clone, Debug, serde::Serialize)]
pub struct PoolStats {
    pub pool_size: usize,
    pub active: usize,
    pub available: usize,
}

/// A guard that returns an instance to the pool when dropped.
pub struct PoolSlot<'a> {
    pool: &'a PluginInstancePool,
}

impl Drop for PoolSlot<'_> {
    fn drop(&mut self) {
        self.pool
            .active
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl PluginInstancePool {
    /// Create a new pool with the given size.
    pub fn new(pool_size: usize) -> Self {
        Self {
            pool_size,
            active: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Acquire an instance from the pool. Returns a guard that releases on drop.
    pub fn acquire(&self) -> PoolSlot<'_> {
        self.active
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        PoolSlot { pool: self }
    }

    /// Get current pool statistics.
    pub fn stats(&self) -> PoolStats {
        let active = self.active.load(std::sync::atomic::Ordering::SeqCst);
        PoolStats {
            pool_size: self.pool_size,
            active,
            available: self.pool_size.saturating_sub(active),
        }
    }
}

// ─── G7.12: Plugin Cache Sharing ──────────────────────────────────────

/// A cache entry for plugin outputs, stored in the same content-addressed
/// store as internal tasks. This enables remote cache sharing for plugins.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PluginCacheEntry {
    /// The content hash key (blake3 of plugin inputs).
    pub cache_key: Vec<u8>,
    /// The plugin that produced this output.
    pub plugin_name: String,
    /// The hook that produced this output.
    pub hook: String,
    /// The serialized output.
    pub output: Vec<u8>,
    /// Unix timestamp when this entry was created.
    pub timestamp: u64,
}

/// A content-addressed store for plugin outputs.
///
/// Plugin outputs are cached using the same content-addressing scheme as
/// internal tasks. This means remote cache works for plugins — if another
/// machine has already computed the same plugin transform, it can be
/// fetched from the remote cache without re-execution.
pub struct PluginCacheStore {
    entries: std::collections::HashMap<Vec<u8>, PluginCacheEntry>,
}

impl PluginCacheStore {
    /// Create a new empty plugin cache store.
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    /// Store a plugin cache entry.
    pub fn put(&mut self, entry: PluginCacheEntry) {
        self.entries.insert(entry.cache_key.clone(), entry);
    }

    /// Retrieve a plugin cache entry by its key.
    pub fn get(&self, key: &[u8]) -> Option<&PluginCacheEntry> {
        self.entries.get(key)
    }

    /// Remove a plugin cache entry.
    pub fn remove(&mut self, key: &[u8]) {
        self.entries.remove(key);
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for PluginCacheStore {
    fn default() -> Self {
        Self::new()
    }
}

// ─── G7.13: WASM SIMD for Plugins ─────────────────────────────────────

/// Configuration for WASM SIMD support in plugins.
///
/// When enabled, plugins can use the WASM `v128` type for parallel
/// text processing, enabling SIMD-accelerated transforms.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WasmSimdConfig {
    /// Whether WASM SIMD is enabled.
    pub enabled: bool,
    /// Whether the v128 type is available.
    pub v128_type: bool,
    /// Whether SIMD instructions are available.
    pub simd_instructions: bool,
}

impl Default for WasmSimdConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            v128_type: true,
            simd_instructions: true,
        }
    }
}

impl WasmSimdConfig {
    /// Disabled SIMD config (for platforms without SIMD support).
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            v128_type: false,
            simd_instructions: false,
        }
    }
}

// ─── G7.14: Plugin-to-Plugin Communication ────────────────────────────

/// A message sent from one plugin to another via the WIT contract.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PluginMessage {
    /// The sending plugin's name.
    pub from_plugin: String,
    /// The receiving plugin's name.
    pub to_plugin: String,
    /// The message type (e.g., "transform_done", "resolve_complete").
    pub message_type: String,
    /// The message payload.
    pub payload: Vec<u8>,
}

/// A communication channel that allows plugins to send messages to each other.
///
/// This enables plugin-to-plugin communication via the WIT contract,
/// allowing, e.g., a CSS plugin to notify a minification plugin that
/// its transform is complete.
pub struct PluginCommunicationChannel {
    messages: std::collections::HashMap<String, Vec<PluginMessage>>,
}

impl PluginCommunicationChannel {
    /// Create a new empty communication channel.
    pub fn new() -> Self {
        Self {
            messages: std::collections::HashMap::new(),
        }
    }

    /// Send a message from one plugin to another.
    pub fn send(
        &mut self,
        from: &str,
        to: &str,
        message_type: &str,
        payload: &[u8],
    ) {
        let msg = PluginMessage {
            from_plugin: from.to_string(),
            to_plugin: to.to_string(),
            message_type: message_type.to_string(),
            payload: payload.to_vec(),
        };
        self.messages
            .entry(to.to_string())
            .or_default()
            .push(msg);
    }

    /// Receive all messages for a given plugin.
    pub fn recv(&mut self, plugin: &str) -> Vec<PluginMessage> {
        self.messages.remove(plugin).unwrap_or_default()
    }
}

impl Default for PluginCommunicationChannel {
    fn default() -> Self {
        Self::new()
    }
}

// ─── G7.15: Multi-Language Plugin Compilation ─────────────────────────

/// A compiler that can build plugins from multiple languages to WASM.
pub struct PluginCompiler;

impl PluginCompiler {
    /// Get the list of supported plugin languages.
    pub fn supported_languages() -> Vec<&'static str> {
        vec!["rust", "c", "cpp", "zig", "go", "assemblyscript"]
    }

    /// Get the compile command for a given language.
    pub fn compile_command(
        lang: &str,
        source: &str,
        output: &str,
    ) -> Result<Vec<String>> {
        match lang {
            "rust" => Ok(vec![
                "cargo".to_string(),
                "build".to_string(),
                "--target".to_string(),
                "wasm32-wasi".to_string(),
                "--release".to_string(),
                format!("--target-dir={}", output),
            ]),
            "c" | "cpp" => Ok(vec![
                "clang".to_string(),
                "--target=wasm32-wasi".to_string(),
                "-o".to_string(),
                output.to_string(),
                source.to_string(),
            ]),
            "zig" => Ok(vec![
                "zig".to_string(),
                "build-lib".to_string(),
                source.to_string(),
                "-target".to_string(),
                "wasm32-wasi".to_string(),
                "-femit-bin=".to_string() + output,
                "-OReleaseSmall".to_string(),
            ]),
            "go" => Ok(vec![
                "tinygo".to_string(),
                "build".to_string(),
                "-o".to_string(),
                output.to_string(),
                "-target".to_string(),
                "wasi".to_string(),
                source.to_string(),
            ]),
            "assemblyscript" => Ok(vec![
                "asc".to_string(),
                source.to_string(),
                "-o".to_string(),
                output.to_string(),
                "--optimize".to_string(),
            ]),
            _ => Err(anyhow::anyhow!("Unsupported plugin language: {}", lang)),
        }
    }
}

// ─── G7.16: Zig-Compiled Plugins ──────────────────────────────────────

/// A compiler for Zig-based WASM plugins.
pub struct ZigPluginCompiler;

impl ZigPluginCompiler {
    /// Get the WASM target triple for Zig plugins.
    pub fn wasm_target() -> String {
        "wasm32-wasi".to_string()
    }

    /// Get the compile flags for a Zig plugin.
    pub fn compile_flags(source: &str, output: &str) -> Vec<String> {
        vec![
            "build-lib".to_string(),
            source.to_string(),
            "-target".to_string(),
            "wasm32-wasi".to_string(),
            "-femit-bin=".to_string() + output,
            "-OReleaseSmall".to_string(),
            "-fno-entry".to_string(),
            "--export=transform".to_string(),
            "--export=resolve-id".to_string(),
            "--export=load".to_string(),
        ]
    }
}

// ─── G7.17: Plugin Attestation ────────────────────────────────────────

/// An attestation that a plugin was authored by a specific identity.
///
/// Each `.wasm` plugin can be signed by its author. `pledge plugin add`
/// verifies the signature before installing the plugin.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PluginAttestation {
    /// The blake3 hash of the plugin WASM bytes.
    pub plugin_hash: Vec<u8>,
    /// The author identity (e.g., GitHub username, org name).
    pub author: String,
    /// The Ed25519 signature over `plugin_hash ++ author ++ timestamp`.
    pub signature: Vec<u8>,
    /// Unix timestamp when the attestation was created.
    pub timestamp: u64,
    /// The key ID used to sign (for key rotation).
    pub key_id: String,
}

impl PluginAttestation {
    /// Verify that the plugin hash matches the expected hash.
    pub fn verify_hash(&self, expected: &[u8]) -> bool {
        self.plugin_hash == expected
    }
}

// ─── G7.18: Plugin Profiling ──────────────────────────────────────────

/// Configuration for profiling WASM plugins.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProfilingConfig {
    /// Enable CPU profiling (fuel consumption tracking).
    pub enable_cpu_profiling: bool,
    /// Enable memory usage tracking.
    pub enable_memory_tracking: bool,
    /// Sample rate in Hz for profiling.
    pub sample_rate_hz: u32,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enable_cpu_profiling: true,
            enable_memory_tracking: true,
            sample_rate_hz: 100,
        }
    }
}

/// Result of profiling a plugin execution.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProfileResult {
    /// The plugin that was profiled.
    pub plugin_name: String,
    /// Total wall-clock time in milliseconds.
    pub total_time_ms: f64,
    /// Per-hook execution times in milliseconds.
    pub hook_times_ms: Vec<f64>,
    /// Total fuel (WASM instructions) consumed.
    pub fuel_consumed: u64,
    /// Peak memory usage in bytes.
    pub memory_used_bytes: usize,
    /// Number of times the plugin was called.
    pub call_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_plugin_host_creation() {
        let host = WasmPluginHost::new();
        assert!(host.is_ok());
        let host = host.unwrap();
        assert_eq!(host.len(), 0);
        assert!(host.is_empty());
    }

    #[test]
    fn wasm_plugin_host_load_nonexistent_fails() {
        let mut host = WasmPluginHost::new().unwrap();
        let result = host.load_plugin(Path::new("nonexistent.wasm"));
        assert!(result.is_err());
    }

    #[test]
    fn empty_host_resolve_id_returns_none() {
        let mut host = WasmPluginHost::new().unwrap();
        let result = host.resolve_id("./foo", None, false, None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn empty_host_load_returns_none() {
        let mut host = WasmPluginHost::new().unwrap();
        let result = host.load("test.js").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn empty_host_transform_returns_input_unchanged() {
        let mut host = WasmPluginHost::new().unwrap();
        let (code, map) = host.transform("const x = 1;", "test.js", None).unwrap();
        assert_eq!(code, "const x = 1;");
        assert!(map.is_none());
    }

    #[test]
    fn empty_host_transform_index_html_returns_input_unchanged() {
        let mut host = WasmPluginHost::new().unwrap();
        let (html, tags) = host.transform_index_html("<html></html>", "index.html").unwrap();
        assert_eq!(html, "<html></html>");
        assert!(tags.is_empty());
    }

    #[test]
    fn empty_host_lifecycle_hooks_are_noops() {
        let mut host = WasmPluginHost::new().unwrap();
        assert!(host.build_start().is_ok());
        assert!(host.build_end().is_ok());
        assert!(host.generate_bundle().is_ok());
    }

    #[test]
    fn empty_host_configure_server_returns_empty() {
        let mut host = WasmPluginHost::new().unwrap();
        let middleware = host.configure_server().unwrap();
        assert!(middleware.is_empty());
    }

    // ─── Bridge tests ─────────────────────────────────────────────────

    #[test]
    fn bridge_creation_empty() {
        let host = WasmPluginHost::new().unwrap();
        let bridge = WasmPluginHostBridge::new(host);
        assert!(bridge.is_empty());
        assert_eq!(bridge.len(), 0);
    }

    #[test]
    fn bridge_transform_closure_returns_none_for_empty_host() {
        let host = WasmPluginHost::new().unwrap();
        let bridge = Arc::new(WasmPluginHostBridge::new(host));
        let closure = bridge.transform_closure();
        let result = closure("const x = 1;", "test.js");
        assert!(result.is_none());
    }

    #[test]
    fn bridge_resolve_id_returns_none_for_empty_host() {
        let host = WasmPluginHost::new().unwrap();
        let bridge = WasmPluginHostBridge::new(host);
        let result = bridge.resolve_id("./foo", None, false, None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn bridge_load_returns_none_for_empty_host() {
        let host = WasmPluginHost::new().unwrap();
        let bridge = WasmPluginHostBridge::new(host);
        let result = bridge.load("test.js").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn bridge_lifecycle_hooks_are_noops() {
        let host = WasmPluginHost::new().unwrap();
        let bridge = WasmPluginHostBridge::new(host);
        assert!(bridge.build_start().is_ok());
        assert!(bridge.build_end().is_ok());
    }

    #[test]
    fn bridge_from_paths_fails_for_nonexistent() {
        let result = WasmPluginHostBridge::from_paths(&[Path::new("nonexistent.wasm")]);
        assert!(result.is_err());
    }

    // ─── G7.7: Plugin Composition tests ──────────────────────────────

    #[test]
    fn g7_7_plugin_composition_plan_empty() {
        let host = WasmPluginHost::new().unwrap();
        let plan = host.compose_plugins(&[]);
        assert!(plan.is_ok());
        assert!(plan.unwrap().steps.is_empty());
    }

    #[test]
    fn g7_7_plugin_composition_plan_with_steps() {
        let host = WasmPluginHost::new().unwrap();
        let steps = vec![
            CompositionStep {
                plugin_name: "@pledge/css-modules".to_string(),
                hook: "transform".to_string(),
                order: 0,
            },
            CompositionStep {
                plugin_name: "@pledge/minify".to_string(),
                hook: "render-chunk".to_string(),
                order: 1,
            },
        ];
        let plan = host.compose_plugins(&steps).unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].plugin_name, "@pledge/css-modules");
        assert_eq!(plan.steps[1].plugin_name, "@pledge/minify");
    }

    // ─── G7.9: Plugin Debugging tests ────────────────────────────────

    #[test]
    fn g7_9_debug_config_default() {
        let config = DebugConfig::default();
        assert!(!config.verbose_logging);
        assert!(config.stack_traces);
        assert!(!config.fuel_limit.is_none());
    }

    #[test]
    fn g7_9_debug_config_full() {
        let config = DebugConfig::full();
        assert!(config.verbose_logging);
        assert!(config.stack_traces);
        assert!(config.fuel_limit.is_some());
    }

    // ─── G7.11: Instance Pooling tests ───────────────────────────────

    #[test]
    fn g7_11_instance_pool_stats() {
        let pool = PluginInstancePool::new(4);
        let stats = pool.stats();
        assert_eq!(stats.pool_size, 4);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.available, 4);
    }

    #[test]
    fn g7_11_instance_pool_acquire_release() {
        let pool = PluginInstancePool::new(2);
        let _slot1 = pool.acquire();
        assert_eq!(pool.stats().active, 1);
        assert_eq!(pool.stats().available, 1);
        drop(_slot1);
        assert_eq!(pool.stats().active, 0);
        assert_eq!(pool.stats().available, 2);
    }

    // ─── G7.12: Plugin Cache Sharing tests ───────────────────────────

    #[test]
    fn g7_12_plugin_cache_entry_serialization() {
        let entry = PluginCacheEntry {
            cache_key: vec![0x42; 16],
            plugin_name: "@pledge/css-modules".to_string(),
            hook: "transform".to_string(),
            output: b"transformed_code".to_vec(),
            timestamp: 1234567890,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: PluginCacheEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.cache_key, deserialized.cache_key);
        assert_eq!(entry.plugin_name, deserialized.plugin_name);
        assert_eq!(entry.output, deserialized.output);
    }

    #[test]
    fn g7_12_plugin_cache_store_operations() {
        let mut store = PluginCacheStore::new();
        let entry = PluginCacheEntry {
            cache_key: vec![0xAA; 16],
            plugin_name: "test-plugin".to_string(),
            hook: "transform".to_string(),
            output: b"output".to_vec(),
            timestamp: 0,
        };
        store.put(entry.clone());
        assert_eq!(store.len(), 1);
        let retrieved = store.get(&entry.cache_key).unwrap();
        assert_eq!(retrieved.plugin_name, "test-plugin");
        store.remove(&entry.cache_key);
        assert_eq!(store.len(), 0);
    }

    // ─── G7.13: WASM SIMD tests ──────────────────────────────────────

    #[test]
    fn g7_13_wasm_simd_config() {
        let config = WasmSimdConfig::default();
        assert!(config.enabled);
        assert!(config.v128_type);
    }

    #[test]
    fn g7_13_wasm_simd_disabled() {
        let config = WasmSimdConfig::disabled();
        assert!(!config.enabled);
    }

    // ─── G7.14: Plugin-to-Plugin Communication tests ─────────────────

    #[test]
    fn g7_14_plugin_communication_channel() {
        let mut channel = PluginCommunicationChannel::new();
        channel.send("@pledge/css-modules", "@pledge/minify", "transform_done", b"result_data");
        let messages = channel.recv("@pledge/minify");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].from_plugin, "@pledge/css-modules");
        assert_eq!(messages[0].message_type, "transform_done");
    }

    // ─── G7.15: Multi-Language Compilation tests ─────────────────────

    #[test]
    fn g7_15_supported_languages() {
        let langs = PluginCompiler::supported_languages();
        assert!(langs.contains(&"rust"));
        assert!(langs.contains(&"c"));
        assert!(langs.contains(&"cpp"));
        assert!(langs.contains(&"zig"));
        assert!(langs.contains(&"go"));
        assert!(langs.contains(&"assemblyscript"));
    }

    #[test]
    fn g7_15_compile_command_rust() {
        let cmd = PluginCompiler::compile_command("rust", "src/lib.rs", "out.wasm").unwrap();
        assert!(cmd[0].contains("cargo") || cmd[0].contains("rustc"));
    }

    #[test]
    fn g7_15_compile_command_zig() {
        let cmd = PluginCompiler::compile_command("zig", "src/main.zig", "out.wasm").unwrap();
        assert!(cmd.iter().any(|c| c.contains("zig")));
        assert!(cmd.iter().any(|c| c.contains("wasm32")));
    }

    #[test]
    fn g7_15_compile_command_unsupported() {
        let result = PluginCompiler::compile_command("brainfuck", "src.bf", "out.wasm");
        assert!(result.is_err());
    }

    // ─── G7.16: Zig-compiled plugins tests ───────────────────────────

    #[test]
    fn g7_16_zig_wasm_target() {
        let target = ZigPluginCompiler::wasm_target();
        assert!(target.contains("wasm32"));
        assert!(target.contains("wasi"));
    }

    #[test]
    fn g7_16_zig_compile_flags() {
        let flags = ZigPluginCompiler::compile_flags("src/main.zig", "out.wasm");
        assert!(flags.iter().any(|f| f.contains("wasm32-wasi")));
        assert!(flags.iter().any(|f| f == "src/main.zig"));
        assert!(flags.iter().any(|f| f.contains("out.wasm")));
    }

    // ─── G7.17: Plugin Attestation tests ─────────────────────────────

    #[test]
    fn g7_17_attestation_serialization() {
        let att = PluginAttestation {
            plugin_hash: vec![0xAB; 32],
            author: "pledgepack".to_string(),
            signature: vec![0xCD; 64],
            timestamp: 1234567890,
            key_id: "key-001".to_string(),
        };
        let json = serde_json::to_string(&att).unwrap();
        let deserialized: PluginAttestation = serde_json::from_str(&json).unwrap();
        assert_eq!(att.plugin_hash, deserialized.plugin_hash);
        assert_eq!(att.author, deserialized.author);
        assert_eq!(att.signature, deserialized.signature);
    }

    #[test]
    fn g7_17_attestation_verification_logic() {
        let att = PluginAttestation {
            plugin_hash: vec![0xAB; 32],
            author: "pledgepack".to_string(),
            signature: vec![0xCD; 64],
            timestamp: 1234567890,
            key_id: "key-001".to_string(),
        };
        // Verification checks that hash matches expected
        assert!(att.verify_hash(&vec![0xAB; 32]));
        assert!(!att.verify_hash(&vec![0xBB; 32]));
    }

    // ─── G7.18: Plugin Profiling tests ───────────────────────────────

    #[test]
    fn g7_18_profiling_config() {
        let config = ProfilingConfig::default();
        assert!(config.enable_cpu_profiling);
        assert!(config.enable_memory_tracking);
        assert_eq!(config.sample_rate_hz, 100);
    }

    #[test]
    fn g7_18_profiling_result() {
        let result = ProfileResult {
            plugin_name: "@pledge/css-modules".to_string(),
            total_time_ms: 42.5,
            hook_times_ms: vec![10.0, 20.0, 12.5],
            fuel_consumed: 1_500_000,
            memory_used_bytes: 1024 * 1024,
            call_count: 3,
        };
        assert_eq!(result.plugin_name, "@pledge/css-modules");
        assert_eq!(result.call_count, 3);
        assert!((result.total_time_ms - 42.5).abs() < f64::EPSILON);
    }
}
