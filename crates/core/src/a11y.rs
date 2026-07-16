// Accessibility linting during build (#108)
//
// Checks for common accessibility issues in HTML output:
// - Missing alt attributes on images
// - Insufficient color contrast (basic check)
// - Missing ARIA labels on interactive elements

use crate::config::A11yConfig;
use anyhow::Result;
use tracing::{info, warn};

/// An a11y violation
#[derive(Debug, Clone)]
pub struct A11yViolation {
    pub rule: String,
    pub element: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

/// Lint HTML output for accessibility issues
pub fn lint_html(html: &str, config: &A11yConfig) -> Result<Vec<A11yViolation>> {
    let mut violations = Vec::new();

    if !config.enabled {
        return Ok(violations);
    }

    // Check for missing alt attributes on <img> tags
    if config.check_alt {
        violations.extend(check_img_alt(html));
    }

    // Check for ARIA labels on interactive elements
    if config.check_aria {
        violations.extend(check_aria_labels(html));
    }

    // Check for color contrast issues (basic — checks inline styles)
    if config.check_contrast {
        violations.extend(check_color_contrast(html));
    }

    // Check for missing <html lang> attribute
    violations.extend(check_html_lang(html));

    // Check for missing <title> tag
    violations.extend(check_title_tag(html));

    // Check for form elements without labels
    violations.extend(check_form_labels(html));

    if violations.is_empty() {
        info!("a11y lint: no violations found");
    } else {
        let errors = violations.iter().filter(|v| v.severity == Severity::Error).count();
        let warnings = violations.iter().filter(|v| v.severity == Severity::Warning).count();
        warn!("a11y lint: {} error(s), {} warning(s)", errors, warnings);
    }

    Ok(violations)
}

/// Check for missing alt attributes on images
fn check_img_alt(html: &str) -> Vec<A11yViolation> {
    let mut violations = Vec::new();

    let mut pos = 0;
    while let Some(start) = html[pos..].find("<img") {
        let abs_start = pos + start;
        // Find the end of the img tag
        if let Some(end) = html[abs_start..].find('>') {
            let tag = &html[abs_start..abs_start + end + 1];

            // Check if alt attribute exists
            if !tag.contains(" alt=") && !tag.contains(" alt ") {
                violations.push(A11yViolation {
                    rule: "img-alt".to_string(),
                    element: tag.to_string(),
                    message: "Image element missing alt attribute".to_string(),
                    severity: Severity::Error,
                });
            }
            pos = abs_start + end + 1;
        } else {
            break;
        }
    }

    violations
}

/// Check for ARIA labels on interactive elements (buttons, links without text)
fn check_aria_labels(html: &str) -> Vec<A11yViolation> {
    let mut violations = Vec::new();

    // Check <button> without aria-label and without text content
    let mut pos = 0;
    while let Some(start) = html[pos..].find("<button") {
        let abs_start = pos + start;
        if let Some(end) = html[abs_start..].find('>') {
            let tag = &html[abs_start..abs_start + end + 1];

            // Check if button has aria-label, aria-labelledby, or title
            let has_label = tag.contains("aria-label=")
                || tag.contains("aria-labelledby=")
                || tag.contains("title=");

            if !has_label {
                // Check if button has text content
                if let Some(close_start) = html[abs_start..].find("</button>") {
                    let content = &html[abs_start + end + 1..abs_start + close_start];
                    let content_str = content.replace('<', " <");
                    let text = content_str.trim();
                    if text.is_empty() {
                        violations.push(A11yViolation {
                            rule: "button-aria-label".to_string(),
                            element: tag.to_string(),
                            message: "Interactive button missing aria-label and text content".to_string(),
                            severity: Severity::Error,
                        });
                    }
                }
            }
            pos = abs_start + end + 1;
        } else {
            break;
        }
    }

    violations
}

/// Basic color contrast check — looks for low contrast inline styles
fn check_color_contrast(html: &str) -> Vec<A11yViolation> {
    let mut violations = Vec::new();

    // Check for color: #999 or lighter on dark backgrounds (simplified)
    let low_contrast_patterns = [
        ("color: #ccc", "Very light text color may have insufficient contrast"),
        ("color: #ddd", "Very light text color may have insufficient contrast"),
        ("color: #eee", "Very light text color may have insufficient contrast"),
        ("color: #999", "Medium gray text may have insufficient contrast"),
    ];

    for (pattern, message) in &low_contrast_patterns {
        if html.contains(pattern) {
            violations.push(A11yViolation {
                rule: "color-contrast".to_string(),
                element: pattern.to_string(),
                message: message.to_string(),
                severity: Severity::Warning,
            });
        }
    }

    violations
}

/// Check for <html lang> attribute
fn check_html_lang(html: &str) -> Vec<A11yViolation> {
    if let Some(html_tag_start) = html.find("<html") {
        if let Some(html_tag_end) = html[html_tag_start..].find('>') {
            let tag = &html[html_tag_start..html_tag_start + html_tag_end + 1];
            if !tag.contains(" lang=") {
                return vec![A11yViolation {
                    rule: "html-lang".to_string(),
                    element: tag.to_string(),
                    message: "HTML element missing lang attribute".to_string(),
                    severity: Severity::Error,
                }];
            }
        }
    }
    Vec::new()
}

/// Check for <title> tag
fn check_title_tag(html: &str) -> Vec<A11yViolation> {
    if !html.contains("<title>") && !html.contains("<title ") {
        return vec![A11yViolation {
            rule: "document-title".to_string(),
            element: "<head>".to_string(),
            message: "Document missing <title> element".to_string(),
            severity: Severity::Error,
        }];
    }
    Vec::new()
}

/// Check for form inputs without associated labels
fn check_form_labels(html: &str) -> Vec<A11yViolation> {
    let mut violations = Vec::new();

    let mut pos = 0;
    while let Some(start) = html[pos..].find("<input") {
        let abs_start = pos + start;
        if let Some(end) = html[abs_start..].find('>') {
            let tag = &html[abs_start..abs_start + end + 1];

            // Skip hidden inputs
            if tag.contains("type=\"hidden\"") || tag.contains("type='hidden'") {
                pos = abs_start + end + 1;
                continue;
            }

            let has_label = tag.contains("aria-label=")
                || tag.contains("aria-labelledby=")
                || tag.contains("id="); // Assume id-based label association

            if !has_label {
                violations.push(A11yViolation {
                    rule: "form-label".to_string(),
                    element: tag.to_string(),
                    message: "Form input missing label or aria-label".to_string(),
                    severity: Severity::Warning,
                });
            }
            pos = abs_start + end + 1;
        } else {
            break;
        }
    }

    violations
}

/// Format violations for CLI output
pub fn format_violations(violations: &[A11yViolation]) -> String {
    let mut output = String::new();
    for v in violations {
        let icon = if v.severity == Severity::Error { "✗" } else { "⚠" };
        let color = if v.severity == Severity::Error { "\x1b[31m" } else { "\x1b[33m" };
        output.push_str(&format!(
            "  {}{} {} \x1b[0m \x1b[90m{}\x1b[0m\n    {}\n",
            color, icon, v.rule, v.element, v.message,
        ));
    }
    output
}
