// Build analyzer — bundle size analysis and visualization
//
// Generates a JSON analysis of the production bundle, including:
//   - Per-module sizes (original + transformed)
//   - Chunk breakdown
//   - Dependency tree with sizes
//   - Duplicate module detection
//
// The output can be viewed with `pledge analyze` which serves an interactive
// treemap visualization in the browser.

use crate::engine::{BuildEngine, CachedOutput};
use crate::module::ModuleKind;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

/// Analysis result for a single module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleAnalysis {
    pub path: String,
    pub kind: String,
    pub original_size: usize,
    pub transformed_size: usize,
    pub dependencies: Vec<String>,
    pub is_entry: bool,
    pub is_css: bool,
    pub is_worker: bool,
}

/// Full bundle analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleAnalysis {
    pub total_modules: usize,
    pub total_original_size: usize,
    pub total_transformed_size: usize,
    pub modules: Vec<ModuleAnalysis>,
    pub chunks: Vec<ChunkAnalysis>,
    pub duplicates: Vec<DuplicateModule>,
    pub largest_modules: Vec<ModuleAnalysis>,
}

/// Chunk analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkAnalysis {
    pub name: String,
    pub size: usize,
    pub module_count: usize,
    pub modules: Vec<String>,
}

/// Duplicate module detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateModule {
    pub specifier: String,
    pub paths: Vec<String>,
    pub total_size: usize,
}

/// Analyze the build and generate a bundle analysis
pub fn analyze_build(engine: &BuildEngine) -> Result<BundleAnalysis> {
    let modules = engine.modules();
    let function_cache = engine.function_cache();

    let mut module_analyses: Vec<ModuleAnalysis> = Vec::new();
    let mut total_original = 0usize;
    let mut total_transformed = 0usize;

    for (id, module) in modules {
        let rel_path = module.path.to_string_lossy().replace('\\', "/");
        let original_size = module.source.len();
        let transformed_size = function_cache
            .get(&module.content_hash)
            .map(|c| c.code.len())
            .unwrap_or(0);

        let cached = function_cache.get(&module.content_hash);

        let kind_str = match module.kind {
            ModuleKind::Tsx => "tsx",
            ModuleKind::TypeScript => "ts",
            ModuleKind::Jsx => "jsx",
            ModuleKind::JavaScript => "js",
            ModuleKind::Css => "css",
            ModuleKind::Json => "json",
            ModuleKind::Asset => "asset",
            ModuleKind::Wasm => "wasm",
            ModuleKind::Vue => "vue",
            ModuleKind::Svelte => "svelte",
            ModuleKind::Astro => "astro",
            ModuleKind::Worker => "worker",
            _ => "unknown",
        };

        let deps = cached
            .map(|c| c.deps.clone())
            .unwrap_or_default();

        let is_css = cached.map(|c| c.is_css).unwrap_or(false);
        let is_worker = cached.map(|c| c.is_worker).unwrap_or(false);

        total_original += original_size;
        total_transformed += transformed_size;

        module_analyses.push(ModuleAnalysis {
            path: rel_path,
            kind: kind_str.to_string(),
            original_size,
            transformed_size,
            dependencies: deps,
            is_entry: engine.modules().values()
                .take(1)
                .any(|m| m.id == *id),
            is_css,
            is_worker,
        });
    }

    // Sort by size (largest first)
    let mut largest = module_analyses.clone();
    largest.sort_by(|a, b| b.transformed_size.cmp(&a.transformed_size));
    largest.truncate(20);

    // Detect duplicates (same specifier resolved from different paths)
    let duplicates = detect_duplicates(&module_analyses);

    // Generate chunk analysis (simplified — group by directory)
    let chunks = generate_chunk_analysis(&module_analyses);

    info!(
        "Bundle analysis: {} modules, {} total ({} → {})",
        module_analyses.len(),
        format_bytes(total_transformed),
        format_bytes(total_original),
        format_bytes(total_transformed),
    );

    Ok(BundleAnalysis {
        total_modules: module_analyses.len(),
        total_original_size: total_original,
        total_transformed_size: total_transformed,
        modules: module_analyses,
        chunks,
        duplicates,
        largest_modules: largest,
    })
}

/// Detect duplicate modules (same basename, different paths)
fn detect_duplicates(modules: &[ModuleAnalysis]) -> Vec<DuplicateModule> {
    let mut by_name: HashMap<String, Vec<&ModuleAnalysis>> = HashMap::new();

    for m in modules {
        let name = m.path.rsplit('/').next().unwrap_or(&m.path).to_string();
        by_name.entry(name).or_default().push(m);
    }

    let mut duplicates = Vec::new();
    for (name, entries) in &by_name {
        if entries.len() > 1 {
            let total_size: usize = entries.iter().map(|e| e.transformed_size).sum();
            duplicates.push(DuplicateModule {
                specifier: name.clone(),
                paths: entries.iter().map(|e| e.path.clone()).collect(),
                total_size,
            });
        }
    }

    duplicates.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    duplicates
}

/// Generate chunk analysis by grouping modules by directory
fn generate_chunk_analysis(modules: &[ModuleAnalysis]) -> Vec<ChunkAnalysis> {
    let mut by_dir: HashMap<String, Vec<&ModuleAnalysis>> = HashMap::new();

    for m in modules {
        let dir = m.path.rsplitn(2, '/').nth(1).unwrap_or("").to_string();
        by_dir.entry(dir).or_default().push(m);
    }

    let mut chunks: Vec<ChunkAnalysis> = by_dir
        .into_iter()
        .map(|(name, mods)| {
            let size: usize = mods.iter().map(|m| m.transformed_size).sum();
            ChunkAnalysis {
                name,
                size,
                module_count: mods.len(),
                modules: mods.iter().map(|m| m.path.clone()).collect(),
            }
        })
        .collect();

    chunks.sort_by(|a, b| b.size.cmp(&a.size));
    chunks
}

/// Generate the HTML visualization for the bundle analysis
pub fn generate_analysis_html(analysis: &BundleAnalysis) -> String {
    let total_kb = analysis.total_transformed_size as f64 / 1024.0;
    let original_kb = analysis.total_original_size as f64 / 1024.0;

    let module_rows: String = analysis.largest_modules
        .iter()
        .map(|m| {
            let size_kb = m.transformed_size as f64 / 1024.0;
            let pct = if analysis.total_transformed_size > 0 {
                (m.transformed_size as f64 / analysis.total_transformed_size as f64) * 100.0
            } else {
                0.0
            };
            format!(
                r#"<tr><td style="padding:4px 12px;color:#e0e0e0;">{}</td>
                <td style="padding:4px 12px;color:#888;">{}</td>
                <td style="padding:4px 12px;text-align:right;color:#e0e0e0;">{:.1}KB</td>
                <td style="padding:4px 12px;text-align:right;color:#666;">{:.1}%</td></tr>"#,
                m.path, m.kind, size_kb, pct
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Pledge Bundle Analysis</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ background: #0a0a0a; color: #e0e0e0; font-family: ui-monospace, monospace; padding: 2rem; }}
        h1 {{ color: #6366f1; margin-bottom: 1rem; }}
        .stats {{ display: flex; gap: 2rem; margin-bottom: 2rem; }}
        .stat {{ background: #1a1a1a; padding: 1rem; border-radius: 8px; border: 1px solid #333; }}
        .stat-label {{ color: #888; font-size: 0.8rem; margin-bottom: 0.25rem; }}
        .stat-value {{ color: #e0e0e0; font-size: 1.5rem; font-weight: 600; }}
        table {{ width: 100%; border-collapse: collapse; margin-top: 1rem; }}
        th {{ text-align: left; padding: 8px 12px; color: #888; border-bottom: 1px solid #333; }}
        td {{ border-bottom: 1px solid #222; }}
        .bar {{ background: #6366f1; height: 4px; border-radius: 2px; }}
    </style>
</head>
<body>
    <h1>Pledge Bundle Analysis</h1>
    <div class="stats">
        <div class="stat">
            <div class="stat-label">Total Size</div>
            <div class="stat-value">{:.1}KB</div>
        </div>
        <div class="stat">
            <div class="stat-label">Original Size</div>
            <div class="stat-value">{:.1}KB</div>
        </div>
        <div class="stat">
            <div class="stat-label">Modules</div>
            <div class="stat-value">{}</div>
        </div>
        <div class="stat">
            <div class="stat-label">Chunks</div>
            <div class="stat-value">{}</div>
        </div>
        <div class="stat">
            <div class="stat-label">Duplicates</div>
            <div class="stat-value">{}</div>
        </div>
    </div>
    <h2 style="color:#888;font-size:1rem;margin-bottom:0.5rem;">Largest Modules</h2>
    <table>
        <thead><tr><th>Path</th><th>Type</th><th style="text-align:right;">Size</th><th style="text-align:right;">%</th></tr></thead>
        <tbody>{}</tbody>
    </table>
</body>
</html>"#,
        total_kb,
        original_kb,
        analysis.total_modules,
        analysis.chunks.len(),
        analysis.duplicates.len(),
        module_rows,
    )
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
