// CSS advanced features: @layer management, container queries polyfill,
// critical CSS extraction, CSS source maps, PostCSS plugin caching.
//
// Features 26-30 from the roadmap.

use std::collections::HashMap;
use std::path::Path;

// ─── Feature 26: CSS @layer cascade layer management ──────────────────

/// CSS cascade layers found in a stylesheet
#[derive(Debug, Clone, Default)]
pub struct CascadeLayers {
    /// Layer names in declaration order
    pub layers: Vec<String>,
    /// Layer order as declared by @layer statement
    pub order_statement: Option<Vec<String>>,
}

/// Parse @layer declarations from CSS and return layer information
pub fn parse_layers(css: &str) -> CascadeLayers {
    let mut layers = CascadeLayers::default();
    let mut seen = std::collections::HashSet::new();

    // @layer name { ... } — block layer rule
    let mut search_pos = 0;
    while let Some(pos) = css[search_pos..].find("@layer ") {
        let abs_pos = search_pos + pos;
        let after = &css[abs_pos + 7..];

        // Check if this is a layer order statement: @layer name1, name2;
        let name_end = after
            .find(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '{')
            .unwrap_or(after.len());
        let first_name = after[..name_end].trim();

        // If next char after whitespace is , or ; it's a layer order statement
        let rest = &after[name_end..].trim_start();
        if rest.starts_with(';') || rest.starts_with(',') {
            // Layer order statement: @layer name1, name2, name3;
            let full_end = after.find(';').unwrap_or(after.len());
            let names_str = &after[..full_end];
            let names: Vec<String> = names_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            layers.order_statement = Some(names.clone());
            for name in &names {
                if !seen.contains(name) {
                    seen.insert(name.clone());
                    layers.layers.push(name.clone());
                }
            }
            search_pos = abs_pos + 7 + full_end + 1;
            continue;
        }

        // Regular @layer block
        if !first_name.is_empty() && !first_name.contains(',') {
            if !seen.contains(first_name) {
                seen.insert(first_name.to_string());
                layers.layers.push(first_name.to_string());
            }
        }

        // Skip past the block
        if let Some(brace) = after.find('{') {
            let mut depth = 1;
            let mut end = brace + 1;
            let bytes = after.as_bytes();
            while end < bytes.len() && depth > 0 {
                match bytes[end] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                end += 1;
            }
            search_pos = abs_pos + 7 + end;
        } else {
            search_pos = abs_pos + 7;
        }
    }

    layers
}

/// Reorder @layer blocks according to a declared order statement
/// Layers declared earlier have lower priority
pub fn reorder_layers(css: &str, layers: &CascadeLayers) -> String {
    let Some(ref order) = layers.order_statement else {
        return css.to_string();
    };

    // Extract layer blocks
    let mut layer_blocks: HashMap<String, String> = HashMap::new();
    let mut non_layer_css = String::new();
    let mut search_pos = 0;

    while let Some(pos) = css[search_pos..].find("@layer ") {
        let abs_pos = search_pos + pos;
        non_layer_css.push_str(&css[search_pos..abs_pos]);
        let after = &css[abs_pos + 7..];

        let name_end = after
            .find(|c: char| c.is_whitespace() || c == '{')
            .unwrap_or(after.len());
        let name = after[..name_end].trim().to_string();

        let rest = &after[name_end..].trim_start();
        if rest.starts_with(';') {
            // Skip order statement
            let semi_end = after.find(';').unwrap_or(after.len());
            search_pos = abs_pos + 7 + semi_end + 1;
            continue;
        }

        if let Some(brace) = after.find('{') {
            let mut depth = 1;
            let mut end = brace + 1;
            let bytes = after.as_bytes();
            while end < bytes.len() && depth > 0 {
                match bytes[end] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                end += 1;
            }
            let body = &after[brace + 1..end - 1];
            layer_blocks.insert(name, body.to_string());
            search_pos = abs_pos + 7 + end;
        } else {
            search_pos = abs_pos + 7;
        }
    }
    non_layer_css.push_str(&css[search_pos..]);

    // Reassemble in declared order
    let mut result = non_layer_css;
    for layer_name in order {
        if let Some(body) = layer_blocks.remove(layer_name) {
            result.push_str(&format!("@layer {} {{{}}}\n", layer_name, body));
        }
    }
    // Add any remaining layers not in the order statement
    for (name, body) in &layer_blocks {
        result.push_str(&format!("@layer {} {{{}}}\n", name, body));
    }

    result
}

// ─── Feature 27: Container queries polyfill ───────────────────────────

/// Polyfill @container queries for older browsers that don't support them.
/// Converts @container (min-width: 300px) { ... } to a JS-based polyfill
/// or a @media query fallback with a container class.
pub fn polyfill_container_queries(css: &str) -> String {
    let mut result = css.to_string();
    let mut search_pos = 0;

    while let Some(pos) = result[search_pos..].find("@container ") {
        let abs_pos = search_pos + pos;
        let after = &result[abs_pos + 11..];

        // Extract container name (optional) and condition
        // @container sidebar (min-width: 300px) { ... }
        // @container (min-width: 300px) { ... }
        let mut container_name = String::new();
        let condition_start;

        if after.trim_start().starts_with('(') {
            // No name, just condition
            condition_start = after.find('(').unwrap_or(0);
        } else {
            // Name then condition
            let name_end = after
                .find(|c: char| c.is_whitespace() || c == '(')
                .unwrap_or(after.len());
            container_name = after[..name_end].trim().to_string();
            condition_start = after.find('(').unwrap_or(name_end);
        }

        // Extract condition
        let condition_end = after[condition_start..]
            .find(')')
            .map(|p| condition_start + p + 1)
            .unwrap_or(condition_start);
        let condition = after[condition_start..condition_end].trim();

        // Convert (min-width: 300px) → .cq-{name}-min-w-300
        let polyfill_class = container_query_polyfill_class(&container_name, condition);

        // Find the block body
        let rest = &after[condition_end..];
        if let Some(brace) = rest.find('{') {
            let mut depth = 1;
            let mut end = brace + 1;
            let bytes = rest.as_bytes();
            while end < bytes.len() && depth > 0 {
                match bytes[end] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                end += 1;
            }

            let body = &rest[brace + 1..end - 1];

            // Generate polyfill: a class-based fallback
            // Also keep the original @container for browsers that support it
            let polyfill = format!(
                "@container {} {} {{\n  .{} {{\n{}\n  }}\n}}\n/* @container polyfill: .{} */\n.{} {{\n{}\n}}",
                container_name, condition, polyfill_class, body,
                polyfill_class, polyfill_class, body
            );

            let full_end = abs_pos + 11 + condition_end + end;
            result.replace_range(abs_pos..full_end, &polyfill);
            search_pos = abs_pos + polyfill.len();
        } else {
            search_pos = abs_pos + 11;
        }
    }

    result
}

/// Generate a polyfill class name from container name and condition
fn container_query_polyfill_class(name: &str, condition: &str) -> String {
    let clean = condition
        .trim_matches(|c| c == '(' || c == ')')
        .replace(':', "-")
        .replace(' ', "")
        .replace('.', "p");
    if name.is_empty() {
        format!("cq-{}", clean)
    } else {
        format!("cq-{}-{}", name, clean)
    }
}

// ─── Feature 28: Critical CSS extraction ──────────────────────────────

/// Configuration for critical CSS extraction
#[derive(Debug, Clone)]
pub struct CriticalCssConfig {
    /// HTML file to extract critical CSS for
    pub html_path: String,
    /// Viewport width for "above the fold" calculation (default: 1920)
    pub viewport_width: u32,
    /// Viewport height (default: 1080)
    pub viewport_height: u32,
    /// Maximum number of CSS rules to extract (default: 500)
    pub max_rules: usize,
}

impl Default for CriticalCssConfig {
    fn default() -> Self {
        Self {
            html_path: "index.html".to_string(),
            viewport_width: 1920,
            viewport_height: 1080,
            max_rules: 500,
        }
    }
}

/// Extract critical (above-the-fold) CSS from a full CSS file
/// based on selectors used in the HTML file
pub fn extract_critical_css(html: &str, css: &str, config: &CriticalCssConfig) -> String {
    // Collect all class names and IDs from the HTML
    let mut used_selectors = std::collections::HashSet::new();

    // Extract class="..." from HTML
    for pattern in ["class=\"", "class='"] {
        let quote = pattern.chars().last().unwrap();
        let mut search = 0;
        while let Some(pos) = html[search..].find(pattern) {
            let abs = search + pos + pattern.len();
            if let Some(end) = html[abs..].find(quote) {
                let class_str = &html[abs..abs + end];
                for cls in class_str.split_whitespace() {
                    used_selectors.insert(format!(".{}", cls));
                }
                search = abs + end + 1;
            } else {
                break;
            }
        }
    }

    // Extract id="..." from HTML
    for pattern in ["id=\"", "id='"] {
        let quote = pattern.chars().last().unwrap();
        let mut search = 0;
        while let Some(pos) = html[search..].find(pattern) {
            let abs = search + pos + pattern.len();
            if let Some(end) = html[abs..].find(quote) {
                let id_str = &html[abs..abs + end];
                used_selectors.insert(format!("#{}", id_str));
                search = abs + end + 1;
            } else {
                break;
            }
        }
    }

    // Extract tag names from HTML
    let tag_pattern = "<";
    let mut search = 0;
    while let Some(pos) = html[search..].find(tag_pattern) {
        let abs = search + pos + 1;
        let tag_end = html[abs..]
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(html[abs..].len());
        let tag = html[abs..abs + tag_end].to_string();
        if !tag.is_empty()
            && tag.chars().all(|c| c.is_alphabetic())
            && !matches!(tag.as_str(), "html" | "head" | "body" | "meta" | "link" | "script" | "style" | "title" | "!--")
        {
            used_selectors.insert(tag);
        }
        search = abs + tag_end;
    }

    // Always include universal and base selectors
    used_selectors.insert("*".to_string());
    used_selectors.insert("html".to_string());
    used_selectors.insert("body".to_string());

    // Filter CSS rules to only include those matching used selectors
    let mut critical = String::new();
    let mut rule_count = 0;

    // Split CSS into rule blocks — track selector start (after previous rule ends)
    let mut brace_depth = 0;
    let mut rule_start = 0;
    let mut selector_start = 0;
    let bytes = css.as_bytes();

    for i in 0..bytes.len() {
        match bytes[i] {
            b'{' => {
                if brace_depth == 0 {
                    rule_start = i;
                }
                brace_depth += 1;
            }
            b'}' => {
                brace_depth -= 1;
                if brace_depth == 0 {
                    // Selector is from selector_start to rule_start (the '{')
                    let selector_part = css[selector_start..rule_start].trim();
                    let full_rule = &css[selector_start..=i];

                    // Check if any used selector matches
                    if selector_matches(selectors_from_rule(selector_part), &used_selectors) {
                        critical.push_str(full_rule);
                        critical.push('\n');
                        rule_count += 1;
                        if rule_count >= config.max_rules {
                            break;
                        }
                    }
                    // Next selector starts after this rule
                    selector_start = i + 1;
                }
            }
            _ => {}
        }
    }

    critical
}

/// Extract selector names from a CSS rule selector
fn selectors_from_rule(selector: &str) -> Vec<String> {
    selector
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Check if any selector matches the used selectors
fn selector_matches(rule_selectors: Vec<String>, used: &std::collections::HashSet<String>) -> bool {
    for sel in &rule_selectors {
        // Exact match
        if used.contains(sel) {
            return true;
        }
        // Check if selector starts with a used class/id
        for used_sel in used {
            if sel.starts_with(used_sel) {
                return true;
            }
        }
        // Universal and element selectors
        if sel == "*" || sel == "html" || sel == "body" {
            return true;
        }
    }
    false
}

/// Inline critical CSS into an HTML file's <head>
pub fn inline_critical_css(html: &str, critical_css: &str) -> String {
    let style_tag = format!("<style>\n{}\n</style>", critical_css);

    // Insert before </head>
    if let Some(pos) = html.find("</head>") {
        let mut result = html[..pos].to_string();
        result.push_str(&style_tag);
        result.push('\n');
        result.push_str(&html[pos..]);
        result
    } else {
        // No </head>, prepend
        format!("{}\n{}", style_tag, html)
    }
}

// ─── Feature 29: CSS source maps in dev ───────────────────────────────

/// Generate a CSS source map pointing to the original file
pub fn generate_css_source_map(
    original_file: &str,
    original_source: &str,
    generated_css: &str,
) -> String {
    let file_name = Path::new(original_file)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.css");

    // Build a simple source map with the original source content
    // A full implementation would use Lightning CSS's source map support
    let map = serde_json::json!({
        "version": 3,
        "file": file_name.replace(".scss", ".css").replace(".less", ".css"),
        "sourceRoot": "",
        "sources": [original_file],
        "sourcesContent": [original_source],
        "mappings": generate_css_mappings(original_source, generated_css),
        "names": []
    });

    map.to_string()
}

/// Generate approximate CSS source map mappings
/// This creates a simple line-to-line mapping
fn generate_css_mappings(original: &str, generated: &str) -> String {
    let orig_lines: usize = original.lines().count();
    let gen_lines: usize = generated.lines().count();

    // Simple line-by-line mapping: each generated line maps to the same original line
    let mut mappings = Vec::new();
    for i in 0..gen_lines.min(orig_lines) {
        // VLQ encoding for: line 0, column 0, source 0, original line i, column 0
        mappings.push(format!("AAAA{}", vlq_encode(i as i32)));
    }

    mappings.join(";")
}

/// Encode a value as VLQ (Variable-Length Quantity) for source maps
fn vlq_encode(value: i32) -> String {
    let base64_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut val = if value < 0 { (-value << 1) | 1 } else { value << 1 };

    loop {
        let mut digit = (val & 0x1f) as usize;
        val >>= 5;
        if val > 0 {
            digit |= 0x20;
        }
        result.push(base64_chars.chars().nth(digit).unwrap_or('A'));
        if val == 0 {
            break;
        }
    }

    result
}

// ─── Feature 30: PostCSS plugin caching ───────────────────────────────

/// Cache for PostCSS plugin results, keyed by content hash + plugin config
#[derive(Debug, Clone, Default)]
pub struct PostCssCache {
    /// Map of cache key → processed CSS
    cache: HashMap<String, String>,
}

impl PostCssCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a cache key from source content and plugin config
    pub fn cache_key(source: &str, file_path: &str, plugin_names: &[&str]) -> String {
        let mut input = source.to_string();
        input.push_str(file_path);
        for name in plugin_names {
            input.push_str(name);
        }
        let hash = blake3::hash(input.as_bytes());
        hash.to_hex()[..16].to_string()
    }

    /// Try to get a cached result
    pub fn get(&self, key: &str) -> Option<&str> {
        self.cache.get(key).map(|s| s.as_str())
    }

    /// Store a result in the cache
    pub fn set(&mut self, key: &str, value: String) {
        self.cache.insert(key.to_string(), value);
    }

    /// Process CSS with caching — returns cached result if available
    pub fn process(
        &mut self,
        source: &str,
        file_path: &str,
        plugin_names: &[&str],
        process_fn: impl Fn(&str, &str) -> String,
    ) -> String {
        let key = Self::cache_key(source, file_path, plugin_names);
        if let Some(cached) = self.get(&key) {
            return cached.to_string();
        }

        let result = process_fn(source, file_path);
        self.set(&key, result.clone());
        result
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_layers() {
        let css = "@layer base, components, utilities;\n@layer base { * { box-sizing: border-box; } }\n@layer utilities { .flex { display: flex; } }";
        let layers = parse_layers(css);
        assert_eq!(layers.layers, vec!["base", "components", "utilities"]);
        assert_eq!(layers.order_statement, Some(vec!["base".to_string(), "components".to_string(), "utilities".to_string()]));
    }

    #[test]
    fn test_container_query_polyfill() {
        let css = "@container (min-width: 300px) { .card { font-size: 1.5rem; } }";
        let result = polyfill_container_queries(css);
        assert!(result.contains(".cq-min-width-300px"));
        assert!(result.contains("font-size: 1.5rem"));
    }

    #[test]
    fn test_extract_critical_css() {
        let html = "<div class=\"container\"><h1 class=\"title\">Hello</h1></div>";
        let css = ".container { max-width: 1200px; }\n.title { font-size: 2rem; }\n.unused { color: blue; }";
        let config = CriticalCssConfig::default();
        let critical = extract_critical_css(html, css, &config);
        assert!(critical.contains(".container"));
        assert!(critical.contains(".title"));
        assert!(!critical.contains(".unused"));
    }

    #[test]
    fn test_inline_critical_css() {
        let html = "<html><head><title>Test</title></head><body></body></html>";
        let css = ".test { color: red; }";
        let result = inline_critical_css(html, css);
        assert!(result.contains("<style>"));
        assert!(result.contains(".test { color: red; }"));
        assert!(result.contains("</style>"));
    }

    #[test]
    fn test_postcss_cache() {
        let mut cache = PostCssCache::new();
        let result1 = cache.process("body { color: red; }", "test.css", &["autoprefixer"], |src, _| {
            format!("/* processed */\n{}", src)
        });
        let result2 = cache.process("body { color: red; }", "test.css", &["autoprefixer"], |src, _| {
            format!("/* should not run */\n{}", src)
        });
        assert_eq!(result1, result2);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_css_source_map() {
        let map = generate_css_source_map("src/style.css", ".test { color: red; }", ".test{color:red}");
        assert!(map.contains("\"version\":3"));
        assert!(map.contains("src/style.css"));
    }
}
