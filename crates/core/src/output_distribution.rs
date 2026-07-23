// Output & Distribution features
//
// Features:
//   43. Service worker generation (delegates to service_worker.rs)
//   44. Web App Manifest generation (delegates to service_worker.rs)
//   45. Performance budget enforcement
//   46. Bundle size diff
//   47. Source map explorer
//   48. Multi-format output — ESM/CJS/IIFE

use std::collections::HashMap;
use std::path::Path;

/// Compile asset budget patterns into a GlobSet for efficient matching.
/// Returns (globset, fallback_map) so callers can match asset paths against
/// both glob patterns and plain string keys.
pub fn compile_asset_budget_globset(
    asset_budgets: &HashMap<String, usize>,
) -> globset::GlobSet {
    let mut builder = globset::GlobSetBuilder::new();
    for key in asset_budgets.keys() {
        if let Ok(glob) = globset::Glob::new(key) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_default()
}

// ─── Feature 45: Performance budget enforcement ───────────────────────

/// Performance budget configuration
#[derive(Debug, Clone)]
pub struct PerformanceBudget {
    /// Maximum total bundle size in bytes
    pub max_total_size: Option<usize>,
    /// Maximum size per entry chunk in bytes
    pub max_entry_size: Option<usize>,
    /// Maximum size per chunk in bytes
    pub max_chunk_size: Option<usize>,
    /// Maximum size for initial load (entry + imported chunks) in bytes
    pub max_initial_size: Option<usize>,
    /// Per-asset-type budgets (key: asset type, value: max bytes)
    pub asset_budgets: HashMap<String, usize>,
    /// Number of warnings allowed before failing
    pub max_warnings: usize,
}

impl Default for PerformanceBudget {
    fn default() -> Self {
        Self {
            max_total_size: Some(2 * 1024 * 1024), // 2MB
            max_entry_size: Some(500 * 1024),       // 500KB
            max_chunk_size: Some(300 * 1024),       // 300KB
            max_initial_size: Some(1 * 1024 * 1024), // 1MB
            asset_budgets: HashMap::new(),
            max_warnings: 0,
        }
    }
}

/// Budget violation
#[derive(Debug, Clone)]
pub struct BudgetViolation {
    pub category: BudgetCategory,
    pub actual: usize,
    pub limit: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetCategory {
    Total,
    Entry,
    Chunk,
    Initial,
    Asset,
}

impl BudgetCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Total => "total",
            Self::Entry => "entry",
            Self::Chunk => "chunk",
            Self::Initial => "initial",
            Self::Asset => "asset",
        }
    }
}

/// Result of budget check
#[derive(Debug)]
pub struct BudgetCheckResult {
    pub violations: Vec<BudgetViolation>,
    pub warnings: Vec<BudgetViolation>,
    pub passed: bool,
}

/// Check bundle sizes against performance budget
pub fn check_budget(
    entries: &[(String, usize)],  // (entry name, size in bytes)
    chunks: &[(String, usize)],   // (chunk name, size in bytes)
    assets: &[(String, String, usize)], // (asset name, type, size in bytes)
    budget: &PerformanceBudget,
) -> BudgetCheckResult {
    let mut violations = Vec::new();
    let mut warnings = Vec::new();

    // Check total size
    let total: usize = entries.iter().map(|(_, s)| s).sum::<usize>()
        + chunks.iter().map(|(_, s)| s).sum::<usize>();
    if let Some(max_total) = budget.max_total_size {
        if total > max_total {
            violations.push(BudgetViolation {
                category: BudgetCategory::Total,
                actual: total,
                limit: max_total,
                message: format!(
                    "Total bundle size {} exceeds budget {}",
                    format_bytes(total),
                    format_bytes(max_total)
                ),
            });
        }
    }

    // Check entry sizes
    if let Some(max_entry) = budget.max_entry_size {
        for (name, size) in entries {
            if *size > max_entry {
                violations.push(BudgetViolation {
                    category: BudgetCategory::Entry,
                    actual: *size,
                    limit: max_entry,
                    message: format!(
                        "Entry '{}' size {} exceeds budget {}",
                        name,
                        format_bytes(*size),
                        format_bytes(max_entry)
                    ),
                });
            }
        }
    }

    // Check chunk sizes
    if let Some(max_chunk) = budget.max_chunk_size {
        for (name, size) in chunks {
            if *size > max_chunk {
                violations.push(BudgetViolation {
                    category: BudgetCategory::Chunk,
                    actual: *size,
                    limit: max_chunk,
                    message: format!(
                        "Chunk '{}' size {} exceeds budget {}",
                        name,
                        format_bytes(*size),
                        format_bytes(max_chunk)
                    ),
                });
            }
        }
    }

    // Check initial load size (first entry + its chunks)
    if let Some(max_initial) = budget.max_initial_size {
        if let Some((_, entry_size)) = entries.first() {
            let initial_total = entry_size + chunks.iter().map(|(_, s)| s).sum::<usize>();
            if initial_total > max_initial {
                violations.push(BudgetViolation {
                    category: BudgetCategory::Initial,
                    actual: initial_total,
                    limit: max_initial,
                    message: format!(
                        "Initial load size {} exceeds budget {}",
                        format_bytes(initial_total),
                        format_bytes(max_initial)
                    ),
                });
            }
        }
    }

    // Check asset-type budgets
    for (name, asset_type, size) in assets {
        if let Some(max_asset) = budget.asset_budgets.get(asset_type) {
            if *size > *max_asset {
                violations.push(BudgetViolation {
                    category: BudgetCategory::Asset,
                    actual: *size,
                    limit: *max_asset,
                    message: format!(
                        "Asset '{}' ({}) size {} exceeds budget {}",
                        name,
                        asset_type,
                        format_bytes(*size),
                        format_bytes(*max_asset)
                    ),
                });
            }
        }
    }

    let passed = violations.len() <= budget.max_warnings;
    BudgetCheckResult {
        violations,
        warnings,
        passed,
    }
}

fn format_bytes(bytes: usize) -> String {
    crate::format_size(bytes)
}

// ─── Feature 46: Bundle size diff ─────────────────────────────────────

/// Bundle size snapshot for comparison
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BundleSizeSnapshot {
    pub timestamp: String,
    pub git_sha: Option<String>,
    pub entries: Vec<BundleEntry>,
    pub total_size: usize,
    pub total_gzip_size: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BundleEntry {
    pub name: String,
    pub size: usize,
    pub gzip_size: usize,
}

/// Diff between two bundle size snapshots
#[derive(Debug)]
pub struct BundleSizeDiff {
    pub added: Vec<BundleEntry>,
    pub removed: Vec<BundleEntry>,
    pub changed: Vec<BundleEntryDiff>,
    pub total_size_delta: i64,
    pub total_gzip_delta: i64,
    pub old_total_size: usize,
    pub new_total_size: usize,
    pub has_regressions: bool,
}

#[derive(Debug)]
pub struct BundleEntryDiff {
    pub name: String,
    pub old_size: usize,
    pub new_size: usize,
    pub size_delta: i64,
    pub gzip_delta: i64,
    pub is_regression: bool,
}

/// Compare two bundle size snapshots
pub fn diff_snapshots(
    old: &BundleSizeSnapshot,
    new: &BundleSizeSnapshot,
    regression_threshold: usize, // bytes
) -> BundleSizeDiff {
    let old_map: HashMap<&str, &BundleEntry> = old
        .entries
        .iter()
        .map(|e| (e.name.as_str(), e))
        .collect();
    let new_map: HashMap<&str, &BundleEntry> = new
        .entries
        .iter()
        .map(|e| (e.name.as_str(), e))
        .collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    // Find added and changed entries
    for entry in &new.entries {
        match old_map.get(entry.name.as_str()) {
            None => added.push(entry.clone()),
            Some(old_entry) => {
                if entry.size != old_entry.size {
                    let size_delta = entry.size as i64 - old_entry.size as i64;
                    let gzip_delta = entry.gzip_size as i64 - old_entry.gzip_size as i64;
                    changed.push(BundleEntryDiff {
                        name: entry.name.clone(),
                        old_size: old_entry.size,
                        new_size: entry.size,
                        size_delta,
                        gzip_delta,
                        is_regression: size_delta as usize > regression_threshold,
                    });
                }
            }
        }
    }

    // Find removed entries
    for entry in &old.entries {
        if !new_map.contains_key(entry.name.as_str()) {
            removed.push(entry.clone());
        }
    }

    let total_size_delta = new.total_size as i64 - old.total_size as i64;
    let total_gzip_delta = new.total_gzip_size as i64 - old.total_gzip_size as i64;
    let has_regressions = changed.iter().any(|c| c.is_regression) || total_size_delta > regression_threshold as i64;

    BundleSizeDiff {
        added,
        removed,
        changed,
        total_size_delta,
        total_gzip_delta,
        old_total_size: old.total_size,
        new_total_size: new.total_size,
        has_regressions,
    }
}

/// Generate a human-readable diff report
pub fn format_diff_report(diff: &BundleSizeDiff) -> String {
    let mut report = String::new();

    report.push_str("# Bundle Size Diff\n\n");

    if diff.total_size_delta != 0 {
        let pct = if diff.old_total_size > 0 { diff.total_size_delta as f64 / diff.old_total_size as f64 * 100.0 } else { 0.0 };
        let sign = if diff.total_size_delta > 0 { "+" } else { "" };
        report.push_str(&format!(
            "**Total size:** {}{} bytes ({:+.2}%)\n\n",
            sign,
            diff.total_size_delta,
            pct
        ));
    }

    if !diff.added.is_empty() {
        report.push_str("## Added\n\n");
        for entry in &diff.added {
            report.push_str(&format!("- **{}**: {} bytes\n", entry.name, entry.size));
        }
        report.push('\n');
    }

    if !diff.removed.is_empty() {
        report.push_str("## Removed\n\n");
        for entry in &diff.removed {
            report.push_str(&format!("- **{}**: {} bytes\n", entry.name, entry.size));
        }
        report.push('\n');
    }

    if !diff.changed.is_empty() {
        report.push_str("## Changed\n\n");
        for entry in &diff.changed {
            let sign = if entry.size_delta > 0 { "+" } else { "" };
            let marker = if entry.is_regression { " ⚠️" } else { "" };
            report.push_str(&format!(
                "- **{}**: {} → {} bytes ({}{}){}\n",
                entry.name,
                entry.old_size,
                entry.new_size,
                sign,
                entry.size_delta,
                marker
            ));
        }
    }

    if diff.has_regressions {
        report.push_str("\n**⚠️ Size regressions detected!**\n");
    } else {
        report.push_str("\n**✅ No size regressions detected.**\n");
    }

    report
}

// ─── Feature 47: Source map explorer ──────────────────────────────────

/// Module contribution to a bundle
#[derive(Debug, Clone)]
pub struct ModuleContribution {
    pub module_id: String,
    pub file_path: String,
    pub size: usize,
    pub source_map_size: usize,
    pub children: Vec<ModuleContribution>,
}

/// Build a treemap of module contributions from a source map
pub fn build_source_map_tree(
    source_map: &str,
    module_sizes: &HashMap<String, usize>,
) -> ModuleContribution {
    let map: serde_json::Value = serde_json::from_str(source_map).unwrap_or(serde_json::json!({}));

    let sources = map["sources"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut root = ModuleContribution {
        module_id: "root".to_string(),
        file_path: "/".to_string(),
        size: 0,
        source_map_size: source_map.len(),
        children: Vec::new(),
    };

    for source in &sources {
        let size = module_sizes.get(source).copied().unwrap_or(0);
        let parts: Vec<&str> = source.split('/').filter(|s| !s.is_empty()).collect();
        insert_into_tree(&mut root, &parts, source, size);
    }

    root
}

/// Recursively insert a source path into the tree
fn insert_into_tree(node: &mut ModuleContribution, parts: &[&str], full_path: &str, size: usize) {
    node.size += size;

    if parts.is_empty() {
        return;
    }

    let first = parts[0];
    let child_idx = node.children.iter().position(|c| c.module_id == first);

    match child_idx {
        Some(idx) => {
            insert_into_tree(&mut node.children[idx], &parts[1..], full_path, size);
        }
        None => {
            let mut child = ModuleContribution {
                module_id: first.to_string(),
                file_path: if parts.len() == 1 { full_path.to_string() } else { first.to_string() },
                size: 0,
                source_map_size: 0,
                children: Vec::new(),
            };
            insert_into_tree(&mut child, &parts[1..], full_path, size);
            node.children.push(child);
        }
    }
}

/// Generate an HTML treemap visualization of source map contributions
pub fn generate_explorer_html(tree: &ModuleContribution) -> String {
    let json_data = serialize_tree_to_json(tree);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Source Map Explorer</title>
  <style>
    body {{ margin: 0; font-family: -apple-system, sans-serif; }}
    #treemap {{ width: 100vw; height: 100vh; }}
    .treemap-cell {{ border: 1px solid #fff; display: flex; align-items: center; justify-content: center; overflow: hidden; }}
    .treemap-label {{ font-size: 12px; color: #fff; text-align: center; padding: 4px; }}
  </style>
</head>
<body>
  <div id="treemap"></div>
  <script>
    const data = {json};
    // Simple treemap rendering
    function renderTreemap(node, container) {{
      if (!node.children || node.children.length === 0) return;
      const total = node.size || 1;
      let offset = 0;
      node.children.sort((a, b) => b.size - a.size);
      for (const child of node.children) {{
        const pct = (child.size / total) * 100;
        const div = document.createElement('div');
        div.className = 'treemap-cell';
        div.style.width = pct + '%';
        div.style.height = '100%';
        div.style.float = 'left';
        div.style.background = getColor(child.size);
        div.innerHTML = '<div class="treemap-label">' + child.name + '<br>' + formatBytes(child.size) + '</div>';
        container.appendChild(div);
        renderTreemap(child, div);
      }}
    }}
    function formatBytes(b) {{
      if (b > 1048576) return (b/1048576).toFixed(1) + 'MB';
      if (b > 1024) return (b/1024).toFixed(1) + 'KB';
      return b + 'B';
    }}
    function getColor(size) {{
      const hue = Math.max(0, 200 - size / 1024);
      return 'hsl(' + hue + ', 70%, 50%)';
    }}
    renderTreemap(data, document.getElementById('treemap'));
  </script>
</body>
</html>"#,
        json = json_data
    )
}

fn serialize_tree_to_json(tree: &ModuleContribution) -> String {
    fn node_to_json(n: &ModuleContribution) -> serde_json::Value {
        serde_json::json!({
            "name": n.module_id,
            "size": n.size,
            "children": n.children.iter().map(node_to_json).collect::<Vec<_>>()
        })
    }
    serde_json::to_string(&node_to_json(tree)).unwrap_or_else(|_| "{}".to_string())
}

// ─── Feature 48: Multi-format output ──────────────────────────────────

/// Output format for library mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormatType {
    ESM,
    CJS,
    IIFE,
    UMD,
}

impl OutputFormatType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ESM => "esm",
            Self::CJS => "cjs",
            Self::IIFE => "iife",
            Self::UMD => "umd",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::ESM => "mjs",
            Self::CJS => "cjs",
            Self::IIFE => "js",
            Self::UMD => "umd.js",
        }
    }
}

/// Configuration for multi-format output
#[derive(Debug, Clone)]
pub struct MultiFormatConfig {
    pub formats: Vec<OutputFormatType>,
    pub global_name: Option<String>, // For IIFE/UMD
    pub exports: Vec<String>,        // Named exports to include
}

impl Default for MultiFormatConfig {
    fn default() -> Self {
        Self {
            formats: vec![OutputFormatType::ESM, OutputFormatType::CJS],
            global_name: None,
            exports: Vec::new(),
        }
    }
}

/// Result of multi-format output generation
pub struct MultiFormatOutput {
    pub format: OutputFormatType,
    pub code: String,
    pub file_name: String,
}

/// Generate multi-format output from ESM source
pub fn generate_multi_format(
    esm_source: &str,
    module_name: &str,
    config: &MultiFormatConfig,
) -> Vec<MultiFormatOutput> {
    config
        .formats
        .iter()
        .map(|&format| {
            let code = match format {
                OutputFormatType::ESM => esm_source.to_string(),
                OutputFormatType::CJS => convert_esm_to_cjs(esm_source, module_name),
                OutputFormatType::IIFE => convert_esm_to_iife(
                    esm_source,
                    config.global_name.as_deref().unwrap_or(module_name),
                ),
                OutputFormatType::UMD => convert_esm_to_umd(
                    esm_source,
                    config.global_name.as_deref().unwrap_or(module_name),
                ),
            };

            let file_name = format!("{}.{}", module_name, format.extension());

            MultiFormatOutput {
                format,
                code,
                file_name,
            }
        })
        .collect()
}

/// Convert ESM source to CommonJS
fn convert_esm_to_cjs(source: &str, _module_name: &str) -> String {
    let mut result = String::new();
    result.push_str("\"use strict\";\n");
    result.push_str("Object.defineProperty(exports, \"__esModule\", { value: true });\n\n");

    let mut export_names = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();

        // Convert: import { foo } from './bar' → const { foo } = require('./bar')
        // Convert: import defaultExport from './bar' → const defaultExport = require('./bar')
        if trimmed.starts_with("import ") && trimmed.contains(" from ") {
            let converted = trimmed
                .replace("import ", "const ")
                .replace(" from ", " = require(")
                + ")";
            result.push_str(&converted);
            result.push('\n');
            continue;
        }

        // Convert: import 'foo' (side-effect only) → require('foo')
        if trimmed.starts_with("import ") && !trimmed.contains(" from ") {
            let mut converted = trimmed.replace("import ", "require(");
            if !converted.ends_with(')') {
                converted.push(')');
            }
            result.push_str(&converted);
            result.push('\n');
            continue;
        }

        // Convert: export const foo = ... → const foo = ...; exports.foo = foo
        if let Some(rest) = trimmed.strip_prefix("export const ") {
            let name = rest.split(|c: char| c == '=' || c.is_whitespace()).next().unwrap_or("");
            if !name.is_empty() {
                export_names.push(name.to_string());
            }
            result.push_str(&trimmed.replace("export ", ""));
            result.push('\n');
            continue;
        }

        // Convert: export function foo() → function foo(); exports.foo = foo
        if let Some(rest) = trimmed.strip_prefix("export function ") {
            let name = rest.split(|c: char| c == '(' || c.is_whitespace()).next().unwrap_or("");
            if !name.is_empty() {
                export_names.push(name.to_string());
            }
            result.push_str(&trimmed.replace("export ", ""));
            result.push('\n');
            continue;
        }

        // Convert: export class Foo → class Foo; exports.Foo = Foo
        if let Some(rest) = trimmed.strip_prefix("export class ") {
            let name = rest.split(|c: char| c == '{' || c.is_whitespace()).next().unwrap_or("");
            if !name.is_empty() {
                export_names.push(name.to_string());
            }
            result.push_str(&trimmed.replace("export ", ""));
            result.push('\n');
            continue;
        }

        // Convert: export { foo, bar } → exports.foo = foo; exports.bar = bar
        if let Some(rest) = trimmed.strip_prefix("export { ") {
            let names_str = rest.trim_end_matches(" }").trim_end_matches(";");
            for name in names_str.split(',') {
                let name = name.trim();
                if !name.is_empty() {
                    export_names.push(name.to_string());
                }
            }
            continue;
        }

        // Convert: export default ... → module.exports = ...
        if let Some(rest) = trimmed.strip_prefix("export default ") {
            result.push_str(&format!("module.exports = {};\n", rest));
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    // Add exports assignments
    if !export_names.is_empty() {
        result.push_str("\n");
        for name in &export_names {
            result.push_str(&format!("exports.{} = {};\n", name, name));
        }
    }

    result
}

/// Convert ESM source to IIFE (Immediately Invoked Function Expression)
fn convert_esm_to_iife(source: &str, global_name: &str) -> String {
    // First convert to CJS-like, then wrap in IIFE
    let cjs_body = convert_esm_to_cjs(source, global_name);

    format!(
        r#"var {global_name} = (function() {{
  var module = {{ exports: {{}} }};
  var exports = module.exports;
{cjs_body}
  return module.exports;
}})();
"#,
        global_name = global_name,
        cjs_body = cjs_body
            .lines()
            .map(|l| format!("  {}", l))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// Convert ESM source to UMD (Universal Module Definition)
fn convert_esm_to_umd(source: &str, global_name: &str) -> String {
    let iife_body = convert_esm_to_iife(source, global_name);

    format!(
        r#"(function (root, factory) {{
  if (typeof define === 'function' && define.amd) {{
    define([], factory);
  }} else if (typeof module === 'object' && module.exports) {{
    module.exports = factory();
  }} else {{
    root.{global_name} = factory();
  }}
}})(typeof self !== 'undefined' ? self : this, function() {{
{iife_body}
  return {global_name};
}});
"#,
        global_name = global_name,
        iife_body = iife_body
            .lines()
            .map(|l| format!("  {}", l))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_check_pass() {
        let entries = vec![("main.js".to_string(), 100_000)];
        let chunks = vec![("vendor.js".to_string(), 200_000)];
        let assets = vec![];
        let budget = PerformanceBudget::default();

        let result = check_budget(&entries, &chunks, &assets, &budget);
        assert!(result.passed);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_budget_check_fail() {
        let entries = vec![("main.js".to_string(), 600_000)]; // exceeds 500KB entry limit
        let chunks = vec![];
        let assets = vec![];
        let budget = PerformanceBudget::default();

        let result = check_budget(&entries, &chunks, &assets, &budget);
        assert!(!result.passed);
        assert!(result.violations.iter().any(|v| v.category == BudgetCategory::Entry));
    }

    #[test]
    fn test_budget_check_total() {
        let entries = vec![("a.js".to_string(), 2 * 1024 * 1024)];
        let chunks = vec![("b.js".to_string(), 1 * 1024 * 1024)];
        let budget = PerformanceBudget::default();

        let result = check_budget(&entries, &chunks, &[], &budget);
        assert!(result.violations.iter().any(|v| v.category == BudgetCategory::Total));
    }

    #[test]
    fn test_bundle_diff_no_changes() {
        let old = BundleSizeSnapshot {
            timestamp: "2024-01-01".to_string(),
            git_sha: None,
            entries: vec![BundleEntry {
                name: "main.js".to_string(),
                size: 1000,
                gzip_size: 300,
            }],
            total_size: 1000,
            total_gzip_size: 300,
        };
        let new = old.clone();
        let diff = diff_snapshots(&old, &new, 100);
        assert!(!diff.has_regressions);
        assert!(diff.changed.is_empty());
    }

    #[test]
    fn test_bundle_diff_regression() {
        let old = BundleSizeSnapshot {
            timestamp: "2024-01-01".to_string(),
            git_sha: None,
            entries: vec![BundleEntry {
                name: "main.js".to_string(),
                size: 1000,
                gzip_size: 300,
            }],
            total_size: 1000,
            total_gzip_size: 300,
        };
        let new = BundleSizeSnapshot {
            timestamp: "2024-01-02".to_string(),
            git_sha: None,
            entries: vec![BundleEntry {
                name: "main.js".to_string(),
                size: 2000,
                gzip_size: 600,
            }],
            total_size: 2000,
            total_gzip_size: 600,
        };
        let diff = diff_snapshots(&old, &new, 100);
        assert!(diff.has_regressions);
        assert_eq!(diff.changed.len(), 1);
        assert!(diff.changed[0].is_regression);
    }

    #[test]
    fn test_bundle_diff_report() {
        let old = BundleSizeSnapshot {
            timestamp: "2024-01-01".to_string(),
            git_sha: None,
            entries: vec![BundleEntry {
                name: "main.js".to_string(),
                size: 1000,
                gzip_size: 300,
            }],
            total_size: 1000,
            total_gzip_size: 300,
        };
        let new = BundleSizeSnapshot {
            timestamp: "2024-01-02".to_string(),
            git_sha: None,
            entries: vec![BundleEntry {
                name: "main.js".to_string(),
                size: 1500,
                gzip_size: 450,
            }],
            total_size: 1500,
            total_gzip_size: 450,
        };
        let diff = diff_snapshots(&old, &new, 100);
        let report = format_diff_report(&diff);
        assert!(report.contains("Bundle Size Diff"));
        assert!(report.contains("main.js"));
    }

    #[test]
    fn test_source_map_tree() {
        let source_map = r#"{"version":3,"sources":["src/index.ts","src/utils.ts"],"mappings":""}"#;
        let mut sizes = HashMap::new();
        sizes.insert("src/index.ts".to_string(), 5000);
        sizes.insert("src/utils.ts".to_string(), 3000);

        let tree = build_source_map_tree(source_map, &sizes);
        assert!(tree.size > 0);
        assert!(!tree.children.is_empty());
    }

    #[test]
    fn test_explorer_html() {
        let tree = ModuleContribution {
            module_id: "root".to_string(),
            file_path: "/".to_string(),
            size: 10000,
            source_map_size: 1000,
            children: vec![ModuleContribution {
                module_id: "index.ts".to_string(),
                file_path: "src/index.ts".to_string(),
                size: 5000,
                source_map_size: 500,
                children: Vec::new(),
            }],
        };
        let html = generate_explorer_html(&tree);
        assert!(html.contains("<html"));
        assert!(html.contains("Source Map Explorer"));
    }

    #[test]
    fn test_multi_format_esm() {
        let source = "export const foo = 42;";
        let config = MultiFormatConfig {
            formats: vec![OutputFormatType::ESM],
            ..Default::default()
        };
        let outputs = generate_multi_format(source, "mylib", &config);
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].format, OutputFormatType::ESM);
        assert!(outputs[0].code.contains("export const foo"));
    }

    #[test]
    fn test_multi_format_cjs() {
        let source = "export const foo = 42;\nexport function bar() { return 1; }";
        let config = MultiFormatConfig {
            formats: vec![OutputFormatType::CJS],
            ..Default::default()
        };
        let outputs = generate_multi_format(source, "mylib", &config);
        assert_eq!(outputs.len(), 1);
        let cjs = &outputs[0].code;
        assert!(cjs.contains("\"use strict\""));
        assert!(cjs.contains("exports.foo"));
        assert!(cjs.contains("exports.bar"));
    }

    #[test]
    fn test_multi_format_iife() {
        let source = "export const foo = 42;";
        let config = MultiFormatConfig {
            formats: vec![OutputFormatType::IIFE],
            global_name: Some("MyLib".to_string()),
            ..Default::default()
        };
        let outputs = generate_multi_format(source, "mylib", &config);
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].code.contains("MyLib"));
        assert!(outputs[0].code.contains("(function()"));
    }

    #[test]
    fn test_multi_format_all() {
        let source = "export const foo = 42;";
        let config = MultiFormatConfig {
            formats: vec![
                OutputFormatType::ESM,
                OutputFormatType::CJS,
                OutputFormatType::IIFE,
                OutputFormatType::UMD,
            ],
            global_name: Some("MyLib".to_string()),
            ..Default::default()
        };
        let outputs = generate_multi_format(source, "mylib", &config);
        assert_eq!(outputs.len(), 4);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        // humansize BINARY format uses space and KiB/MiB suffixes
        assert!(format_bytes(1024).contains("KiB"));
        assert!(format_bytes(1024 * 1024).contains("MiB"));
    }
}
