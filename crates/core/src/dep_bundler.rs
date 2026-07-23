// Dependency pre-bundling — scan node_modules and pre-bundle dependencies
//
// This module identifies bare imports (e.g., `import React from "react"`) in the
// project source, resolves them from node_modules, and pre-bundles them into
// optimized ESM modules. This converts CJS dependencies to ESM and deduplicates
// shared dependencies.
//
// Similar to Vite's dep pre-bundling (which uses esbuild), this module:
//   1. Scans entry points for bare imports
//   2. Resolves each dependency via the resolver
//   3. Converts CJS → ESM using interop wrappers
//   4. Writes pre-bundled deps to node_modules/.pledge-deps

use crate::config::PledgeConfig;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{info, warn};

/// Pre-bundled dependency information
#[derive(Debug, Clone)]
pub struct PreBundledDep {
    /// The original bare specifier (e.g., "react")
    pub specifier: String,
    /// The resolved path in node_modules
    pub source_path: PathBuf,
    /// The pre-bundled ESM output path
    pub output_path: PathBuf,
    /// Whether this was a CJS module that needed interop
    pub was_cjs: bool,
    /// Size of the pre-bundled output in bytes
    pub size: usize,
}

/// Dep pre-bundler
pub struct DepBundler {
    deps: HashMap<String, PreBundledDep>,
}

impl DepBundler {
    pub fn new() -> Self {
        Self {
            deps: HashMap::new(),
        }
    }

    /// Scan source files for bare imports and pre-bundle them.
    /// Returns the list of pre-bundled dependencies.
    pub fn pre_bundle(&mut self, config: &PledgeConfig) -> Result<Vec<PreBundledDep>> {
        let root = &config.root;
        let deps_dir = root.join("node_modules").join(".pledge-deps");
        std::fs::create_dir_all(&deps_dir)?;

        // Step 1: Scan all entry points and their dependencies for bare imports
        let bare_imports = self.scan_for_bare_imports(config)?;

        info!("Pre-bundling {} dependencies", bare_imports.len());

        // Step 2: Resolve and pre-bundle each dependency
        for specifier in &bare_imports {
            if self.deps.contains_key(specifier) {
                continue;
            }

            match self.pre_bundle_dep(specifier, root, &deps_dir) {
                Ok(dep) => {
                    info!(
                        "  ✓ {} ({} bytes{})",
                        dep.specifier,
                        dep.size,
                        if dep.was_cjs { " [CJS→ESM]" } else { "" }
                    );
                    self.deps.insert(specifier.clone(), dep);
                }
                Err(e) => {
                    warn!("  ✗ Failed to pre-bundle {}: {}", specifier, e);
                }
            }
        }

        Ok(self.deps.values().cloned().collect())
    }

    /// Scan source files for bare imports (non-relative specifiers)
    fn scan_for_bare_imports(&self, config: &PledgeConfig) -> Result<HashSet<String>> {
        let mut imports = HashSet::new();

        for entry in &config.entry {
            let entry_path = config.root.join(entry);
            if entry_path.exists() {
                let source = std::fs::read_to_string(&entry_path)?;
                Self::extract_bare_imports(&source, &mut imports);
            }
        }

        // Also scan common source directories
        let src_dir = config.root.join("src");
        if src_dir.exists() {
            self.scan_directory(&src_dir, &mut imports)?;
        }

        Ok(imports)
    }

    /// Recursively scan a directory for bare imports
    fn scan_directory(&self, dir: &PathBuf, imports: &mut HashSet<String>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip node_modules and hidden directories
                if path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n == "node_modules" || n.starts_with('.'))
                    .unwrap_or(false)
                {
                    continue;
                }
                self.scan_directory(&path, imports)?;
            } else if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mjs") {
                    if let Ok(source) = std::fs::read_to_string(&path) {
                        Self::extract_bare_imports(&source, imports);
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract bare import specifiers from source code
    fn extract_bare_imports(source: &str, imports: &mut HashSet<String>) {
        // Look for import patterns: from "X", import "X", import("X")
        for pattern in ["from \"", "from '", "import \"", "import '", "import("] {
            let mut search_pos = 0;
            while let Some(pos) = source[search_pos..].find(pattern) {
                let abs_pos = search_pos + pos;
                let after_pattern = abs_pos + pattern.len();
                let rest = &source[after_pattern..];

                let closing = if pattern.ends_with('"') { '"' }
                    else if pattern.ends_with('\'') { '\'' }
                    else { '(' };

                if closing == '(' {
                    // Dynamic import: find the string inside
                    if let Some(q_pos) = rest.find(|c: char| c == '"' || c == '\'') {
                        let q_char = rest.as_bytes()[q_pos] as char;
                        let spec_start = q_pos + 1;
                        if let Some(end) = rest[spec_start..].find(q_char) {
                            let specifier = &rest[spec_start..spec_start + end];
                            if Self::is_bare_specifier(specifier) {
                                imports.insert(specifier.to_string());
                            }
                        }
                    }
                } else if let Some(end) = rest.find(closing) {
                    let specifier = &rest[..end];
                    if Self::is_bare_specifier(specifier) {
                        imports.insert(specifier.to_string());
                    }
                }

                search_pos = after_pattern;
            }
        }
    }

    /// Check if a specifier is a bare import (not relative or absolute)
    fn is_bare_specifier(specifier: &str) -> bool {
        !specifier.starts_with("./")
            && !specifier.starts_with("../")
            && !specifier.starts_with("/")
            && !specifier.starts_with("http")
            && !specifier.is_empty()
    }

    /// Pre-bundle a single dependency
    fn pre_bundle_dep(
        &self,
        specifier: &str,
        root: &PathBuf,
        deps_dir: &PathBuf,
    ) -> Result<PreBundledDep> {
        // Resolve the dependency from node_modules
        let node_modules = root.join("node_modules");
        let dep_path = self.resolve_dep(specifier, &node_modules)?;

        // Read the source
        let source = std::fs::read_to_string(&dep_path)?;

        // Check if it's CJS or ESM
        let is_cjs = !source.contains("export ")
            && !source.contains("export default")
            && !source.contains("import ")
            && (source.contains("module.exports") || source.contains("require("));

        // Generate ESM wrapper for CJS modules
        let (esm_code, was_cjs) = if is_cjs {
            (Self::cjs_to_esm_wrapper(specifier, &source), true)
        } else {
            // Already ESM, just copy with potential optimizations
            (source, false)
        };

        // Write pre-bundled output
        let safe_name = specifier.replace('/', "_").replace('@', "");
        let output_path = deps_dir.join(format!("{}.js", safe_name));
        std::fs::write(&output_path, &esm_code)?;

        let size = esm_code.len();

        Ok(PreBundledDep {
            specifier: specifier.to_string(),
            source_path: dep_path,
            output_path,
            was_cjs,
            size,
        })
    }

    /// Resolve a bare specifier to a file path in node_modules
    fn resolve_dep(&self, specifier: &str, node_modules: &PathBuf) -> Result<PathBuf> {
        // Handle scoped packages and subpaths: @org/pkg/sub → @org/pkg + /sub
        let (pkg_name, subpath) = if specifier.starts_with('@') {
            let parts: Vec<&str> = specifier.splitn(3, '/').collect();
            if parts.len() >= 2 {
                let pkg = format!("{}/{}", parts[0], parts[1]);
                let sub = if parts.len() > 2 { parts[2] } else { "" };
                (pkg, sub)
            } else {
                (specifier.to_string(), "")
            }
        } else {
            let parts: Vec<&str> = specifier.splitn(2, '/').collect();
            if parts.len() > 1 {
                (parts[0].to_string(), parts[1])
            } else {
                (specifier.to_string(), "")
            }
        };

        let dep_dir = node_modules.join(&pkg_name);

        if dep_dir.is_dir() {
            let pkg_json_path = dep_dir.join("package.json");
            if pkg_json_path.exists() {
                let pkg = std::fs::read_to_string(&pkg_json_path)?;
                let pkg_json: serde_json::Value = serde_json::from_str(&pkg)?;

                // Check exports field for subpath
                if !subpath.is_empty() {
                    if let Some(exports) = pkg_json.get("exports") {
                        if let Some(obj) = exports.as_object() {
                            let key = format!("./{}", subpath);
                            if let Some(export_val) = obj.get(&key) {
                                let resolved = if let Some(s) = export_val.as_str() {
                                    Some(s.to_string())
                                } else if let Some(conditions) = export_val.as_object() {
                                    conditions.get("browser")
                                        .or_else(|| conditions.get("module"))
                                        .or_else(|| conditions.get("import"))
                                        .or_else(|| conditions.get("default"))
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                } else {
                                    None
                                };
                                if let Some(resolved_path) = resolved {
                                    let full = dep_dir.join(resolved_path.trim_start_matches("./"));
                                    if full.exists() {
                                        return Ok(full);
                                    }
                                }
                            }
                        }
                    }
                    // Try direct subpath file
                    let direct = dep_dir.join(subpath);
                    if direct.exists() {
                        return Ok(direct);
                    }
                }

                // Prefer "module" (ESM) over "main" (CJS)
                let entry = pkg_json
                    .get("module")
                    .and_then(|v| v.as_str())
                    .or_else(|| pkg_json.get("main").and_then(|v| v.as_str()))
                    .unwrap_or("index.js");

                let entry_path = dep_dir.join(entry);
                if entry_path.exists() {
                    return Ok(entry_path);
                }
            }

            // Fall back to index.js
            let index = dep_dir.join("index.js");
            if index.exists() {
                return Ok(index);
            }
        }

        // Try direct file resolution
        let direct = node_modules.join(format!("{}.js", specifier));
        if direct.exists() {
            return Ok(direct);
        }

        anyhow::bail!("Could not resolve dependency: {}", specifier)
    }

    /// Generate an ESM wrapper for a CJS module
    pub fn cjs_to_esm_wrapper(specifier: &str, cjs_source: &str) -> String {
        // Create an ESM wrapper that imports the CJS module and re-exports
        let safe_name = specifier.replace('/', "_").replace('@', "").replace('-', "_");

        format!(
            r#"// Pledge pre-bundled: {} (CJS → ESM interop)
const __pledge_cjs_module = {{}};
const __pledge_require = (id) => __pledge_cjs_module.exports || {{}};
const module = {{ exports: __pledge_cjs_module }};
const exports = __pledge_cjs_module;
const require = __pledge_require;

{}
const __pledge_default = module.exports;

export default __pledge_default;
export const __pledge_named = new Proxy(__pledge_default, {{
    get: (target, prop) => target[prop]
}});
"#,
            specifier,
            cjs_source
        )
    }

    /// Get the pre-bundled dependency info for a specifier
    pub fn get_dep(&self, specifier: &str) -> Option<&PreBundledDep> {
        self.deps.get(specifier)
    }

    /// Get all pre-bundled dependencies
    pub fn deps(&self) -> &HashMap<String, PreBundledDep> {
        &self.deps
    }
}

impl Default for DepBundler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_bare_specifier() {
        assert!(DepBundler::is_bare_specifier("react"));
        assert!(DepBundler::is_bare_specifier("@tanstack/router"));
        assert!(!DepBundler::is_bare_specifier("./foo"));
        assert!(!DepBundler::is_bare_specifier("../bar"));
        assert!(!DepBundler::is_bare_specifier("/abs/path"));
    }

    #[test]
    fn test_extract_bare_imports() {
        let source = r#"
            import React from "react";
            import { createRoot } from "react-dom/client";
            import { defineConfig } from "pledge";
            import "./local.css";
            import "../utils.js";
        "#;
        let mut imports = HashSet::new();
        DepBundler::extract_bare_imports(source, &mut imports);
        assert!(imports.contains("react"));
        assert!(imports.contains("react-dom/client"));
        assert!(imports.contains("pledge"));
        assert!(!imports.contains("./local.css"));
        assert!(!imports.contains("../utils.js"));
    }
}
