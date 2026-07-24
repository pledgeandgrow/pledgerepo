// Bundle size budget checking (#102)
//
// Verifies that build output stays within configured size budgets.
// Exits non-zero on violations. Supports GitHub Actions annotation format.

use crate::config::BudgetConfig;
use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, warn};

/// Budget violation
#[derive(Debug, Clone)]
pub struct BudgetViolation {
    pub field: String,
    pub actual: usize,
    pub limit: usize,
    pub message: String,
}

/// Check build output against configured budgets
pub fn check_budgets(
    _out_dir: &PathBuf,
    config: &BudgetConfig,
    chunk_sizes: &[(String, usize)],
) -> Result<Vec<BudgetViolation>> {
    let mut violations = Vec::new();

    if !config.enabled {
        return Ok(violations);
    }

    // Calculate total bundle size
    let total_size: usize = chunk_sizes.iter().map(|(_, s)| s).sum();

    // Check max total bundle size
    if config.max_bundle_size > 0 && total_size > config.max_bundle_size {
        violations.push(BudgetViolation {
            field: "total bundle size".to_string(),
            actual: total_size,
            limit: config.max_bundle_size,
            message: format!(
                "Total bundle size {} exceeds budget of {}",
                format_bytes(total_size),
                format_bytes(config.max_bundle_size),
            ),
        });
    }

    // Check max per-chunk size
    if config.max_chunk_size > 0 {
        for (name, size) in chunk_sizes {
            if *size > config.max_chunk_size {
                violations.push(BudgetViolation {
                    field: format!("chunk '{}'", name),
                    actual: *size,
                    limit: config.max_chunk_size,
                    message: format!(
                        "Chunk '{}' size {} exceeds per-chunk budget of {}",
                        name,
                        format_bytes(*size),
                        format_bytes(config.max_chunk_size),
                    ),
                });
            }
        }
    }

    // Check max chunk count
    if config.max_chunk_count > 0 && chunk_sizes.len() > config.max_chunk_count {
        violations.push(BudgetViolation {
            field: "chunk count".to_string(),
            actual: chunk_sizes.len(),
            limit: config.max_chunk_count,
            message: format!(
                "Chunk count {} exceeds budget of {}",
                chunk_sizes.len(),
                config.max_chunk_count,
            ),
        });
    }

    // Check per-entry budgets
    for (entry_name, max_size) in &config.entry_budgets {
        let entry_size: usize = chunk_sizes.iter()
            .filter(|(name, _)| name.starts_with(entry_name.as_str()))
            .map(|(_, s)| s)
            .sum();
        if *max_size > 0 && entry_size > *max_size {
            violations.push(BudgetViolation {
                field: format!("entry '{}'", entry_name),
                actual: entry_size,
                limit: *max_size,
                message: format!(
                    "Entry '{}' size {} exceeds budget of {}",
                    entry_name,
                    format_bytes(entry_size),
                    format_bytes(*max_size),
                ),
            });
        }
    }

    if violations.is_empty() {
        info!("Budget check passed: {} chunks, {} total", chunk_sizes.len(), format_bytes(total_size));
    } else {
        warn!("Budget check failed with {} violation(s)", violations.len());
    }

    Ok(violations)
}

/// Format violations as GitHub Actions annotations
pub fn format_github_annotations(violations: &[BudgetViolation]) -> String {
    let mut output = String::new();
    for v in violations {
        output.push_str(&format!(
            "::error file=pledge.config.ts::Budget violation: {} (actual: {}, limit: {})\n",
            v.message,
            format_bytes(v.actual),
            format_bytes(v.limit),
        ));
    }
    output
}

/// Format violations as a PR comment markdown
pub fn format_pr_comment(violations: &[BudgetViolation], chunk_sizes: &[(String, usize)]) -> String {
    let mut md = String::from("## Bundle Size Budget Report\n\n");

    if violations.is_empty() {
        md.push_str("All budgets passed.\n\n");
    } else {
        md.push_str(&format!("**{} violation(s) found.**\n\n", violations.len()));
        for v in violations {
            md.push_str(&format!(
                "- **{}**: {} (limit: {})\n",
                v.field, format_bytes(v.actual), format_bytes(v.limit),
            ));
        }
        md.push('\n');
    }

    md.push_str("### Chunk Sizes\n\n");
    md.push_str("| Chunk | Size |\n|-------|------|\n");
    for (name, size) in chunk_sizes {
        md.push_str(&format!("| {} | {} |\n", name, format_bytes(*size)));
    }

    md
}

/// Format budget violations and chunk sizes as a comfy-table for CLI output
pub fn format_budget_table(violations: &[BudgetViolation], chunk_sizes: &[(String, usize)]) -> String {
    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    if violations.is_empty() {
        table
            .set_header(vec!["Status", "Message"])
            .add_row(vec!["✓", "All budgets within limits"]);
    } else {
        table
            .set_header(vec!["Field", "Actual", "Limit", "Message"])
            .add_rows(
                violations.iter().map(|v| {
                    vec![
                        v.field.clone(),
                        format_bytes(v.actual),
                        format_bytes(v.limit),
                        v.message.clone(),
                    ]
                }),
            );
    }

    // Add chunk sizes section
    let mut chunk_table = comfy_table::Table::new();
    chunk_table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .set_header(vec!["Chunk", "Size"])
        .add_rows(
            chunk_sizes.iter().map(|(name, size)| {
                vec![name.clone(), format_bytes(*size)]
            }),
        );

    format!("{}\n\n{}", table, chunk_table)
}

fn format_bytes(bytes: usize) -> String {
    crate::format_size(bytes)
}
