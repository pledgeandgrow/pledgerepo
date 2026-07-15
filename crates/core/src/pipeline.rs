// Pipeline: coordinates the build phases
//
// Phase 1 (Dev):  Resolve → Parse → Transform → Serve (no bundling)
// Phase 2 (Prod): Resolve → Parse → Transform → Optimize → Bundle → Output

use crate::config::BuildMode;
use crate::engine::BuildEngine;
use anyhow::Result;
use std::sync::Arc;

pub async fn run_build(config: Arc<crate::config::PledgeConfig>) -> Result<BuildEngine> {
    let engine = BuildEngine::new(config.clone());

    match config.mode {
        BuildMode::Development => run_dev_build(engine, &config).await,
        BuildMode::Production => run_prod_build(engine, &config).await,
    }
}

async fn run_dev_build(
    engine: BuildEngine,
    _config: &crate::config::PledgeConfig,
) -> Result<BuildEngine> {
    tracing::info!("Starting dev build...");
    Ok(engine)
}

async fn run_prod_build(
    mut engine: BuildEngine,
    config: &crate::config::PledgeConfig,
) -> Result<BuildEngine> {
    tracing::info!("Starting production build...");

    let profile = config.profile;
    let build_start = std::time::Instant::now();

    // Phase 1: Build module graph (resolve + parse + transform)
    let result = engine.build().await?;

    if profile {
        tracing::info!("[profile] Parse + Transform: {}ms", result.duration_ms);
    }

    // Phase 2: Optimize (tree shaking, code splitting, minification)
    // NOTE: Optimization is handled by the CLI layer which has access to pledgepack-optimizer
    // This avoids a cyclic dependency between core and optimizer

    // Phase 3: Emit output to disk with asset hashing + manifest
    let emit_start = std::time::Instant::now();
    engine.emit()?;
    tracing::info!("Output written to {}", config.out_dir.display());

    if profile {
        tracing::info!("[profile] Emit: {}ms", emit_start.elapsed().as_millis());
        tracing::info!("[profile] Total: {}ms", build_start.elapsed().as_millis());
    }

    tracing::info!(
        "Production build complete: {} modules in {}ms",
        result.modules_built + result.modules_cached,
        build_start.elapsed().as_millis()
    );

    Ok(engine)
}
