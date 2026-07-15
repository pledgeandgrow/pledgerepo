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
    for ext in &["", ".js", ".mjs", ".cjs"] {
        let candidate = if ext.is_empty() {
            node_modules.join(specifier)
        } else {
            node_modules.join(format!("{}{}", specifier, ext))
        };
        if candidate.exists() {
            return Ok(candidate.canonicalize().unwrap_or(candidate));
        }
        // Check package.json main field
        let pkg_json = node_modules.join(specifier).join("package.json");
        if pkg_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                    let main = pkg.get("module").or_else(|| pkg.get("main"))
                        .and_then(|v| v.as_str()).unwrap_or("index.js");
                    let main_path = node_modules.join(specifier).join(main);
                    if main_path.exists() {
                        return Ok(main_path.canonicalize().unwrap_or(main_path));
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!("Cannot resolve '{}' from '{}'", specifier, importer))
}
