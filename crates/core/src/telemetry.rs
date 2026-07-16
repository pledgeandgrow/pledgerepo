// Build telemetry — build history tracking and dashboard (#101)
//
// Records build metrics (duration, module count, cache hit rate, bundle size)
// to .pledge/history.json and serves an interactive web dashboard.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

/// A single build history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecord {
    /// Unix timestamp of the build
    pub timestamp: u64,
    /// Build duration in milliseconds
    pub duration_ms: u128,
    /// Number of modules built
    pub modules_built: usize,
    /// Number of modules served from cache
    pub modules_cached: usize,
    /// Total bundle size in bytes
    pub bundle_size: usize,
    /// Build mode ("production" or "development")
    pub mode: String,
    /// Cache hit rate (0.0 - 1.0)
    pub cache_hit_rate: f64,
    /// Number of chunks emitted
    pub chunk_count: usize,
    /// Whether the build succeeded
    pub success: bool,
    /// Error message if build failed
    pub error: Option<String>,
}

/// Build history persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildHistory {
    pub builds: Vec<BuildRecord>,
}

impl BuildHistory {
    /// Load history from .pledge/history.json
    pub fn load(root: &PathBuf) -> Result<Self> {
        let path = root.join(".pledge").join("history.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    /// Save history to .pledge/history.json
    pub fn save(&self, root: &PathBuf) -> Result<()> {
        let dir = root.join(".pledge");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("history.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Add a build record, keeping at most 100 entries
    pub fn add(&mut self, record: BuildRecord) {
        self.builds.push(record);
        if self.builds.len() > 100 {
            self.builds.remove(0);
        }
    }

    /// Get recent builds (last N)
    pub fn recent(&self, n: usize) -> &[BuildRecord] {
        let start = self.builds.len().saturating_sub(n);
        &self.builds[start..]
    }

    /// Calculate average build time from recent builds
    pub fn avg_duration_ms(&self, n: usize) -> u128 {
        let recent = self.recent(n);
        if recent.is_empty() {
            return 0;
        }
        recent.iter().map(|r| r.duration_ms).sum::<u128>() / recent.len() as u128
    }

    /// Calculate average cache hit rate from recent builds
    pub fn avg_cache_hit_rate(&self, n: usize) -> f64 {
        let recent = self.recent(n);
        if recent.is_empty() {
            return 0.0;
        }
        recent.iter().map(|r| r.cache_hit_rate).sum::<f64>() / recent.len() as f64
    }
}

/// Record a build result into history
pub fn record_build(
    root: &PathBuf,
    duration_ms: u128,
    modules_built: usize,
    modules_cached: usize,
    bundle_size: usize,
    mode: &str,
    chunk_count: usize,
    success: bool,
    error: Option<String>,
) -> Result<()> {
    let mut history = BuildHistory::load(root)?;

    let total_modules = modules_built + modules_cached;
    let cache_hit_rate = if total_modules > 0 {
        modules_cached as f64 / total_modules as f64
    } else {
        0.0
    };

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    history.add(BuildRecord {
        timestamp,
        duration_ms,
        modules_built,
        modules_cached,
        bundle_size,
        mode: mode.to_string(),
        cache_hit_rate,
        chunk_count,
        success,
        error,
    });

    history.save(root)?;
    info!("Telemetry: build recorded ({}ms, {} modules)", duration_ms, total_modules);
    Ok(())
}

/// Generate the dashboard HTML
pub fn generate_dashboard_html(history: &BuildHistory) -> String {
    let total_builds = history.builds.len();
    let avg_ms = history.avg_duration_ms(20);
    let avg_cache = history.avg_cache_hit_rate(20);

    let recent: Vec<&BuildRecord> = history.builds.iter().rev().take(20).collect();

    let chart_data: String = recent.iter().rev()
        .map(|r| format!("{{\"x\":{},\"y\":{}}}", r.timestamp, r.duration_ms))
        .collect::<Vec<_>>()
        .join(",");

    let table_rows: String = recent.iter()
        .map(|r| {
            let status = if r.success { "✓" } else { "✗" };
            let status_color = if r.success { "#22c55e" } else { "#ef4444" };
            let date = chrono::DateTime::from_timestamp(r.timestamp as i64, 0)
                .map(|d| d.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "-".to_string());
            format!(
                r#"<tr><td style="color:{};">{}</td><td>{}</td><td>{}ms</td><td>{}</td><td>{}</td><td>{:.0}%</td><td>{:.1}KB</td><td>{}</td></tr>"#,
                status_color, status, date, r.duration_ms, r.modules_built, r.modules_cached,
                r.cache_hit_rate * 100.0, r.bundle_size as f64 / 1024.0, r.chunk_count,
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
    <title>Pledge Telemetry Dashboard</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ background: #0a0a0a; color: #e0e0e0; font-family: ui-monospace, monospace; padding: 2rem; }}
        h1 {{ color: #6366f1; margin-bottom: 1rem; }}
        .stats {{ display: flex; gap: 1rem; margin-bottom: 2rem; flex-wrap: wrap; }}
        .stat {{ background: #1a1a1a; padding: 1rem 1.5rem; border-radius: 8px; border: 1px solid #333; min-width: 140px; }}
        .stat-label {{ color: #888; font-size: 0.75rem; margin-bottom: 0.25rem; text-transform: uppercase; }}
        .stat-value {{ color: #e0e0e0; font-size: 1.5rem; font-weight: 600; }}
        .chart {{ background: #1a1a1a; border-radius: 8px; border: 1px solid #333; padding: 1rem; margin-bottom: 2rem; height: 200px; position: relative; }}
        canvas {{ width: 100%; height: 100%; }}
        table {{ width: 100%; border-collapse: collapse; }}
        th {{ text-align: left; padding: 8px 12px; color: #888; border-bottom: 1px solid #333; font-size: 0.8rem; }}
        td {{ padding: 6px 12px; border-bottom: 1px solid #222; font-size: 0.85rem; }}
        .refresh {{ position: fixed; top: 1rem; right: 1rem; background: #6366f1; color: #fff; border: none; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; }}
    </style>
</head>
<body>
    <button class="refresh" onclick="location.reload()">Refresh</button>
    <h1>Pledge Telemetry Dashboard</h1>
    <div class="stats">
        <div class="stat"><div class="stat-label">Total Builds</div><div class="stat-value">{}</div></div>
        <div class="stat"><div class="stat-label">Avg Duration</div><div class="stat-value">{}ms</div></div>
        <div class="stat"><div class="stat-label">Avg Cache Hit</div><div class="stat-value">{:.0}%</div></div>
    </div>
    <div class="chart"><canvas id="chart"></canvas></div>
    <table>
        <thead><tr><th></th><th>Time</th><th>Duration</th><th>Built</th><th>Cached</th><th>Cache %</th><th>Size</th><th>Chunks</th></tr></thead>
        <tbody>{}</tbody>
    </table>
    <script>
        const data = [{}];
        const canvas = document.getElementById('chart');
        const ctx = canvas.getContext('2d');
        canvas.width = canvas.offsetWidth;
        canvas.height = canvas.offsetHeight;
        if (data.length > 1) {{
            const max = Math.max(...data.map(d => d.y));
            const min = Math.min(...data.map(d => d.y));
            const range = max - min || 1;
            ctx.strokeStyle = '#6366f1';
            ctx.lineWidth = 2;
            ctx.beginPath();
            data.forEach((d, i) => {{
                const x = (i / (data.length - 1)) * canvas.width;
                const y = canvas.height - ((d.y - min) / range) * (canvas.height - 20) - 10;
                if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
            }});
            ctx.stroke();
        }}
    </script>
</body>
</html>"#,
        total_builds, avg_ms, avg_cache * 100.0, table_rows, chart_data,
    )
}
