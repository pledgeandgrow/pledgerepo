// Build engine: orchestrates the entire build pipeline
//
// This is the "Turbo engine" equivalent — a function-level
// incremental computation system that caches aggressively
// and only recomputes what changed.

use crate::config::PledgeConfig;
use crate::module::{ModuleId, ResolvedModule};
use anyhow::Result;
use pledgepack_native_sys::Graph;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

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

        Self {
            config,
            graph: Graph::new(),
            path_to_id: HashMap::new(),
            modules: HashMap::new(),
            function_cache: HashMap::new(),
            persistent_cache,
        }
    }

    /// Run a full build (dev or production)
    pub async fn build(&mut self) -> Result<BuildResult> {
        let start = std::time::Instant::now();

        // Phase 1: Resolve entry points (relative to project root)
        let entries = self.config.entry.clone();
        for entry in &entries {
            self.resolve_and_add(entry, None)?;
        }

        // Phase 2: Parse, transform, and build graph
        let mut modules_built = 0usize;
        let mut modules_cached = 0usize;

        // Process modules in BFS order from entry points
        let mut queue: Vec<ModuleId> = self.path_to_id.values().copied().collect();
        let mut processed = std::collections::HashSet::new();

        while let Some(module_id) = queue.pop() {
            if processed.contains(&module_id) {
                continue;
            }
            processed.insert(module_id);

            let module = match self.modules.get(&module_id) {
                Some(m) => m.clone(),
                None => continue,
            };

            // Check function-level cache (memory first, then disk)
            let cache_key = module.content_hash;
            if let Some(cached) = self.function_cache.get(&cache_key).cloned() {
                modules_cached += 1;
                // Add cached dependencies to the graph
                for dep_path in &cached.deps {
                    let dep_id = self.resolve_and_add(dep_path, Some(&module.path))?;
                    self.graph.add_dependency(module_id, dep_id);
                    queue.push(dep_id);
                }
            } else if let Some(ref pc) = self.persistent_cache {
                // Check persistent (disk) cache
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
                        queue.push(dep_id);
                    }
                } else {
                    modules_built += 1;
                    let output = self.transform_module(&module).await?;
                    // Persist to disk
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
                        queue.push(dep_id);
                    }
                    self.function_cache.insert(cache_key, output);
                }
            } else {
                modules_built += 1;
                // Transform and cache
                let output = self.transform_module(&module).await?;
                for dep_path in &output.deps {
                    let dep_id = self.resolve_and_add(dep_path, Some(&module.path))?;
                    self.graph.add_dependency(module_id, dep_id);
                    queue.push(dep_id);
                }
                self.function_cache.insert(cache_key, output);
            }
        }

        let duration = start.elapsed();

        info!(
            "Build complete: {} built, {} cached, {}ms",
            modules_built,
            modules_cached,
            duration.as_millis()
        );

        Ok(BuildResult {
            modules_built,
            modules_cached,
            duration_ms: duration.as_millis(),
        })
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
            let pkg_json = node_modules.join(specifier).join("package.json");

            if pkg_json.exists() {
                let content = std::fs::read_to_string(&pkg_json)?;
                let pkg: serde_json::Value = serde_json::from_str(&content)?;

                // Resolve via "module" or "main" field
                let entry = pkg
                    .get("module")
                    .or_else(|| pkg.get("main"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("index.js");

                return Ok(node_modules.join(specifier).join(entry));
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
    pub fn emit(&self) -> Result<()> {
        let out_dir = &self.config.out_dir;
        std::fs::create_dir_all(out_dir)?;

        let mut css_files: Vec<String> = Vec::new();
        let mut manifest_entries: Vec<(String, String)> = Vec::new();

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
                let (out_path, hashed_rel) = if cached.is_css {
                    let stem = out_path.file_stem().and_then(|s| s.to_str()).unwrap_or("index");
                    let hashed_name = format!("{}.{}.css", stem, hash_hex);
                    let p = out_path.with_file_name(hashed_name);
                    let rel = p.strip_prefix(out_dir).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                    (p, rel)
                } else {
                    let stem = out_path.file_stem().and_then(|s| s.to_str()).unwrap_or("index");
                    let hashed_name = format!("{}.{}.js", stem, hash_hex);
                    let p = out_path.with_file_name(hashed_name);
                    let rel = p.strip_prefix(out_dir).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                    (p, rel)
                };

                // Create parent directories
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Write the transformed code
                std::fs::write(&out_path, &cached.code)?;
                tracing::info!("Emitted: {}", out_path.display());

                // Write source map if present
                if let Some(ref source_map) = cached.source_map {
                    let map_path = out_path.with_extension(
                        format!("{}.map", out_path.extension().and_then(|e| e.to_str()).unwrap_or("js"))
                    );
                    std::fs::write(&map_path, source_map)?;
                }

                // Track CSS files for HTML injection
                if cached.is_css {
                    css_files.push(hashed_rel.clone());
                }

                // Track manifest entry (original path → hashed path)
                let original_rel = rel.to_string_lossy().replace('\\', "/");
                manifest_entries.push((original_rel, hashed_rel));
            }
        }

        // Generate manifest.json
        let manifest: std::collections::HashMap<String, String> = manifest_entries.into_iter().collect();
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(out_dir.join("manifest.json"), manifest_json)?;

        // Generate index.html with CSS links
        let entry = &self.config.entry[0];
        let entry_js = entry.replace(".tsx", ".js").replace(".ts", ".js")
            .replace(".jsx", ".js");

        // Resolve entry to hashed filename from manifest
        let entry_hashed = manifest.get(&entry_js)
            .cloned()
            .unwrap_or_else(|| entry_js.clone());

        // Build CSS <link> tags
        let css_links: String = css_files.iter()
            .map(|css| format!(r#"    <link rel="stylesheet" href="/{}" />"#, css))
            .collect::<Vec<_>>()
            .join("\n");

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
            css_links,
            entry_hashed
        );
        std::fs::write(out_dir.join("index.html"), html)?;

        Ok(())
    }

    /// Emit a single-file bundle — concatenate all modules into one ESM file.
    /// All imports are inlined, no separate chunks.
    pub fn emit_single_file(&self) -> Result<()> {
        let out_dir = &self.config.out_dir;
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
