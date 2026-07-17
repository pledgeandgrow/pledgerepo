// HMR partial update support — line-level diff computation
//
// Instead of sending the full module on every change, we compute a line-level
// diff and send only the changed lines. This significantly reduces WebSocket
// bandwidth for large modules with small edits.
//
// Uses the `similar` crate (Myers diff algorithm) for robust, efficient
// diffing with no line-count limits.

use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

/// A single diff operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffOp {
    /// Lines were added at the given line number
    #[serde(rename = "insert")]
    Insert { line: u32, content: Vec<String> },
    /// Lines were removed at the given line number
    #[serde(rename = "delete")]
    Delete { line: u32, count: u32 },
    /// Lines were replaced at the given line number
    #[serde(rename = "replace")]
    Replace { line: u32, count: u32, content: Vec<String> },
}

/// A line-level diff between two versions of a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDiff {
    pub ops: Vec<DiffOp>,
    /// Total lines in the old version
    pub old_lines: u32,
    /// Total lines in the new version
    pub new_lines: u32,
}

impl LineDiff {
    /// Check if the diff is small enough to be worth sending instead of full code
    pub fn is_small(&self) -> bool {
        // Send diff if it affects less than 30% of lines and has fewer than 10 ops
        if self.ops.len() > 10 {
            return false;
        }
        let changed_lines: u32 = self.ops.iter().map(|op| match op {
            DiffOp::Insert { content, .. } => content.len() as u32,
            DiffOp::Delete { count, .. } => *count,
            DiffOp::Replace { count, content, .. } => (*count).max(content.len() as u32),
        }).sum();
        let max_lines = self.old_lines.max(self.new_lines).max(1);
        changed_lines < (max_lines * 30 / 100)
    }
}

/// Compute a line-level diff between old and new source code.
/// Uses the `similar` crate's Myers diff algorithm for efficient, correct
/// diffing without the previous 200-line cap.
pub fn compute_diff(old: &str, new: &str) -> LineDiff {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let old_len = old_lines.len() as u32;
    let new_len = new_lines.len() as u32;

    // Fast path: identical
    if old == new {
        return LineDiff {
            ops: Vec::new(),
            old_lines: old_len,
            new_lines: new_len,
        };
    }

    // Fast path: one side empty
    if old_lines.is_empty() {
        return LineDiff {
            ops: vec![DiffOp::Insert {
                line: 0,
                content: new_lines.iter().map(|s| s.to_string()).collect(),
            }],
            old_lines: old_len,
            new_lines: new_len,
        };
    }
    if new_lines.is_empty() {
        return LineDiff {
            ops: vec![DiffOp::Delete {
                line: 0,
                count: old_len,
            }],
            old_lines: old_len,
            new_lines: new_len,
        };
    }

    // Use similar's TextDiff for line-level diffing
    let diff = TextDiff::from_lines(old, new);
    let ops = similar_to_diff_ops(&diff);

    LineDiff {
        ops,
        old_lines: old_len,
        new_lines: new_len,
    }
}

/// Convert similar's diff ops into our DiffOp format, coalescing
/// adjacent inserts and deletes into Insert/Delete/Replace operations.
fn similar_to_diff_ops<'a>(diff: &TextDiff<'a, 'a, 'a, str>) -> Vec<DiffOp> {
    let mut ops = Vec::new();
    let mut old_line: u32 = 0;
    let mut pending_inserts: Vec<String> = Vec::new();
    let mut pending_deletes: u32 = 0;
    let mut pending_delete_start: u32 = 0;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                // Flush pending ops at the current position
                flush_pending(
                    &mut ops,
                    &mut pending_inserts,
                    &mut pending_deletes,
                    &mut pending_delete_start,
                );
                old_line += 1;
            }
            ChangeTag::Delete => {
                if pending_deletes == 0 {
                    pending_delete_start = old_line;
                }
                pending_deletes += 1;
                old_line += 1;
            }
            ChangeTag::Insert => {
                pending_inserts.push(change.value().trim_end_matches('\n').to_string());
            }
        }
    }

    // Flush any remaining pending ops
    flush_pending(
        &mut ops,
        &mut pending_inserts,
        &mut pending_deletes,
        &mut pending_delete_start,
    );

    ops
}

/// Flush pending insert/delete operations into a single DiffOp
fn flush_pending(
    ops: &mut Vec<DiffOp>,
    inserts: &mut Vec<String>,
    deletes: &mut u32,
    delete_start: &mut u32,
) {
    if *deletes > 0 && !inserts.is_empty() {
        ops.push(DiffOp::Replace {
            line: *delete_start,
            count: *deletes,
            content: std::mem::take(inserts),
        });
    } else if *deletes > 0 {
        ops.push(DiffOp::Delete {
            line: *delete_start,
            count: *deletes,
        });
    } else if !inserts.is_empty() {
        ops.push(DiffOp::Insert {
            line: *delete_start,
            content: std::mem::take(inserts),
        });
    }
    *deletes = 0;
}
