// Performance regression detection (#103)
//
// Compares build times against a baseline. The `pledge bench --baseline <ref>`
// flag loads historical benchmark data and warns when build time increases
// beyond a configured threshold.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

/// Benchmark result for a single run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchResult {
    /// Git commit hash or ref name
    pub ref_name: String,
    /// Unix timestamp
    pub timestamp: u64,
    /// Build duration in milliseconds
    pub duration_ms: u128,
    /// Number of modules
    pub modules: usize,
    /// Number of cached modules
    pub cached: usize,
}

/// Stored benchmark baseline
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchBaseline {
    pub results: Vec<BenchResult>,
}

impl BenchBaseline {
    /// Load baseline from .pledge/bench.json
    pub fn load(root: &PathBuf) -> Result<Self> {
        let path = root.join(".pledge").join("bench.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    /// Save baseline to .pledge/bench.json
    pub fn save(&self, root: &PathBuf) -> Result<()> {
        let dir = root.join(".pledge");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("bench.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Get baseline result for a specific ref
    pub fn get_baseline(&self, ref_name: &str) -> Option<&BenchResult> {
        self.results.iter().find(|r| r.ref_name == ref_name)
    }

    /// Add a benchmark result
    pub fn add(&mut self, result: BenchResult) {
        // Remove existing entry with same ref_name
        self.results.retain(|r| r.ref_name != result.ref_name);
        self.results.push(result);
    }
}

/// Compare current build time against baseline and detect regressions
pub fn detect_regression(
    current_ms: u128,
    baseline_ms: u128,
    threshold_pct: f64,
) -> Option<RegressionReport> {
    if baseline_ms == 0 {
        return None;
    }

    let diff_ms = current_ms as i128 - baseline_ms as i128;
    let pct_change = (diff_ms as f64 / baseline_ms as f64) * 100.0;

    if pct_change > threshold_pct {
        Some(RegressionReport {
            current_ms,
            baseline_ms,
            diff_ms: diff_ms as u128,
            pct_change,
            threshold_pct,
            is_regression: true,
        })
    } else {
        None
    }
}

/// Regression analysis report
#[derive(Debug, Clone)]
pub struct RegressionReport {
    pub current_ms: u128,
    pub baseline_ms: u128,
    pub diff_ms: u128,
    pub pct_change: f64,
    pub threshold_pct: f64,
    pub is_regression: bool,
}

impl RegressionReport {
    /// Format as a human-readable string using comfy-table
    pub fn format(&self) -> String {
        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
            .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
            .set_header(vec!["Metric", "Value"])
            .add_row(vec!["Baseline", &format!("{}ms", self.baseline_ms)])
            .add_row(vec!["Current", &format!("{}ms", self.current_ms)])
            .add_row(vec!["Change", &format!("+{}ms", self.diff_ms)])
            .add_row(vec!["% Change", &format!("{:.1}%", self.pct_change)])
            .add_row(vec!["Threshold", &format!("{:.1}%", self.threshold_pct)]);
        format!("  \x1b[31m⚠ Performance regression\x1b[0m\n{}", table)
    }
}

/// Record a benchmark result
pub fn record_bench(
    root: &PathBuf,
    ref_name: &str,
    duration_ms: u128,
    modules: usize,
    cached: usize,
) -> Result<()> {
    let mut baseline = BenchBaseline::load(root)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    baseline.add(BenchResult {
        ref_name: ref_name.to_string(),
        timestamp,
        duration_ms,
        modules,
        cached,
    });

    baseline.save(root)?;
    info!("Benchmark recorded: {} ({}ms)", ref_name, duration_ms);
    Ok(())
}

/// Compare current run against baseline ref
pub fn compare_with_baseline(
    root: &PathBuf,
    baseline_ref: &str,
    current_ms: u128,
    threshold_pct: f64,
) -> Result<Option<RegressionReport>> {
    let baseline = BenchBaseline::load(root)?;

    if let Some(base) = baseline.get_baseline(baseline_ref) {
        let report = detect_regression(current_ms, base.duration_ms, threshold_pct);
        if let Some(ref r) = report {
            warn!("{}", r.format());
        } else {
            info!("No regression detected: {}ms vs baseline {}ms", current_ms, base.duration_ms);
        }
        Ok(report)
    } else {
        info!("No baseline found for ref '{}', skipping comparison", baseline_ref);
        Ok(None)
    }
}
