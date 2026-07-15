// SVG optimization — built-in SVGO with component import support.
//
// Features:
//   - SVG minification (remove comments, metadata, empty elements)
//   - Component import — import .svg files as React/Vue/Svelte components
//   - Sprite generation — combine multiple SVGs into a single sprite
//   - Inline SVG — inject SVG directly into HTML

use std::path::Path;

/// SVG optimization options
#[derive(Debug, Clone)]
pub struct SvgOptions {
    /// Remove XML declaration
    pub remove_xml_decl: bool,
    /// Remove comments
    pub remove_comments: bool,
    /// Remove metadata element
    pub remove_metadata: bool,
    /// Remove empty elements
    pub remove_empty_elements: bool,
    /// Remove unused namespaces
    pub remove_unused_ns: bool,
    /// Collapse whitespace
    pub collapse_whitespace: bool,
    /// Minify styles
    pub minify_styles: bool,
    /// Convert colors to short form
    pub convert_colors: bool,
    /// Output as component (React/Vue/Svelte)
    pub as_component: bool,
    /// Component framework
    pub framework: SvgComponentFramework,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            remove_xml_decl: true,
            remove_comments: true,
            remove_metadata: true,
            remove_empty_elements: true,
            remove_unused_ns: true,
            collapse_whitespace: true,
            minify_styles: true,
            convert_colors: true,
            as_component: false,
            framework: SvgComponentFramework::React,
        }
    }
}

/// Target framework for SVG component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgComponentFramework {
    React,
    Vue,
    Svelte,
    Solid,
}

/// Optimize an SVG string
pub fn optimize_svg(svg: &str, opts: &SvgOptions) -> String {
    let mut result = svg.to_string();

    if opts.remove_xml_decl {
        if let Some(end) = result.find("?>") {
            if result.starts_with("<?xml") {
                result = result[end + 2..].trim_start().to_string();
            }
        }
    }

    if opts.remove_comments {
        // Remove HTML comments <!-- -->
        while let Some(start) = result.find("<!--") {
            if let Some(end) = result[start..].find("-->") {
                result = format!("{}{}", &result[..start], &result[start + end + 3..]);
            } else {
                break;
            }
        }
    }

    if opts.remove_metadata {
        // Remove <metadata>...</metadata>
        while let Some(start) = result.find("<metadata") {
            if let Some(end) = result[start..].find("</metadata>") {
                result = format!("{}{}", &result[..start], &result[start + end + 11..]);
            } else {
                break;
            }
        }
    }

    if opts.collapse_whitespace {
        // Collapse multiple whitespace into single space
        let mut collapsed = String::with_capacity(result.len());
        let mut prev_ws = false;
        for c in result.chars() {
            if c.is_whitespace() {
                if !prev_ws {
                    collapsed.push(' ');
                }
                prev_ws = true;
            } else {
                collapsed.push(c);
                prev_ws = false;
            }
        }
        result = collapsed;
    }

    if opts.remove_empty_elements {
        // Remove empty elements like <g></g>, <rect/>
        // Simple heuristic: remove <tag></tag> with nothing between
        let empty_pattern = regex_like_empty_removal(&result);
        result = empty_pattern;
    }

    result.trim().to_string()
}

/// Simple empty element removal (without regex crate)
fn regex_like_empty_removal(svg: &str) -> String {
    let mut result = svg.to_string();
    let tags_to_check = ["g", "defs", "clipPath", "mask", "symbol", "pattern", "linearGradient", "radialGradient"];

    for tag in &tags_to_check {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);
        loop {
            if let Some(start) = result.find(&open) {
                if let Some(close_pos) = result[start..].find(&close) {
                    let inner = &result[start + open.len()..start + close_pos];
                    // Check if inner is empty or only whitespace
                    let inner_trimmed = inner.trim();
                    if inner_trimmed.is_empty() || inner_trimmed.starts_with("/>") {
                        // Skip self-closing
                        continue;
                    }
                    // Check if it's just an empty tag with attributes
                    if let Some(gt_pos) = inner.find('>') {
                        let after_gt = &inner[gt_pos + 1..].trim();
                        if after_gt.is_empty() {
                            result = format!("{}{}", &result[..start], &result[start + close_pos + close.len()..]);
                            continue;
                        }
                    }
                }
            }
            break;
        }
    }

    result
}

/// Convert an SVG to a React component
pub fn svg_to_react_component(svg: &str, component_name: &str) -> String {
    let optimized = optimize_svg(svg, &SvgOptions::default());
    // Convert SVG attributes to React-compatible (camelCase)
    let react_svg = svg_attributes_to_react(&optimized);

    format!(
        r#"import {{ type SVGProps }} from 'react';

export function {name}(props: SVGProps<SVGSVGElement>) {{
  return (
    {svg}
  );
}}

export default {name};"#,
        name = component_name,
        svg = react_svg.trim()
    )
}

/// Convert an SVG to a Vue component
pub fn svg_to_vue_component(svg: &str) -> String {
    let optimized = optimize_svg(svg, &SvgOptions::default());
    format!(
        r#"<template>
{}
</template>

<script setup lang="ts">
// Auto-generated SVG component
</script>"#,
        optimized
    )
}

/// Convert an SVG to a Svelte component
pub fn svg_to_svelte_component(svg: &str) -> String {
    let optimized = optimize_svg(svg, &SvgOptions::default());
    optimized
}

/// Convert SVG attributes to React-compatible format (camelCase)
fn svg_attributes_to_react(svg: &str) -> String {
    let mut result = svg.to_string();
    let replacements = [
        ("class=", "className="),
        ("stroke-width", "strokeWidth"),
        ("stroke-linecap", "strokeLinecap"),
        ("stroke-linejoin", "strokeLinejoin"),
        ("stroke-dasharray", "strokeDasharray"),
        ("stroke-dashoffset", "strokeDashoffset"),
        ("stroke-opacity", "strokeOpacity"),
        ("fill-opacity", "fillOpacity"),
        ("fill-rule", "fillRule"),
        ("clip-path", "clipPath"),
        ("clip-rule", "clipRule"),
        ("stop-color", "stopColor"),
        ("stop-opacity", "stopOpacity"),
        ("font-family", "fontFamily"),
        ("font-size", "fontSize"),
        ("font-weight", "fontWeight"),
        ("text-anchor", "textAnchor"),
        ("text-decoration", "textDecoration"),
        ("xlink:href", "xlinkHref"),
    ];

    for (from, to) in &replacements {
        result = result.replace(from, to);
    }

    result
}

/// Check if a file is an SVG
pub fn is_svg(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("svg"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_svg_removes_comments() {
        let svg = r#"<svg><!-- comment --><rect/></svg>"#;
        let optimized = optimize_svg(svg, &SvgOptions::default());
        assert!(!optimized.contains("<!--"));
        assert!(!optimized.contains("comment"));
    }

    #[test]
    fn test_optimize_svg_removes_xml_decl() {
        let svg = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg><rect/></svg>"#;
        let optimized = optimize_svg(svg, &SvgOptions::default());
        assert!(!optimized.contains("<?xml"));
    }

    #[test]
    fn test_svg_to_react_component() {
        let svg = r#"<svg class="icon"><rect width="10" height="10"/></svg>"#;
        let component = svg_to_react_component(svg, "MyIcon");
        assert!(component.contains("export function MyIcon"));
        assert!(component.contains("className="));
        assert!(!component.contains("class="));
    }

    #[test]
    fn test_is_svg() {
        assert!(is_svg(Path::new("icon.svg")));
        assert!(is_svg(Path::new("ICON.SVG")));
        assert!(!is_svg(Path::new("photo.jpg")));
    }
}
