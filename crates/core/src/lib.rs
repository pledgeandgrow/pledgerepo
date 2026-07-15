// pledge-core: The core build engine
//
// Orchestrates the build pipeline:
//   1. Resolve entry point
//   2. Parse + transform modules (via SWC)
//   3. Build module graph (via Zig native layer)
//   4. Cache results (function-level incremental computation)
//   5. Output bundles (dev: serve modules, prod: optimize + chunk)

pub mod analyzer;
pub mod compression;
pub mod config;
pub mod config_validate;
pub mod css_frameworks;
pub mod dep_bundler;
pub mod detect;
pub mod doctor;
pub mod edge;
pub mod engine;
pub mod env;
pub mod fonts;
pub mod html;
pub mod image_pipeline;
pub mod migrate;
pub mod module;
pub mod pipeline;
pub mod polyfills;
pub mod postcss;
pub mod presets;
pub mod service_worker;
pub mod svg;
pub mod transform;

pub use config::PledgeConfig;
pub use config::PathAlias;
pub use config::ProxyConfig;
pub use config::OutputFormat;
pub use config::ImageConfig;
pub use config::LibraryConfig;
pub use config::HttpsConfig;
pub use config::WatchConfig;
pub use engine::BuildEngine;
pub use env::EnvVars;
pub use module::{ModuleId, ModuleKind, ResolvedModule};

use pledgepack_native_sys as native;

/// Re-export the Zig-backed graph for internal use
pub use native::Graph;
