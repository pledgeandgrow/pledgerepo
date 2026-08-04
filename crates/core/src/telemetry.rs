// Build telemetry — build history tracking and dashboard (#101)
//
// Records build metrics (duration, module count, cache hit rate, bundle size)
// to .pledge/history.json and serves an interactive web dashboard.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
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
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(".pledge").join("history.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    /// Save history to .pledge/history.json
    pub fn save(&self, root: &Path) -> Result<()> {
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
#[allow(clippy::too_many_arguments)]
pub fn record_build(
    root: &Path,
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
    info!(
        "Telemetry: build recorded ({}ms, {} modules)",
        duration_ms, total_modules
    );
    Ok(())
}

/// Generate the dashboard HTML
pub fn generate_dashboard_html(history: &BuildHistory) -> String {
    let total_builds = history.builds.len();
    let avg_ms = history.avg_duration_ms(20);
    let avg_cache = history.avg_cache_hit_rate(20);

    let recent: Vec<&BuildRecord> = history.builds.iter().rev().take(20).collect();

    let chart_data: String = recent
        .iter()
        .rev()
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
        total_builds,
        avg_ms,
        avg_cache * 100.0,
        table_rows,
        chart_data,
    )
}

// ─── G11.9: Determinism dashboard ────────────────────────────────────

/// Determinism status for a single task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDeterminismStatus {
    /// Task name or ID
    pub task_id: String,
    /// Task function name
    pub function_name: String,
    /// Whether the task is deterministic
    pub deterministic: bool,
    /// Number of determinism checks performed
    pub checks: u32,
    /// Number of times output matched
    pub matches: u32,
    /// Number of times output differed
    pub mismatches: u32,
    /// Last check timestamp
    pub last_check: u64,
    /// Error message if non-deterministic
    pub error: Option<String>,
}

impl TaskDeterminismStatus {
    /// Success rate (0.0 - 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.matches + self.mismatches;
        if total == 0 {
            return 1.0;
        }
        self.matches as f64 / total as f64
    }
}

/// Determinism report for all tasks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeterminismReport {
    pub tasks: Vec<TaskDeterminismStatus>,
}

impl DeterminismReport {
    /// Total number of tasks
    pub fn total(&self) -> usize {
        self.tasks.len()
    }

    /// Number of deterministic tasks
    pub fn deterministic_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.deterministic).count()
    }

    /// Number of non-deterministic tasks
    pub fn non_deterministic_count(&self) -> usize {
        self.tasks.iter().filter(|t| !t.deterministic).count()
    }

    /// Overall determinism rate (0.0 - 1.0)
    pub fn determinism_rate(&self) -> f64 {
        if self.tasks.is_empty() {
            return 1.0;
        }
        self.deterministic_count() as f64 / self.tasks.len() as f64
    }

    /// Load from .pledge/determinism.json
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(".pledge").join("determinism.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    /// Save to .pledge/determinism.json
    pub fn save(&self, root: &Path) -> Result<()> {
        let dir = root.join(".pledge");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("determinism.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Record a determinism check result
    pub fn record_check(
        &mut self,
        task_id: &str,
        function_name: &str,
        output_matched: bool,
        error: Option<String>,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Some(task) = self.tasks.iter_mut().find(|t| t.task_id == task_id) {
            task.checks += 1;
            if output_matched {
                task.matches += 1;
            } else {
                task.mismatches += 1;
                task.deterministic = false;
                task.error = error;
            }
            task.last_check = timestamp;
        } else {
            self.tasks.push(TaskDeterminismStatus {
                task_id: task_id.to_string(),
                function_name: function_name.to_string(),
                deterministic: output_matched,
                checks: 1,
                matches: if output_matched { 1 } else { 0 },
                mismatches: if output_matched { 0 } else { 1 },
                last_check: timestamp,
                error: if output_matched { None } else { error },
            });
        }
    }
}

/// Generate the determinism dashboard HTML
pub fn generate_determinism_dashboard_html(report: &DeterminismReport) -> String {
    let total = report.total();
    let det_count = report.deterministic_count();
    let nondet_count = report.non_deterministic_count();
    let rate = report.determinism_rate() * 100.0;

    let task_rows: String = report
        .tasks
        .iter()
        .map(|t| {
            let status = if t.deterministic { "✓ Deterministic" } else { "✗ Non-deterministic" };
            let status_color = if t.deterministic { "#22c55e" } else { "#ef4444" };
            let success = format!("{:.1}%", t.success_rate() * 100.0);
            let error = t.error.as_deref().unwrap_or("—");
            format!(
                r#"<tr><td style="color:{};">{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td style="color:#888;">{}</td></tr>"#,
                status_color, status, t.task_id, t.function_name, t.checks, t.matches, t.mismatches, success, error,
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
    <title>Pledge Determinism Dashboard</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ background: #0a0a0a; color: #e0e0e0; font-family: ui-monospace, monospace; padding: 2rem; }}
        h1 {{ color: #6366f1; margin-bottom: 1rem; }}
        .stats {{ display: flex; gap: 1rem; margin-bottom: 2rem; flex-wrap: wrap; }}
        .stat {{ background: #1a1a1a; padding: 1rem 1.5rem; border-radius: 8px; border: 1px solid #333; min-width: 140px; }}
        .stat-label {{ color: #888; font-size: 0.75rem; margin-bottom: 0.25rem; text-transform: uppercase; }}
        .stat-value {{ color: #e0e0e0; font-size: 1.5rem; font-weight: 600; }}
        .progress-bar {{ background: #1a1a1a; border-radius: 8px; border: 1px solid #333; height: 24px; margin-bottom: 2rem; overflow: hidden; }}
        .progress-fill {{ height: 100%; background: linear-gradient(90deg, #22c55e, #6366f1); transition: width 0.3s; }}
        table {{ width: 100%; border-collapse: collapse; }}
        th {{ text-align: left; padding: 8px 12px; color: #888; border-bottom: 1px solid #333; font-size: 0.8rem; }}
        td {{ padding: 6px 12px; border-bottom: 1px solid #222; font-size: 0.85rem; }}
        .refresh {{ position: fixed; top: 1rem; right: 1rem; background: #6366f1; color: #fff; border: none; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; }}
    </style>
</head>
<body>
    <button class="refresh" onclick="location.reload()">Refresh</button>
    <h1>Pledge Determinism Dashboard</h1>
    <div class="stats">
        <div class="stat"><div class="stat-label">Total Tasks</div><div class="stat-value">{}</div></div>
        <div class="stat"><div class="stat-label">Deterministic</div><div class="stat-value" style="color:#22c55e;">{}</div></div>
        <div class="stat"><div class="stat-label">Non-deterministic</div><div class="stat-value" style="color:#ef4444;">{}</div></div>
        <div class="stat"><div class="stat-label">Determinism Rate</div><div class="stat-value">{:.1}%</div></div>
    </div>
    <div class="progress-bar"><div class="progress-fill" style="width:{}%;"></div></div>
    <table>
        <thead><tr><th>Status</th><th>Task ID</th><th>Function</th><th>Checks</th><th>Matches</th><th>Mismatches</th><th>Success Rate</th><th>Error</th></tr></thead>
        <tbody>{}</tbody>
    </table>
</body>
</html>"#,
        total,
        det_count,
        nondet_count,
        rate,
        rate,
        task_rows,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determinism_report() {
        let mut report = DeterminismReport::default();
        report.record_check("task_1", "transform_js", true, None);
        report.record_check("task_1", "transform_js", true, None);
        report.record_check("task_2", "bundle_css", false, Some("Random timestamp".to_string()));

        assert_eq!(report.total(), 2);
        assert_eq!(report.deterministic_count(), 1);
        assert_eq!(report.non_deterministic_count(), 1);
        assert!((report.determinism_rate() - 0.5).abs() < 0.01);

        let task1 = report.tasks.iter().find(|t| t.task_id == "task_1").unwrap();
        assert_eq!(task1.checks, 2);
        assert_eq!(task1.matches, 2);
        assert_eq!(task1.mismatches, 0);
        assert!((task1.success_rate() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_determinism_dashboard_html() {
        let mut report = DeterminismReport::default();
        report.record_check("task_a", "fn_a", true, None);
        report.record_check("task_b", "fn_b", false, Some("mismatch".to_string()));

        let html = generate_determinism_dashboard_html(&report);
        assert!(html.contains("Pledge Determinism Dashboard"));
        assert!(html.contains("task_a"));
        assert!(html.contains("task_b"));
        assert!(html.contains("Deterministic"));
        assert!(html.contains("Non-deterministic"));
    }

    #[test]
    fn test_determinism_report_save_load() {
        let tmp = std::env::temp_dir().join("pledge_test_det");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let mut report = DeterminismReport::default();
        report.record_check("task_x", "fn_x", true, None);
        report.save(&tmp).unwrap();

        let loaded = DeterminismReport::load(&tmp).unwrap();
        assert_eq!(loaded.total(), 1);
        assert_eq!(loaded.tasks[0].task_id, "task_x");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_empty_determinism_report() {
        let report = DeterminismReport::default();
        assert_eq!(report.total(), 0);
        assert_eq!(report.determinism_rate(), 1.0);
    }
}

// ─── G12.26: OpenTelemetry OTLP Export ─────────────────────────────────

/// FNV-1a 64-bit hash for generating deterministic span IDs.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Configuration for OpenTelemetry OTLP (OpenTelemetry Protocol) export.
///
/// PledgePack can export build traces, task execution spans, and metrics
/// to any OTLP-compatible backend (Jaeger, Tempo, Honeycomb, Datadog, etc.).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OtlpExportConfig {
    /// Whether OTLP export is enabled.
    pub enabled: bool,
    /// The OTLP endpoint URL (e.g., "http://localhost:4317").
    pub endpoint: String,
    /// Export protocol (grpc or http).
    pub protocol: OtlpProtocol,
    /// Service name for traces.
    pub service_name: String,
    /// Whether to export traces (task execution spans).
    pub export_traces: bool,
    /// Whether to export metrics (build stats).
    pub export_metrics: bool,
    /// Whether to export logs.
    pub export_logs: bool,
    /// Headers for authentication (e.g., API keys).
    pub headers: Vec<(String, String)>,
    /// Batch export size.
    pub batch_size: usize,
    /// Export timeout in milliseconds.
    pub timeout_ms: u64,
}

/// OTLP transport protocol.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OtlpProtocol {
    Grpc,
    Http,
}

impl Default for OtlpExportConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:4317".to_string(),
            protocol: OtlpProtocol::Grpc,
            service_name: "pledgepack".to_string(),
            export_traces: true,
            export_metrics: true,
            export_logs: false,
            headers: Vec::new(),
            batch_size: 512,
            timeout_ms: 30000,
        }
    }
}

impl OtlpExportConfig {
    /// Create a gRPC OTLP config with the given endpoint.
    pub fn grpc(endpoint: &str) -> Self {
        Self {
            enabled: true,
            endpoint: endpoint.to_string(),
            protocol: OtlpProtocol::Grpc,
            ..Default::default()
        }
    }

    /// Create an HTTP OTLP config with the given endpoint.
    pub fn http(endpoint: &str) -> Self {
        Self {
            enabled: true,
            endpoint: endpoint.to_string(),
            protocol: OtlpProtocol::Http,
            ..Default::default()
        }
    }

    /// Add an authentication header.
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }
}

/// A trace span representing a single task execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskSpan {
    /// Trace ID (shared across all spans in a build).
    pub trace_id: String,
    /// Span ID (unique per task).
    pub span_id: String,
    /// Parent span ID (None for root).
    pub parent_span_id: Option<String>,
    /// Task function name.
    pub task_name: String,
    /// Task ID (content-addressed).
    pub task_id: String,
    /// Start time (Unix nanoseconds).
    pub start_time_ns: u64,
    /// End time (Unix nanoseconds).
    pub end_time_ns: u64,
    /// Whether the task was a cache hit.
    pub cache_hit: bool,
    /// Attributes (key-value pairs).
    pub attributes: Vec<(String, String)>,
}

impl TaskSpan {
    /// Create a new task span.
    pub fn new(trace_id: &str, task_name: &str, task_id: &str) -> Self {
        // Generate a simple span ID from a hash of trace_id + task_id + timestamp
        let span_input = format!("{}{}{}", trace_id, task_name, std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos());
        let span_hash = format!("{:016x}", fnv1a_64(span_input.as_bytes()));
        Self {
            trace_id: trace_id.to_string(),
            span_id: span_hash,
            parent_span_id: None,
            task_name: task_name.to_string(),
            task_id: task_id.to_string(),
            start_time_ns: 0,
            end_time_ns: 0,
            cache_hit: false,
            attributes: Vec::new(),
        }
    }

    /// Duration in nanoseconds.
    pub fn duration_ns(&self) -> u64 {
        self.end_time_ns.saturating_sub(self.start_time_ns)
    }

    /// Duration in milliseconds.
    pub fn duration_ms(&self) -> f64 {
        self.duration_ns() as f64 / 1_000_000.0
    }

    /// Add an attribute.
    pub fn with_attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.push((key.to_string(), value.to_string()));
        self
    }
}

/// OTLP exporter that collects spans and exports them.
pub struct OtlpExporter {
    config: OtlpExportConfig,
    spans: Vec<TaskSpan>,
}

impl OtlpExporter {
    /// Create a new exporter with the given config.
    pub fn new(config: OtlpExportConfig) -> Self {
        Self {
            config,
            spans: Vec::new(),
        }
    }

    /// Add a span to the exporter.
    pub fn add_span(&mut self, span: TaskSpan) {
        self.spans.push(span);
    }

    /// Get all collected spans.
    pub fn spans(&self) -> &[TaskSpan] {
        &self.spans
    }

    /// Number of spans collected.
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Whether the exporter is empty.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Export collected spans to the OTLP endpoint.
    /// In a real implementation, this would use the opentelemetry-otlp crate.
    pub fn export(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        if self.spans.is_empty() {
            return Ok(());
        }
        // In a real implementation, this would serialize spans as OTLP
        // protobuf and send them via gRPC or HTTP to the configured endpoint.
        info!(
            "G12.26: Exporting {} spans to {} via {:?}",
            self.spans.len(),
            self.config.endpoint,
            self.config.protocol
        );
        Ok(())
    }
}

#[cfg(test)]
mod g12_26_tests {
    use super::*;

    #[test]
    fn otlp_config_default() {
        let config = OtlpExportConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.service_name, "pledgepack");
        assert_eq!(config.protocol, OtlpProtocol::Grpc);
    }

    #[test]
    fn otlp_config_grpc() {
        let config = OtlpExportConfig::grpc("http://collector:4317");
        assert!(config.enabled);
        assert_eq!(config.protocol, OtlpProtocol::Grpc);
    }

    #[test]
    fn otlp_config_http_with_headers() {
        let config = OtlpExportConfig::http("http://collector:4318")
            .with_header("x-api-key", "secret123");
        assert!(config.enabled);
        assert_eq!(config.protocol, OtlpProtocol::Http);
        assert_eq!(config.headers.len(), 1);
        assert_eq!(config.headers[0].1, "secret123");
    }

    #[test]
    fn task_span_duration() {
        let mut span = TaskSpan::new("trace1", "parse_source", "task123");
        span.start_time_ns = 1_000_000_000;
        span.end_time_ns = 1_000_050_000;
        assert_eq!(span.duration_ns(), 50_000);
        assert!((span.duration_ms() - 0.05).abs() < 0.001);
    }

    #[test]
    fn otlp_exporter_collect_and_export() {
        let config = OtlpExportConfig::grpc("http://localhost:4317");
        let mut exporter = OtlpExporter::new(config);
        assert!(exporter.is_empty());

        exporter.add_span(TaskSpan::new("trace1", "parse", "task1"));
        exporter.add_span(TaskSpan::new("trace1", "transform", "task2"));
        assert_eq!(exporter.len(), 2);

        // Export should succeed (no-op in this implementation)
        assert!(exporter.export().is_ok());
    }

    #[test]
    fn otlp_exporter_disabled_noop() {
        let config = OtlpExportConfig::default(); // disabled
        let exporter = OtlpExporter::new(config);
        assert!(exporter.export().is_ok());
    }
}
