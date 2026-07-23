// Programmatic API — Vite-compatible JavaScript API
//
// Provides:
//   - createServer(options) → DevServer handle
//   - build(options) → BuildResult
//   - transform(code, id) → TransformResult
//
// These functions wrap the internal engine, transform, and dev-server
// into a clean public API that can be called from Rust code or
// exposed via FFI to Node.js bindings.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::PledgeConfig;
use crate::engine::{BuildEngine, BuildResult};
use crate::module::ModuleKind;
use crate::transform::transform as transform_module;

/// Options for creating a dev server programmatically
#[derive(Debug, Clone)]
pub struct CreateServerOptions {
    pub root: PathBuf,
    pub port: u16,
    pub host: String,
    pub hmr: bool,
    pub https: bool,
    pub mode: String,
}

impl Default for CreateServerOptions {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            port: 3000,
            host: "localhost".to_string(),
            hmr: true,
            https: false,
            mode: "development".to_string(),
        }
    }
}

/// Options for a programmatic build
#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub root: PathBuf,
    pub out_dir: PathBuf,
    pub mode: String,
    pub sourcemap: bool,
    pub minify: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            out_dir: PathBuf::from(".pledge/dist"),
            mode: "production".to_string(),
            sourcemap: true,
            minify: true,
        }
    }
}

/// Result of a transform operation
#[derive(Debug, Clone)]
pub struct TransformResult {
    pub code: String,
    pub map: Option<String>,
    pub deps: Vec<String>,
}

/// Create a dev server instance programmatically
pub fn create_server(options: CreateServerOptions) -> Result<(BuildEngine, PledgeConfig)> {
    let mut config = PledgeConfig::load(&options.root)?;
    config.dev_server.port = options.port;
    config.dev_server.host = options.host;
    config.dev_server.hmr = options.hmr;
    config.mode = match options.mode.as_str() {
        "production" => crate::config::BuildMode::Production,
        _ => crate::config::BuildMode::Development,
    };

    let engine = BuildEngine::new(Arc::new(config.clone()));
    Ok((engine, config))
}

/// Run a build programmatically
pub async fn build(options: BuildOptions) -> Result<BuildResult> {
    let mut config = PledgeConfig::load(&options.root)?;
    config.out_dir = options.out_dir;
    config.mode = match options.mode.as_str() {
        "production" => crate::config::BuildMode::Production,
        _ => crate::config::BuildMode::Development,
    };
    config.source_maps = options.sourcemap;

    let mut engine = BuildEngine::new(Arc::new(config));
    engine.build().await
}

/// Transform a single module programmatically
pub fn transform(code: &str, id: &str, is_production: bool, config: &PledgeConfig) -> Result<TransformResult> {
    let kind = ModuleKind::from_extension(
        &std::path::Path::new(id)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default(),
    );

    let output = transform_module(code, kind, id, is_production, config)?;

    Ok(TransformResult {
        code: output.code,
        map: output.source_map,
        deps: output.dynamic_imports,
    })
}

/// Resolve a module specifier to a file path
pub fn resolve(specifier: &str, importer: &str, root: &std::path::Path) -> Result<PathBuf> {
    // Handle relative paths
    if specifier.starts_with("./") || specifier.starts_with("../") {
        let importer_dir = std::path::Path::new(importer).parent().unwrap_or(root);
        let resolved = importer_dir.join(specifier);
        // Try with extensions
        for ext in &["", ".ts", ".tsx", ".js", ".jsx", ".json", ".css"] {
            let candidate = if ext.is_empty() {
                resolved.clone()
            } else {
                resolved.with_extension(ext.trim_start_matches('.'))
            };
            if candidate.exists() {
                return Ok(candidate.canonicalize().unwrap_or(candidate));
            }
        }
        return Err(anyhow::anyhow!("Cannot resolve '{}' from '{}'", specifier, importer));
    }

    // Handle absolute paths
    if specifier.starts_with('/') {
        let path = PathBuf::from(specifier);
        if path.exists() {
            return Ok(path);
        }
    }

    // Handle bare specifiers — look in node_modules
    let node_modules = root.join("node_modules");

    // Handle scoped packages: @org/pkg or @org/pkg/subpath
    let (pkg_dir_name, subpath) = if specifier.starts_with('@') {
        let parts: Vec<&str> = specifier.splitn(3, '/').collect();
        if parts.len() >= 2 {
            let pkg = format!("{}/{}", parts[0], parts[1]);
            let sub = if parts.len() > 2 { format!("/{}", parts[2]) } else { String::new() };
            (pkg, sub)
        } else {
            (specifier.to_string(), String::new())
        }
    } else {
        let parts: Vec<&str> = specifier.splitn(2, '/').collect();
        if parts.len() > 1 {
            (parts[0].to_string(), format!("/{}", parts[1]))
        } else {
            (specifier.to_string(), String::new())
        }
    };

    let dep_dir = node_modules.join(&pkg_dir_name);
    if dep_dir.is_dir() {
        // Check exports field first
        let pkg_json_path = dep_dir.join("package.json");
        if pkg_json_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_json_path) {
                if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Check exports for subpath
                    if !subpath.is_empty() {
                        if let Some(exports) = pkg.get("exports") {
                            if let Some(obj) = exports.as_object() {
                                let key = format!(".{}", subpath);
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
                                            return Ok(full.canonicalize().unwrap_or(full));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Fallback: module or main
                    let main = pkg.get("module").or_else(|| pkg.get("main"))
                        .and_then(|v| v.as_str()).unwrap_or("index.js");
                    let main_path = dep_dir.join(main);
                    if main_path.exists() {
                        return Ok(main_path.canonicalize().unwrap_or(main_path));
                    }
                }
            }
        }
        // Fall back to index.js
        let index = dep_dir.join("index.js");
        if index.exists() {
            return Ok(index.canonicalize().unwrap_or(index));
        }
        // Try subpath as direct file
        if !subpath.is_empty() {
            let direct = dep_dir.join(subpath.trim_start_matches('/'));
            if direct.exists() {
                return Ok(direct.canonicalize().unwrap_or(direct));
            }
        }
    }

    // Try direct file resolution
    let direct = node_modules.join(format!("{}.js", specifier));
    if direct.exists() {
        return Ok(direct.canonicalize().unwrap_or(direct));
    }

    Err(anyhow::anyhow!("Cannot resolve '{}' from '{}'", specifier, importer))
}
