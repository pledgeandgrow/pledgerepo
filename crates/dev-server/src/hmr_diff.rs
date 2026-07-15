// HMR partial update support — line-level diff computation
//
// Instead of sending the full module on every change, we compute a line-level
// diff and send only the changed lines. This significantly reduces WebSocket
// bandwidth for large modules with small edits.

use serde::{Deserialize, Serialize};

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
/// Uses a simple LCS-based algorithm optimized for small edits.
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

    // Find the common prefix and suffix to narrow the diff window
    let mut prefix = 0;
    while prefix < old_lines.len() && prefix < new_lines.len() && old_lines[prefix] == new_lines[prefix] {
        prefix += 1;
    }

    let mut suffix = 0;
    while suffix < old_lines.len() - prefix
        && suffix < new_lines.len() - prefix
        && old_lines[old_lines.len() - 1 - suffix] == new_lines[new_lines.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let old_mid = &old_lines[prefix..old_lines.len() - suffix];
    let new_mid = &new_lines[prefix..new_lines.len() - suffix];

    let ops = if old_mid.is_empty() {
        vec![DiffOp::Insert {
            line: prefix as u32,
            content: new_mid.iter().map(|s| s.to_string()).collect(),
        }]
    } else if new_mid.is_empty() {
        vec![DiffOp::Delete {
            line: prefix as u32,
            count: old_mid.len() as u32,
        }]
    } else {
        // Use LCS to find the minimal edit script
        lcs_diff(old_mid, new_mid, prefix as u32)
    };

    LineDiff {
        ops,
        old_lines: old_len,
        new_lines: new_len,
    }
}

/// Compute diff using LCS (Longest Common Subsequence) algorithm
fn lcs_diff(old: &[&str], new: &[&str], base_line: u32) -> Vec<DiffOp> {
    let m = old.len();
    let n = new.len();

    // Build LCS table (limited to small windows for performance)
    if m > 200 || n > 200 {
        // For large diffs, just use replace
        return vec![DiffOp::Replace {
            line: base_line,
            count: m as u32,
            content: new.iter().map(|s| s.to_string()).collect(),
        }];
    }

    let mut lcs = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if old[i - 1] == new[j - 1] {
                lcs[i][j] = lcs[i - 1][j - 1] + 1;
            } else {
                lcs[i][j] = lcs[i - 1][j].max(lcs[i][j - 1]);
            }
        }
    }

    // Backtrack to build the edit script
    let mut ops = Vec::new();
    let mut i = m;
    let mut j = n;

    // We build ops in reverse, then reverse at the end
    let mut pending_inserts: Vec<String> = Vec::new();
    let mut pending_deletes: u32 = 0;
    let mut insert_at: u32 = 0;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
            // Match — flush pending ops
            if pending_deletes > 0 || !pending_inserts.is_empty() {
                let content = pending_inserts.drain(..).rev().collect::<Vec<_>>();
                if pending_deletes > 0 && !content.is_empty() {
                    ops.push(DiffOp::Replace {
                        line: insert_at,
                        count: pending_deletes,
                        content,
                    });
                } else if pending_deletes > 0 {
                    ops.push(DiffOp::Delete {
                        line: insert_at,
                        count: pending_deletes,
                    });
                } else if !content.is_empty() {
                    ops.push(DiffOp::Insert {
                        line: insert_at,
                        content,
                    });
                }
                pending_deletes = 0;
            }
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || lcs[i][j - 1] >= lcs[i - 1][j]) {
            // Insert
            if pending_inserts.is_empty() {
                insert_at = base_line + i as u32;
            }
            pending_inserts.push(new[j - 1].to_string());
            j -= 1;
        } else {
            // Delete
            if pending_deletes == 0 {
                insert_at = base_line + (i - 1) as u32;
            }
            pending_deletes += 1;
            i -= 1;
        }
    }

    // Flush remaining pending ops
    if pending_deletes > 0 || !pending_inserts.is_empty() {
        let content = pending_inserts.drain(..).rev().collect::<Vec<_>>();
        if pending_deletes > 0 && !content.is_empty() {
            ops.push(DiffOp::Replace {
                line: insert_at,
                count: pending_deletes,
                content,
            });
        } else if pending_deletes > 0 {
            ops.push(DiffOp::Delete {
                line: insert_at,
                count: pending_deletes,
            });
        } else if !content.is_empty() {
            ops.push(DiffOp::Insert {
                line: insert_at,
                content,
            });
        }
    }

    ops.reverse();
    ops
}
