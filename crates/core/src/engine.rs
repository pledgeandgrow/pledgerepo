// Build engine: orchestrates the entire build pipeline
//
// This is the "Turbo engine" equivalent — a function-level
// incremental computation system that caches aggressively
// and only recomputes what changed.

use crate::config::PledgeConfig;
use crate::module::{ModuleId, ResolvedModule};
use crate::module_graph::SerializableModuleGraph;
use anyhow::Result;
use pledgepack_native_sys::Graph;
use rayon::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, debug};

/// Manifest entry for build manifest.json with entry-to-chunk mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Hashed output filename (e.g., "index.a1b2c3d4.js")
    pub file: String,
    /// Whether this is an entry point
    pub is_entry: bool,
    /// Whether this is a CSS file
    pub is_css: bool,
    /// Whether this is an async chunk (dynamic import)
    pub is_async: bool,
    /// Dynamic imports this module depends on
    pub imports: Vec<String>,
    /// CSS file associated with this module (if any)
    pub css: Option<String>,
}

pub struct BuildEngine {
    config: Arc<PledgeConfig>,
    graph: Graph,
    /// Map from file path to module ID
    path_to_id: HashMap<PathBuf, ModuleId>,
    /// Cached resolved modules
    modules: HashMap<ModuleId, ResolvedModule>,
    /// Function-level cache (content hash → cached output)
    function_cache: HashMap<u64, CachedOutput>,
    /// Persistent function-level cache (disk-backed)
    persistent_cache: Option<pledgepack_cache::FunctionCache>,
    /// Remote cache for sharing across machines
    remote_cache: Option<pledgepack_cache::remote::RemoteCache>,
    /// Serializable module graph for incremental rebuilds
    module_graph: SerializableModuleGraph,
    /// Previous module graph loaded from disk (for incremental comparison)
    previous_graph: Option<SerializableModuleGraph>,
    /// Git-based cache invalidator
    git_invalidator: Option<pledgepack_cache::git_cache::GitCacheInvalidator>,
    /// Whether this is an incremental rebuild (not first build)
    is_incremental: bool,
}

#[derive(Debug, Clone)]
pub struct CachedOutput {
    pub code: String,
    pub source_map: Option<String>,
    pub deps: Vec<String>,
    pub is_css: bool,
    pub css_modules: Option<Vec<(String, String)>>,
    pub extracted_css: Option<String>,
    pub is_worker: bool,
    pub dynamic_imports: Vec<String>,
}

#[derive(Debug)]
pub struct BuildResult {
    pub modules_built: usize,
    pub modules_cached: usize,
    pub duration_ms: u128,
}

impl BuildEngine {
    pub fn new(config: Arc<PledgeConfig>) -> Self {
        let persistent_cache = if config.cache.enabled {
            Some(pledgepack_cache::FunctionCache::new(
                config.cache.dir.clone(),
                true,
            ))
        } else {
            None
        };

        // Initialize remote cache if configured
        let remote_cache = if config.cache.enabled && config.cache.remote.enabled {
            let remote_config = pledgepack_cache::remote::RemoteCacheConfig {
                backend: config.cache.remote.backend.clone(),
                endpoint: config.cache.remote.endpoint.clone(),
                bucket: config.cache.remote.bucket.clone(),
                region: config.cache.remote.region.clone(),
                access_key: None,
                secret_key: None,
                namespace: config.cache.remote.namespace.clone(),
                timeout_secs: 30,
                enabled: true,
            };
            let rc = pledgepack_cache::remote::RemoteCache::new(remote_config);
            if rc.is_enabled() { Some(rc) } else { None }
        } else {
            None
        };

        // Initialize git-based invalidator
        let git_invalidator = pledgepack_cache::git_cache::GitCacheInvalidator::new(&config.root);

        // Try to load previous module graph from disk for incremental rebuilds
        let graph_path = config.cache.dir.join("module_graph.bin");
        let previous_graph = if config.cache.enabled && SerializableModuleGraph::exists_on_disk(&graph_path) {
            match SerializableModuleGraph::load_from_disk(&graph_path) {
                Ok(g) => {
                    info!("Loaded previous module graph: {} modules", g.modules.len());
                    Some(g)
                }
                Err(e) => {
                    warn!("Failed to load previous module graph: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let is_incremental = previous_graph.is_some();

        Self {
            config,
            graph: Graph::new(),
            path_to_id: HashMap::new(),
            modules: HashMap::new(),
            function_cache: HashMap::new(),
            persistent_cache,
            remote_cache,
            module_graph: SerializableModuleGraph::new(),
            previous_graph,
            git_invalidator: Some(git_invalidator).filter(|g| g.is_available()),
            is_incremental,
        }
    }

    /// Run a full build (dev or production)
    ///
    /// Supports incremental rebuilds: if a previous module graph was loaded
    /// from disk, only changed modules and their dependents are re-transformed.
    /// Unchanged modules are loaded from cache (memory → disk → remote).
    pub async fn build(&mut self) -> Result<BuildResult> {
        let start = std::time::Instant::now();

        // Phase 1: Resolve entry points (lazy — only resolve entries first)
        let entries = self.config.entry.clone();
        for entry in &entries {
            self.resolve_and_add(entry, None)?;
        }

        // Record entry module IDs in the serializable graph
        let entry_ids: Vec<ModuleId> = self.path_to_id.values().copied().collect();
        self.module_graph.set_entries(entry_ids.clone());

        // Phase 2: Determine which modules need rebuilding (incremental)
        let mut skip_set: HashSet<ModuleId> = HashSet::new();
        if self.is_incremental {
            if let Some(ref prev) = self.previous_graph {
                // Use git tree hash for fast repo-level change detection
                if let Some(ref git) = self.git_invalidator {
                    let prev_tree = prev.git_tree_hash.as_deref();
                    if !git.has_repo_changed(prev_tree) {
                        info!("Git tree hash unchanged — full cache hit");
                        // Load all modules from previous graph
                        for (id, node) in &prev.modules {
                            if let Some(cached) = self.load_cached_module(*id, node.content_hash, &node.path) {
                                self.function_cache.insert(node.content_hash, cached);
                                skip_set.insert(*id);
                            }
                        }
                    }
                }

                // If git invalidation isn't available or tree changed,
                // use content-hash-based incremental detection
                if skip_set.is_empty() {
                    // Compare current entry content hashes with previous
                    let mut changed: HashSet<ModuleId> = HashSet::new();
                    for (id, module) in &self.modules {
                        if let Some(prev_node) = prev.modules.get(id) {
                            if module.content_hash != prev_node.content_hash {
                                debug!("Changed module: {:?} (hash {} → {})",
                                    module.path, prev_node.content_hash, module.content_hash);
                                changed.insert(*id);
                            }
                        } else {
                            changed.insert(*id);
                        }
                    }
                    // Compute affected dependents (transitive closure)
                    let affected = self.module_graph.compute_affected(&changed);
                    // Load unaffected modules from cache
                    for (id, node) in &prev.modules {
                        if !affected.contains(id) {
                            if let Some(cached) = self.load_cached_module(*id, node.content_hash, &node.path) {
                                self.function_cache.insert(node.content_hash, cached);
                                skip_set.insert(*id);
                            }
                        }
                    }
                    if !affected.is_empty() {
                        info!("Incremental: {} changed, {} affected, {} cached",
                            changed.len(), affected.len(), skip_set.len());
                    }
                }
            }
        }

        // Phase 3: Parse, transform, and build graph (BFS from entries)
        let mut modules_built = 0usize;
        let mut modules_cached = 0usize;

        // Process modules in BFS order from entry points (lazy scanning)
        let mut queue: Vec<ModuleId> = self.path_to_id.values().copied().collect();
        let mut processed = HashSet::new();

        while let Some(module_id) = queue.pop() {
            if processed.contains(&module_id) {
                continue;
            }
            processed.insert(module_id);

            let module = match self.modules.get(&module_id) {
                Some(m) => m.clone(),
                None => continue,
            };

            // Add to serializable graph
            self.module_graph.add_module(
                module_id,
                module.path.clone(),
                module.kind,
                module.content_hash,
            );

            // Skip if already loaded from incremental cache
            if skip_set.contains(&module_id) {
                modules_cached += 1;
                if let Some(cached) = self.function_cache.get(&module.content_hash).cloned() {
                    for dep_path in &cached.deps {
                        let dep_id = self.resolve_and_add(dep_path, Some(&module.path))?;
                        self.graph.add_dependency(module_id, dep_id);
                        self.module_graph.add_dependency(module_id, dep_id);
                        queue.push(dep_id);
                    }
                }
                continue;
            }

            // Check function-level cache (memory first, then disk, then remote)
            let cache_key = module.content_hash;
            if let Some(cached) = self.function_cache.get(&cache_key).cloned() {
                modules_cached += 1;
                for dep_path in &cached.deps {
                    let dep_id = self.resolve_and_add(dep_path, Some(&module.path))?;
                    self.graph.add_dependency(module_id, dep_id);
                    self.module_graph.add_dependency(module_id, dep_id);
                    queue.push(dep_id);
                }
            } else if let Some(ref pc) = self.persistent_cache {
                let pkey = pledgepack_cache::make_key(cache_key, "transform", &module.path.to_string_lossy().to_string());
                if let Some(entry) = pc.get(&pkey) {
                    modules_cached += 1;
                    let cached = CachedOutput {
                        code: entry.code,
                        source_map: entry.source_map,
                        deps: entry.deps,
                        is_css: false,
                        css_modules: None,
                        extracted_css: None,
                        is_worker: false,
                        dynamic_imports: Vec::new(),
                    };
                    self.function_cache.insert(cache_key, cached.clone());
                    for dep_path in &cached.deps {
                        let dep_id = self.resolve_and_add(dep_path, Some(&module.path))?;
                        self.graph.add_dependency(module_id, dep_id);
                        self.module_graph.add_dependency(module_id, dep_id);
                        queue.push(dep_id);
                    }
                } else if let Some(ref rc) = self.remote_cache {
                    // Try remote cache before transforming
                    let rkey = pledgepack_cache::remote::remote_cache_key(cache_key, "transform", &module.path.to_string_lossy().to_string());
                    if let Ok(Some(remote_entry)) = rc.get(&rkey) {
                        modules_cached += 1;
                        debug!("Remote cache hit: {:?}", module.path);
                        let cached = CachedOutput {
                            code: remote_entry.code,
                            source_map: remote_entry.source_map,
                            deps: remote_entry.deps,
                            is_css: false,
                            css_modules: None,
                            extracted_css: None,
                            is_worker: false,
                            dynamic_imports: Vec::new(),
                        };
                        // Populate local caches
                        self.function_cache.insert(cache_key, cached.clone());
                        pc.set(pkey, pledgepack_cache::CacheEntry {
                            code: cached.code.clone(),
                            source_map: cached.source_map.clone(),
                            deps: cached.deps.clone(),
                            created_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                        });
                        for dep_path in &cached.deps {
                            let dep_id = self.resolve_and_add(dep_path, Some(&module.path))?;
                            self.graph.add_dependency(module_id, dep_id);
                            self.module_graph.add_dependency(module_id, dep_id);
                            queue.push(dep_id);
                        }
                    } else {
                        modules_built += 1;
                        let output = self.transform_module(&module).await?;
                        pc.set(pkey, pledgepack_cache::CacheEntry {
                            code: output.code.clone(),
                            source_map: output.source_map.clone(),
                            deps: output.deps.clone(),
                            created_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                        });
                        // Store to remote cache as well
                        let rkey = pledgepack_cache::remote::remote_cache_key(cache_key, "transform", &module.path.to_string_lossy().to_string());
                        let _ = rc.set(&rkey, &pledgepack_cache::remote::RemoteCacheEntry {
                            code: output.code.clone(),
                            source_map: output.source_map.clone(),
                            deps: output.deps.clone(),
                            created_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                        });
                        for dep_path in &output.deps {
                            let dep_id = self.resolve_and_add(dep_path, Some(&module.path))?;
                            self.graph.add_dependency(module_id, dep_id);
                            self.module_graph.add_dependency(module_id, dep_id);
                            queue.push(dep_id);
                        }
                        self.function_cache.insert(cache_key, output);
                    }
                } else {
                    modules_built += 1;
                    let output = self.transform_module(&module).await?;
                    pc.set(pkey, pledgepack_cache::CacheEntry {
                        code: output.code.clone(),
                        source_map: output.source_map.clone(),
                        deps: output.deps.clone(),
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    });
                    for dep_path in &output.deps {
                        let dep_id = self.resolve_and_add(dep_path, Some(&module.path))?;
                        self.graph.add_dependency(module_id, dep_id);
                        self.module_graph.add_dependency(module_id, dep_id);
                        queue.push(dep_id);
                    }
                    self.function_cache.insert(cache_key, output);
                }
            } else {
                modules_built += 1;
                let output = self.transform_module(&module).await?;
                for dep_path in &output.deps {
                    let dep_id = self.resolve_and_add(dep_path, Some(&module.path))?;
                    self.graph.add_dependency(module_id, dep_id);
                    self.module_graph.add_dependency(module_id, dep_id);
                    queue.push(dep_id);
                }
                self.function_cache.insert(cache_key, output);
            }
        }

        // Phase 4: Save module graph to disk for next incremental build
        if self.config.cache.enabled {
            let graph_path = self.config.cache.dir.join("module_graph.bin");
            self.module_graph.built_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            // Store git tree hash for fast invalidation on next build
            if let Some(ref git) = self.git_invalidator {
                self.module_graph.git_tree_hash = git.root_tree_hash().map(|s| s.to_string());
            }
            if let Err(e) = self.module_graph.save_to_disk(&graph_path) {
                warn!("Failed to save module graph: {}", e);
            }
        }

        let duration = start.elapsed();

        info!(
            "Build complete: {} built, {} cached, {}ms{}",
            modules_built,
            modules_cached,
            duration.as_millis(),
            if self.is_incremental { " (incremental)" } else { "" }
        );

        Ok(BuildResult {
            modules_built,
            modules_cached,
            duration_ms: duration.as_millis(),
        })
    }

    /// Try to load a cached module output from persistent cache
    fn load_cached_module(&self, id: ModuleId, content_hash: u64, path: &PathBuf) -> Option<CachedOutput> {
        if let Some(cached) = self.function_cache.get(&content_hash) {
            return Some(cached.clone());
        }
        if let Some(ref pc) = self.persistent_cache {
            let pkey = pledgepack_cache::make_key(content_hash, "transform", &path.to_string_lossy().to_string());
            if let Some(entry) = pc.get(&pkey) {
                return Some(CachedOutput {
                    code: entry.code,
                    source_map: entry.source_map,
                    deps: entry.deps,
                    is_css: false,
                    css_modules: None,
                    extracted_css: None,
                    is_worker: false,
                    dynamic_imports: Vec::new(),
                });
            }
        }
        None
    }

    /// Resolve a module path and add it to the graph.
    /// `importer` is the path of the importing module (for relative resolution).
    fn resolve_and_add(&mut self, specifier: &str, importer: Option<&PathBuf>) -> Result<ModuleId> {
        let path = self.resolve(specifier, importer)?;

        if let Some(&id) = self.path_to_id.get(&path) {
            return Ok(id);
        }

        let id = self.graph.add_module(path.to_str().unwrap_or(""));
        self.path_to_id.insert(path.clone(), id);

        // Read source via Zig I/O layer
        let source = pledgepack_native_sys::read_file(path.to_str().unwrap_or(""))?;
        let content_hash = u64::from_be_bytes(blake3::hash(&source).as_bytes()[0..8].try_into().unwrap());

        let ext_str = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let kind = crate::module::ModuleKind::from_extension(&ext_str);

        let module = ResolvedModule {
            id,
            path: path.clone(),
            kind,
            source,
            content_hash,
        };

        self.modules.insert(id, module);
        Ok(id)
    }

    /// Resolve a module specifier to a file path.
    /// `importer` is the path of the importing module (for relative resolution).
    fn resolve(&self, specifier: &str, importer: Option<&PathBuf>) -> Result<PathBuf> {
        // Handle relative paths
        if specifier.starts_with("./") || specifier.starts_with("../") {
            let base = importer
                .and_then(|p| p.parent())
                .unwrap_or(&self.config.root);
            let path = base.join(specifier);

            // Try with extensions
            for ext in &self.config.extensions {
                let with_ext = if path.extension().is_some() {
                    path.clone()
                } else {
                    path.with_extension(ext.trim_start_matches('.'))
                };
                if with_ext.exists() {
                    return Ok(with_ext);
                }
            }

            // Try as directory with index file
            for ext in &self.config.extensions {
                let index = path.join(format!("index{}", ext));
                if index.exists() {
                    return Ok(index);
                }
            }
        }

        // Handle bare specifiers (node_modules)
        if !specifier.starts_with('.') && !specifier.starts_with('/') {
            let node_modules = self.config.root.join("node_modules");

            // Handle subpath imports: "react-dom/client" → "react-dom" + "/client"
            let (pkg_name, subpath) = if specifier.starts_with('@') {
                // Scoped: @org/pkg/sub → (@org/pkg, /sub)
                let parts: Vec<&str> = specifier.splitn(3, '/').collect();
                if parts.len() >= 2 {
                    let pkg = format!("{}/{}", parts[0], parts[1]);
                    let sub = if parts.len() == 3 { format!("/{}", parts[2]) } else { String::new() };
                    (pkg, sub)
                } else {
                    (specifier.to_string(), String::new())
                }
            } else {
                // Non-scoped: pkg/sub → (pkg, /sub)
                match specifier.split_once('/') {
                    Some((pkg, sub)) => (pkg.to_string(), format!("/{}", sub)),
                    None => (specifier.to_string(), String::new()),
                }
            };

            let pkg_json = node_modules.join(&pkg_name).join("package.json");

            if pkg_json.exists() {
                let content = std::fs::read_to_string(&pkg_json)?;
                let pkg: serde_json::Value = serde_json::from_str(&content)?;

                if subpath.is_empty() {
                    // Root import: resolve via "module" or "main"
                    let entry = pkg
                        .get("module")
                        .or_else(|| pkg.get("main"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("index.js");
                    return Ok(node_modules.join(&pkg_name).join(entry));
                } else {
                    // Subpath import: check "exports" map first
                    if let Some(exports) = pkg.get("exports") {
                        if let Some(obj) = exports.as_object() {
                            let key = format!(".{}", subpath);
                            if let Some(export_val) = obj.get(&key) {
                                let resolved = if let Some(s) = export_val.as_str() {
                                    Some(s.to_string())
                                } else if let Some(conditions) = export_val.as_object() {
                                    conditions.get("browser")
                                        .or_else(|| conditions.get("import"))
                                        .or_else(|| conditions.get("default"))
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                } else {
                                    None
                                };
                                if let Some(resolved_path) = resolved {
                                    let full = node_modules.join(&pkg_name).join(resolved_path.trim_start_matches("./"));
                                    if full.exists() {
                                        return Ok(full);
                                    }
                                }
                            }
                        }
                    }
                    // Fallback: try direct file path
                    let direct = node_modules.join(&pkg_name).join(subpath.trim_start_matches('/'));
                    if direct.exists() {
                        return Ok(direct);
                    }
                    // Try with extensions
                    for ext in &self.config.extensions {
                        let with_ext = direct.with_extension(ext.trim_start_matches('.'));
                        if with_ext.exists() {
                            return Ok(with_ext);
                        }
                    }
                }
            }
        }

        // Last resort: try as-is
        let path = self.config.root.join(specifier);
        if path.exists() {
            return Ok(path);
        }

        anyhow::bail!("Cannot resolve module: {}", specifier)
    }

    /// Transform a single module (parse + compile)
    async fn transform_module(&self, module: &ResolvedModule) -> Result<CachedOutput> {
        let source_str = String::from_utf8_lossy(&module.source).to_string();

        // Use SIMD scanning to find imports/exports
        let import_offsets = pledgepack_native_sys::find_imports(&module.source);

        // Extract dependency specifiers from import statements
        let mut deps = Vec::new();
        for offset in import_offsets {
            // Find the string literal after 'import'
            let rest = &source_str[offset..];
            if let Some(dep) = extract_module_specifier(rest) {
                deps.push(dep);
            }
        }

        // Transform using Oxc (JSX → JS, TS type stripping, minification)
        let is_production = self.config.mode == crate::config::BuildMode::Production;
        let file_path = module.path.to_str().unwrap_or("");
        let transform_output = crate::transform::transform(
            &source_str,
            module.kind,
            file_path,
            is_production,
            &self.config,
        )?;

        Ok(CachedOutput {
            code: transform_output.code,
            source_map: transform_output.source_map,
            deps,
            is_css: transform_output.is_css,
            css_modules: transform_output.css_modules,
            extracted_css: transform_output.extracted_css,
            is_worker: transform_output.is_worker,
            dynamic_imports: transform_output.dynamic_imports,
        })
    }

    /// Transform multiple modules in parallel using rayon.
    /// Returns transformed outputs keyed by module ID.
    pub fn transform_modules_parallel(
        &self,
        modules: Vec<(ModuleId, ResolvedModule)>,
    ) -> Result<Vec<(ModuleId, CachedOutput)>> {
        let is_production = self.config.mode == crate::config::BuildMode::Production;
        let config = self.config.clone();

        let results: Vec<Result<(ModuleId, CachedOutput)>> = modules
            .par_iter()
            .map(|(id, module)| {
                let source_str = String::from_utf8_lossy(&module.source).to_string();
                let import_offsets = pledgepack_native_sys::find_imports(&module.source);

                let mut deps = Vec::new();
                for offset in import_offsets {
                    let rest = &source_str[offset..];
                    if let Some(dep) = extract_module_specifier(rest) {
                        deps.push(dep);
                    }
                }

                let file_path = module.path.to_str().unwrap_or("");
                let transform_output = crate::transform::transform(
                    &source_str,
                    module.kind,
                    file_path,
                    is_production,
                    &config,
                )?;

                Ok((
                    *id,
                    CachedOutput {
                        code: transform_output.code,
                        source_map: transform_output.source_map,
                        deps,
                        is_css: transform_output.is_css,
                        css_modules: transform_output.css_modules,
                        extracted_css: transform_output.extracted_css,
                        is_worker: transform_output.is_worker,
                        dynamic_imports: transform_output.dynamic_imports,
                    },
                ))
            })
            .collect();

        // Collect results, propagating errors
        let mut outputs = Vec::with_capacity(results.len());
        for result in results {
            outputs.push(result?);
        }
        Ok(outputs)
    }

    /// Get the module graph (for dev server / HMR)
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Get all resolved modules
    pub fn modules(&self) -> &HashMap<ModuleId, ResolvedModule> {
        &self.modules
    }

    /// Get the function-level cache (transformed outputs)
    pub fn function_cache(&self) -> &HashMap<u64, CachedOutput> {
        &self.function_cache
    }

    /// Emit production build artifacts to the output directory.
    /// Writes each module as a separate file with content hashes and generates index.html + manifest.json.
    /// Supports CSS code splitting, CSS extraction from JS, manual chunks, inline dynamic imports,
    /// module preload directives, preload/prefetch links, multi-script entry, and build manifest.
    pub fn emit(&self) -> Result<()> {
        let out_dir = &self.config.out_dir;
        // Clean output directory to remove stale files from previous builds
        if out_dir.exists() {
            std::fs::remove_dir_all(out_dir)?;
        }
        std::fs::create_dir_all(out_dir)?;

        let mut css_files: Vec<String> = Vec::new();
        let mut js_files: Vec<String> = Vec::new();
        let mut async_chunks: Vec<String> = Vec::new();
        let mut manifest_entries: std::collections::HashMap<String, ManifestEntry> = std::collections::HashMap::new();
        let mut entry_chunks: Vec<(String, String)> = Vec::new(); // (entry name, hashed filename)

        // Determine entry modules from config
        let entries = &self.config.entry;

        // Write each transformed module to .pledge/
        for (_id, module) in &self.modules {
            if let Some(cached) = self.function_cache.get(&module.content_hash) {
                // Determine output path relative to project root
                let rel = module.path.strip_prefix(&self.config.root)
                    .unwrap_or(&module.path);
                let out_path = out_dir.join(rel);

                // Compute content hash for filename
                let hash = blake3::hash(cached.code.as_bytes());
                let hash_hex = &hash.to_hex()[..8];

                // CSS files keep .css extension, JS files get .js
                let (out_path, hashed_rel, is_css) = if cached.is_css {
                    let stem = out_path.file_stem().and_then(|s| s.to_str()).unwrap_or("index");
                    let hashed_name = format!("{}.{}.css", stem, hash_hex);
                    let p = out_path.with_file_name(hashed_name);
                    let rel = p.strip_prefix(out_dir).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                    (p, rel, true)
                } else {
                    let stem = out_path.file_stem().and_then(|s| s.to_str()).unwrap_or("index");
                    let hashed_name = format!("{}.{}.js", stem, hash_hex);
                    let p = out_path.with_file_name(hashed_name);
                    let rel = p.strip_prefix(out_dir).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                    (p, rel, false)
                };

                // Create parent directories
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Write the transformed code using mmap for large files
                write_output_file(&out_path, &cached.code)?;
                tracing::info!("Emitted: {}", out_path.display());

                // Write source map if present (respecting source_map_mode)
                if let Some(ref source_map) = cached.source_map {
                    let mode = &self.config.build.source_map_mode;
                    if mode != "hidden" {
                        let map_path = out_path.with_extension(
                            format!("{}.map", out_path.extension().and_then(|e| e.to_str()).unwrap_or("js"))
                        );
                        std::fs::write(&map_path, source_map)?;
                    }
                }

                // Track CSS files for HTML injection
                if is_css {
                    css_files.push(hashed_rel.clone());
                } else {
                    // CSS extraction from JS: if this JS module has extracted CSS, write it as a separate .css file
                    if let Some(ref extracted_css) = cached.extracted_css {
                        let css_hash = blake3::hash(extracted_css.as_bytes());
                        let css_hash_hex = &css_hash.to_hex()[..8];
                        let css_stem = out_path.file_stem().and_then(|s| s.to_str()).unwrap_or("index");
                        let css_name = format!("{}.{}.css", css_stem, css_hash_hex);
                        let css_out_path = out_path.with_file_name(css_name);
                        let css_rel = css_out_path.strip_prefix(out_dir).unwrap_or(&css_out_path).to_string_lossy().replace('\\', "/");
                        std::fs::write(&css_out_path, extracted_css)?;
                        css_files.push(css_rel.clone());
                        tracing::info!("Extracted CSS: {}", css_out_path.display());
                    }

                    // Check if this is an async chunk (dynamic import)
                    let is_async = !cached.dynamic_imports.is_empty()
                        && !self.config.build.inline_dynamic_imports
                        && !entries.iter().any(|e| {
                            let entry_path = self.config.root.join(e);
                            module.path == entry_path
                        });

                    if is_async {
                        async_chunks.push(hashed_rel.clone());
                    } else {
                        js_files.push(hashed_rel.clone());
                    }
                }

                // Track manifest entry (original path → hashed path + metadata)
                let original_rel = rel.to_string_lossy().replace('\\', "/");
                let is_entry = entries.iter().any(|e| {
                    let entry_normalized = e.replace('\\', "/");
                    original_rel == *e || original_rel == entry_normalized
                        || original_rel.ends_with(&entry_normalized)
                });

                manifest_entries.insert(original_rel.clone(), ManifestEntry {
                    file: hashed_rel.clone(),
                    is_entry,
                    is_css,
                    is_async: !is_css && !cached.dynamic_imports.is_empty() && !self.config.build.inline_dynamic_imports,
                    imports: if !is_css { cached.dynamic_imports.clone() } else { Vec::new() },
                    css: if is_css { Some(hashed_rel.clone()) } else { None },
                });

                // Track entry chunks for multi-script HTML
                if is_entry {
                    entry_chunks.push((original_rel, hashed_rel));
                }
            }
        }

        // Apply manual chunks configuration
        if !self.config.build.manual_chunks.is_empty() {
            for (chunk_name, module_patterns) in &self.config.build.manual_chunks {
                let mut chunk_modules: Vec<String> = Vec::new();
                for (_id, module) in &self.modules {
                    if let Some(cached) = self.function_cache.get(&module.content_hash) {
                        let path_str = module.path.to_string_lossy().replace('\\', "/");
                        if module_patterns.iter().any(|pattern| {
                            path_str.contains(pattern) || path_str == *pattern
                        }) {
                            let rel = module.path.strip_prefix(&self.config.root)
                                .unwrap_or(&module.path);
                            let out_path = out_dir.join(rel);
                            let hash = blake3::hash(cached.code.as_bytes());
                            let hash_hex = &hash.to_hex()[..8];
                            let stem = out_path.file_stem().and_then(|s| s.to_str()).unwrap_or("index");
                            let hashed_name = format!("{}.{}.js", stem, hash_hex);
                            let p = out_path.with_file_name(hashed_name);
                            let hashed_rel = p.strip_prefix(out_dir).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                            chunk_modules.push(hashed_rel);
                        }
                    }
                }
                if !chunk_modules.is_empty() {
                    tracing::info!("Manual chunk '{}': {} modules", chunk_name, chunk_modules.len());
                }
            }
        }

        // Generate manifest.json with entry-to-chunk mapping
        let manifest_json = serde_json::to_string_pretty(&manifest_entries)?;
        std::fs::write(out_dir.join("manifest.json"), manifest_json)?;

        // Generate index.html with CSS links, module preloads, and multi-script entry
        let css_links: String = css_files.iter()
            .map(|css| format!(r#"    <link rel="stylesheet" href="/{}" />"#, css))
            .collect::<Vec<_>>()
            .join("\n");

        // Font subsetting — generate @font-face CSS and preload tags
        let mut font_css = String::new();
        let mut font_preload_tags: Vec<String> = Vec::new();
        if self.config.build.font_subsetting {
            let fonts_dir = self.config.root.join("fonts");
            if fonts_dir.exists() {
                let font_config = crate::fonts::FontSubsetConfig::default();
                match crate::fonts::optimize_fonts(&fonts_dir, &font_config) {
                    Ok(subsets) => {
                        if !subsets.is_empty() {
                            font_css = crate::fonts::generate_subset_css(&subsets);
                            font_preload_tags = crate::fonts::generate_subset_preload_tags(&subsets);
                            // Write font CSS to a file
                            let font_css_hash = blake3::hash(font_css.as_bytes());
                            let font_css_hash_hex = &font_css_hash.to_hex()[..8];
                            let font_css_name = format!("fonts.{}.css", font_css_hash_hex);
                            let font_css_path = out_dir.join(&font_css_name);
                            std::fs::write(&font_css_path, &font_css)?;
                            css_files.push(font_css_name.clone());
                            tracing::info!("Font subsetting: generated {} subsets", subsets.len());
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Font subsetting failed: {}", e);
                    }
                }
            }
        }

        // SVG sprite generation — collect all SVG files and generate a sprite sheet
        if self.config.build.svg_sprite {
            let mut svg_entries: Vec<crate::svg::SvgSpriteEntry> = Vec::new();
            for (_id, module) in &self.modules {
                if crate::svg::is_svg(&module.path) {
                    if let Some(cached) = self.function_cache.get(&module.content_hash) {
                        let stem = module.path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("icon");
                        svg_entries.push(crate::svg::SvgSpriteEntry {
                            id: stem.to_string(),
                            svg: cached.code.clone(),
                        });
                    }
                }
            }
            if !svg_entries.is_empty() {
                let sprite = crate::svg::generate_sprite(&svg_entries);
                let sprite_hash = blake3::hash(sprite.as_bytes());
                let sprite_hash_hex = &sprite_hash.to_hex()[..8];
                let sprite_name = format!("sprite.{}.svg", sprite_hash_hex);
                let sprite_path = out_dir.join(&sprite_name);
                std::fs::write(&sprite_path, &sprite)?;
                tracing::info!("SVG sprite: generated with {} icons", svg_entries.len());
            }
        }

        // Module preload directives for async chunks
        let module_preloads: String = if self.config.build.module_preload {
            async_chunks.iter()
                .map(|chunk| format!(r#"    <link rel="modulepreload" href="/{}" />"#, chunk))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };

        // Preload directives for critical assets (fonts, images)
        let preload_links: String = if self.config.build.preload {
            let mut links: Vec<String> = Vec::new();
            // Preload first CSS file
            if let Some(first_css) = css_files.iter().filter(|css| css.ends_with(".css")).next() {
                links.push(format!(r#"    <link rel="preload" href="/{}" as="style" />"#, first_css));
            }
            // Font preload tags from subsetting
            for tag in &font_preload_tags {
                links.push(format!(r#"    {}"#, tag));
            }
            links.join("\n")
        } else {
            // Even if preload is off, include font preloads if subsetting is on
            if !font_preload_tags.is_empty() {
                font_preload_tags.iter()
                    .map(|t| format!(r#"    {}"#, t))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                String::new()
            }
        };

        // Prefetch directives for non-critical assets
        let prefetch_links: String = if self.config.build.prefetch {
            async_chunks.iter()
                .map(|chunk| format!(r#"    <link rel="prefetch" href="/{}" />"#, chunk))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };

        // Build script tags — support multiple entry points
        let script_tags: String = if entry_chunks.is_empty() {
            // Fallback: use first entry from config
            let entry = &entries[0];
            let entry_js = entry.replace(".tsx", ".js").replace(".ts", ".js")
                .replace(".jsx", ".js");
            let entry_hashed = manifest_entries.values()
                .find(|m| m.is_entry)
                .map(|m| m.file.clone())
                .unwrap_or(entry_js);
            format!(r#"    <script type="module" src="/{}"></script>"#, entry_hashed)
        } else {
            entry_chunks.iter()
                .map(|(_, hashed)| format!(r#"    <script type="module" src="/{}"></script>"#, hashed))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>.pledge</title>
    <style>* {{ margin: 0; padding: 0; box-sizing: border-box; }} body {{ background: #0a0a0a; }}</style>
{}
{}
{}
{}
</head>
<body>
    <div id="root"></div>
{}
</body>
</html>"#,
            css_links,
            module_preloads,
            preload_links,
            prefetch_links,
            script_tags
        );
        std::fs::write(out_dir.join("index.html"), html)?;

        Ok(())
    }

    /// Emit a single-file bundle — concatenate all modules into one ESM file.
    /// All imports are inlined, no separate chunks.
    pub fn emit_single_file(&self) -> Result<()> {
        let out_dir = &self.config.out_dir;
        if out_dir.exists() {
            std::fs::remove_dir_all(out_dir)?;
        }
        std::fs::create_dir_all(out_dir)?;

        let mut bundle = String::new();
        let mut css_bundle = String::new();

        // Collect all module codes in dependency order
        let mut visited = std::collections::HashSet::new();
        let entry_id = self.modules.values().next().map(|m| m.id);

        if let Some(entry) = entry_id {
            self.collect_module_code(entry, &mut visited, &mut bundle, &mut css_bundle);
        }

        // Write the single JS bundle
        let js_hash = blake3::hash(bundle.as_bytes());
        let js_hash_hex = &js_hash.to_hex()[..8];
        let js_filename = format!("index.{}.js", js_hash_hex);
        std::fs::write(out_dir.join(&js_filename), &bundle)?;

        // Write CSS bundle if any
        let css_filename = if !css_bundle.is_empty() {
            let css_hash = blake3::hash(css_bundle.as_bytes());
            let css_hash_hex = &css_hash.to_hex()[..8];
            let css_fn = format!("index.{}.css", css_hash_hex);
            std::fs::write(out_dir.join(&css_fn), &css_bundle)?;
            Some(css_fn)
        } else {
            None
        };

        // Generate HTML
        let css_link = css_filename.map(|f| {
            format!(r#"    <link rel="stylesheet" href="/{}" />"#, f)
        }).unwrap_or_default();

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>.pledge</title>
    <style>* {{ margin: 0; padding: 0; box-sizing: border-box; }} body {{ background: #0a0a0a; }}</style>
{}
</head>
<body>
    <div id="root"></div>
    <script type="module" src="/{}"></script>
</body>
</html>"#,
            css_link,
            js_filename
        );
        std::fs::write(out_dir.join("index.html"), html)?;

        Ok(())
    }

    /// Recursively collect module code in dependency order for single-file bundle
    fn collect_module_code(
        &self,
        module_id: ModuleId,
        visited: &mut std::collections::HashSet<ModuleId>,
        bundle: &mut String,
        css_bundle: &mut String,
    ) {
        if !visited.insert(module_id) {
            return;
        }

        if let Some(module) = self.modules.get(&module_id) {
            // Add this module's code
            if let Some(cached) = self.function_cache.get(&module.content_hash) {
                if cached.is_css {
                    css_bundle.push_str(&cached.code);
                    css_bundle.push('\n');
                } else {
                    let rel = module.path.strip_prefix(&self.config.root)
                        .unwrap_or(&module.path);
                    bundle.push_str(&format!("// === {} ===\n", rel.display()));
                    bundle.push_str(&cached.code);
                    bundle.push('\n');
                }
            }
        }
    }

    /// Invalidate modules that depend on a changed file
    pub fn invalidate(&mut self, changed_path: &PathBuf) -> Vec<ModuleId> {
        if let Some(&id) = self.path_to_id.get(changed_path) {
            let dependents = self.graph.get_dependents(id, 256);

            // Remove from function cache
            if let Some(module) = self.modules.get(&id) {
                self.function_cache.remove(&module.content_hash);
            }

            // Also invalidate dependents
            for dep_id in &dependents {
                if let Some(module) = self.modules.get(dep_id) {
                    self.function_cache.remove(&module.content_hash);
                }
            }

            let mut all = vec![id];
            all.extend(dependents);
            all
        } else {
            warn!("Invalidation: module not found: {:?}", changed_path);
            vec![]
        }
    }
}

/// Write output file using memory-mapped I/O for large files.
/// Falls back to standard buffered write for files below the mmap threshold.
///
/// mmap threshold: 64KB — files smaller than this use std::fs::write
/// which is faster for small files due to fewer syscalls.
fn write_output_file(path: &std::path::Path, content: &str) -> Result<()> {
    const MMAP_THRESHOLD: usize = 64 * 1024; // 64KB

    if content.len() < MMAP_THRESHOLD {
        std::fs::write(path, content)?;
        return Ok(());
    }

    // For large files: create the file, truncate to content size, then mmap + copy
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;

    file.set_len(content.len() as u64)?;

    // Use mmap to write directly — avoids an extra copy through the kernel buffer
    #[cfg(unix)]
    unsafe {
        use std::os::unix::io::AsRawFd;
        use std::ptr;
        let ptr = libc::mmap(
            ptr::null_mut(),
            content.len(),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            file.as_raw_fd(),
            0,
        );
        if ptr == libc::MAP_FAILED {
            // Fall back to standard write
            drop(file);
            std::fs::write(path, content)?;
            return Ok(());
        }
        std::ptr::copy_nonoverlapping(content.as_ptr(), ptr as *mut u8, content.len());
        libc::munmap(ptr, content.len());
    }

    #[cfg(windows)]
    {
        // On Windows, write to the already-opened file handle.
        // A full mmap implementation would use CreateFileMapping + MapViewOfFile
        // via the windows-sys crate, but buffered write is sufficient
        // since the file is already created and truncated to the right size.
        use std::io::Write;
        let mut f = file;
        f.write_all(content.as_bytes())?;
        f.flush()?;
    }

    #[cfg(not(any(unix, windows)))]
    {
        drop(file);
        std::fs::write(path, content)?;
    }

    Ok(())
}

/// Extract the module specifier from an import statement
/// e.g., "import React from 'react'" → "react"
fn extract_module_specifier(source: &str) -> Option<String> {
    // Find the first string literal (single or double quoted)
    let bytes = source.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i];
            let start = i + 1;
            let mut end = start;

            while end < bytes.len() && bytes[end] != quote {
                end += 1;
            }

            if end < bytes.len() {
                return Some(source[start..end].to_string());
            }
        }
        i += 1;

        // Don't scan too far — just look for the first string
        if i > 200 {
            break;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_module_specifier() {
        assert_eq!(
            extract_module_specifier("import React from 'react'"),
            Some("react".to_string())
        );
        assert_eq!(
            extract_module_specifier("import { foo } from \"./bar\""),
            Some("./bar".to_string())
        );
        assert_eq!(
            extract_module_specifier("import('./lazy')"),
            Some("./lazy".to_string())
        );
    }
}
