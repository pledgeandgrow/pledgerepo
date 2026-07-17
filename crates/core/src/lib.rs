// pledge-core: The core build engine
//
// Orchestrates the build pipeline:
//   1. Resolve entry point
//   2. Parse + transform modules (via SWC)
//   3. Build module graph (via Zig native layer)
//   4. Cache results (function-level incremental computation)
//   5. Output bundles (dev: serve modules, prod: optimize + chunk)

pub mod analyzer;
pub mod api;
pub mod asset_pipeline;
pub mod a11y;
pub mod advanced;
pub mod bench;
pub mod budgets;
pub mod compression;
pub mod config;
pub mod config_validate;
pub mod css_frameworks;
pub mod css_features;
pub mod css_in_js;
pub mod dep_bundler;
pub mod detect;
pub mod doctor;
pub mod edge;
pub mod encrypt;
pub mod engine;
pub mod env;
pub mod fonts;
pub mod html;
pub mod i18n;
pub mod image_pipeline;
pub mod lsp_server;
pub mod migrate;
pub mod module;
pub mod module_graph;
pub mod output_distribution;
pub mod pipeline;
pub mod plugin_system;
pub mod polyfills;
pub mod postcss;
pub mod presets;
pub mod router;
pub mod rtl;
pub mod service_worker;
pub mod svg;
pub mod tailwind_v4;
pub mod telemetry;
pub mod transform;
pub mod transform_optimizations;
pub mod webhooks;
pub mod ecosystem;
pub mod css_advanced;
pub mod security;
pub mod performance;

pub use config::PledgeConfig;
pub use config::PathAlias;
pub use config::ProxyConfig;
pub use config::OutputFormat;
pub use config::ImageConfig;
pub use config::LibraryConfig;
pub use config::HttpsConfig;
pub use config::WatchConfig;
pub use config::BuildConfig;
pub use config::TestConfig;
pub use config::CacheConfig;
pub use config::RemoteCacheSettings;
pub use config::GraphqlConfig;
pub use config::SwCachingConfig;
pub use config::SwCacheRule;
pub use config::ExportsConfig;
pub use config::PluginPreset;
pub use config::TransformPipelineConfig;
pub use config::WorkspaceConfig;
pub use config::SecurityConfig;
pub use engine::BuildEngine;
pub use env::EnvVars;
pub use module::{ModuleId, ModuleKind, ResolvedModule};
pub use module_graph::SerializableModuleGraph;

use pledgepack_native_sys as native;

/// Re-export the Zig-backed graph for internal use
pub use native::Graph;

/// Create a debounced file watcher using notify-debouncer.
/// Returns a receiver that yields paths of changed files (debounced).
/// This replaces manual debounce logic with the crate's built-in debouncing.
pub fn create_debounced_watcher(
    root: &std::path::Path,
    debounce_ms: u64,
) -> anyhow::Result<std::sync::mpsc::Receiver<std::path::PathBuf>> {
    use notify::RecursiveMode;
    use notify_debouncer_full::new_debouncer;
    use std::time::Duration;

    let (tx, rx) = std::sync::mpsc::channel::<std::path::PathBuf>();

    let mut debouncer = new_debouncer(
        Duration::from_millis(debounce_ms),
        None,
        move |result: Result<Vec<notify_debouncer_full::DebouncedEvent>, Vec<notify::Error>>| {
            if let Ok(events) = result {
                for event in events {
                    for path in &event.paths {
                        let _ = tx.send(path.clone());
                    }
                }
            }
        },
    )?;

    debouncer.watch(root, RecursiveMode::Recursive)?;
    std::mem::forget(debouncer);

    Ok(rx)
}

/// Format a byte count as a human-readable string using humansize.
/// Replaces the 4 duplicate `format_bytes` functions across the codebase.
pub fn format_size(bytes: usize) -> String {
    humansize::format_size(bytes, humansize::BINARY)
}

/// Generate a JSON Schema for `PledgeConfig`, suitable for IDE autocompletion
/// and config validation. Returns the schema as a `serde_json::Value`.
pub fn generate_config_schema() -> serde_json::Value {
    let schema = schemars::schema_for!(PledgeConfig);
    serde_json::to_value(&schema).unwrap_or(serde_json::Value::Null)
}
