// RTL CSS auto-generation (#107)
//
// Auto-generates RTL CSS from LTR stylesheets using CSS logical properties.
// Converts physical properties to their RTL equivalents.

use crate::config::CssConfig;
use tracing::info;

/// Physical-to-logical property mappings for RTL generation
const RTL_MAPPINGS: &[(&str, &str)] = &[
    ("margin-left", "margin-inline-start"),
    ("margin-right", "margin-inline-end"),
    ("padding-left", "padding-inline-start"),
    ("padding-right", "padding-inline-end"),
    ("border-left", "border-inline-start"),
    ("border-right", "border-inline-end"),
    ("border-left-width", "border-inline-start-width"),
    ("border-right-width", "border-inline-end-width"),
    ("border-left-color", "border-inline-start-color"),
    ("border-right-color", "border-inline-end-color"),
    ("border-left-style", "border-inline-start-style"),
    ("border-right-style", "border-inline-end-style"),
    ("left", "inset-inline-start"),
    ("right", "inset-inline-end"),
    ("text-align: left", "text-align: start"),
    ("text-align: right", "text-align: end"),
    ("float: left", "float: inline-start"),
    ("float: right", "float: inline-end"),
    ("clear: left", "clear: inline-start"),
    ("clear: right", "clear: inline-end"),
];

/// Generate RTL CSS from LTR source CSS
pub fn generate_rtl_css(source: &str, config: &CssConfig) -> Option<String> {
    if config.rtl != "auto" {
        return None;
    }

    let mut rtl = String::new();
    rtl.push_str("/* Auto-generated RTL CSS (#107) */\n");
    rtl.push_str("[dir=\"rtl\"] {\n");

    // Parse CSS rules and generate RTL equivalents
    let mut in_rule = false;
    let mut current_selector = String::new();
    let mut current_block = String::new();

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.ends_with('{') {
            in_rule = true;
            current_selector = trimmed.trim_end_matches('{').trim().to_string();
            current_block.clear();
        } else if trimmed == "}" {
            if in_rule && !current_block.is_empty() {
                let rtl_block = flip_properties(&current_block);
                if !rtl_block.is_empty() {
                    rtl.push_str(&format!("  {} {{\n{}\n  }}\n", current_selector, rtl_block));
                }
            }
            in_rule = false;
            current_selector.clear();
            current_block.clear();
        } else if in_rule {
            current_block.push_str(line);
            current_block.push('\n');
        }
    }

    rtl.push_str("}\n");

    info!("RTL CSS generated from LTR source");
    Some(rtl)
}

/// Flip physical properties to logical properties in a CSS block
fn flip_properties(block: &str) -> String {
    let mut result = String::new();

    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("/*") {
            continue;
        }

        let mut flipped = trimmed.to_string();
        let mut changed = false;

        for (physical, logical) in RTL_MAPPINGS {
            if flipped.contains(physical) {
                flipped = flipped.replace(physical, logical);
                changed = true;
            }
        }

        // Also flip values for specific properties
        if changed {
            result.push_str("  ");
            result.push_str(&flipped);
            result.push('\n');
        }
    }

    result
}

/// Check if CSS should have RTL generation applied
pub fn should_generate_rtl(config: &CssConfig) -> bool {
    config.rtl == "auto"
}
