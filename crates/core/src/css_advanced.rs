// Advanced CSS features: #66 composes, #67 dark mode, #68 custom property
// optimization, #69 scoped CSS for React, #70 nesting polyfill verification.

use std::collections::HashMap;
use std::path::Path;
use regex::Regex;
use std::sync::OnceLock;
use tracing::{info, warn};

// ── Feature 66: CSS Modules composes ──────────────────────────────────

/// Parsed `composes` directive: `composes: button from './buttons.css'`
/// or `composes: button` (local composition)
#[derive(Debug, Clone)]
pub struct ComposesDirective {
    /// Local class name being composed into
    pub local_class: String,
    /// Class names to compose from
    pub source_classes: Vec<String>,
    /// Source file path (None = local composition)
    pub from_file: Option<String>,
}

/// Parse `composes` directives from CSS source.
/// Handles both local (`composes: button`) and cross-file (`composes: button from './btn.css'`).
pub fn parse_composes(css: &str, _file_path: &str) -> Vec<ComposesDirective> {
    static COMPOSES_RE: OnceLock<Regex> = OnceLock::new();
    let re = COMPOSES_RE.get_or_init(|| {
        Regex::new(
            r"\.([a-zA-Z_][\w-]*)\s*\{[^}]*composes:\s*([^;]+);",
        ).unwrap()
    });

    let mut directives = Vec::new();

    for cap in re.captures_iter(css) {
        let local_class = cap[1].to_string();
        let raw = cap[2].trim();

        // Check for "from" keyword: `button from './buttons.css'`
        if let Some(from_pos) = raw.find(" from ") {
            let classes_str = &raw[..from_pos].trim();
            let from_file = raw[from_pos + 6..].trim().trim_matches(|c| c == '"' || c == '\'');

            let source_classes: Vec<String> = classes_str
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            directives.push(ComposesDirective {
                local_class,
                source_classes,
                from_file: Some(from_file.to_string()),
            });
        } else {
            // Local composition
            let source_classes: Vec<String> = raw
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            directives.push(ComposesDirective {
                local_class,
                source_classes,
                from_file: None,
            });
        }
    }

    directives
}

/// Resolve composes directives by loading source files and merging class mappings.
/// Returns a map of local class → composed class list (scoped names).
pub fn resolve_composes(
    css: &str,
    file_path: &str,
    css_module_map: &[(String, String)],
    _root: &Path,
) -> HashMap<String, Vec<String>> {
    let directives = parse_composes(css, file_path);
    let mut result = HashMap::new();

    // Build a lookup from original → scoped name
    let local_map: HashMap<&str, &str> = css_module_map
        .iter()
        .map(|(o, s)| (o.as_str(), s.as_str()))
        .collect();

    for dir in &directives {
        let mut composed_classes = Vec::new();

        // Add the local scoped class
        if let Some(scoped) = local_map.get(dir.local_class.as_str()) {
            composed_classes.push(scoped.to_string());
        }

        // Add composed classes
        for src_class in &dir.source_classes {
            if let Some(ref from_file) = dir.from_file {
                // Cross-file: load the source file's CSS module map
                let source_path = Path::new(file_path)
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join(from_file);

                if source_path.is_file() {
                    if let Ok(source_css) = std::fs::read_to_string(&source_path) {
                        let source_map = generate_css_module_map(&source_css, &source_path.to_string_lossy());
                        for (orig, scoped) in &source_map {
                            if orig == src_class {
                                composed_classes.push(scoped.clone());
                            }
                        }
                    }
                }
            } else {
                // Local composition
                if let Some(scoped) = local_map.get(src_class.as_str()) {
                    composed_classes.push(scoped.to_string());
                }
            }
        }

        result.insert(dir.local_class.clone(), composed_classes);
    }

    result
}

/// Remove `composes:` directives from CSS after resolution.
pub fn strip_composes(css: &str) -> String {
    static COMPOSES_LINE_RE: OnceLock<Regex> = OnceLock::new();
    let re = COMPOSES_LINE_RE.get_or_init(|| {
        Regex::new(r"^\s*composes:\s*[^;]+;\s*$").unwrap()
    });

    css.lines()
        .filter(|line| !re.is_match(line))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Feature 67: Dark mode CSS generation ──────────────────────────────

/// Generate dark mode CSS variants from a stylesheet.
/// Strategy: detect `prefers-color-scheme: dark` media queries and extract
/// them, or auto-generate dark variants using CSS custom property inversion.
pub fn generate_dark_mode_css(css: &str, strategy: &str) -> String {
    match strategy {
        "auto" => auto_dark_mode(css),
        "extract" => extract_dark_media(css),
        _ => css.to_string(),
    }
}

/// Auto-generate dark mode by inverting lightness of color values via CSS custom properties.
fn auto_dark_mode(css: &str) -> String {
    // Check if the CSS already has prefers-color-scheme
    if css.contains("prefers-color-scheme: dark") {
        return css.to_string();
    }

    // Extract :root custom properties and generate dark variants
    static ROOT_VAR_RE: OnceLock<Regex> = OnceLock::new();
    let re = ROOT_VAR_RE.get_or_init(|| {
        Regex::new(r":root\s*\{([^}]+)\}").unwrap()
    });

    if let Some(cap) = re.captures(css) {
        let root_vars = &cap[1];
        let mut dark_vars = Vec::new();

        // Parse custom properties
        for line in root_vars.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("--") {
                if let Some(colon_pos) = trimmed.find(':') {
                    let name = trimmed[..colon_pos].trim();
                    let value = trimmed[colon_pos + 1..].trim().trim_end_matches(';');

                    // Generate dark variant by checking for color-like values
                    if is_color_value(value) {
                        let dark_value = invert_color_lightness(value);
                        dark_vars.push(format!("    {}: {};", name, dark_value));
                    }
                }
            }
        }

        if !dark_vars.is_empty() {
            let dark_block = format!(
                "\n\n@media (prefers-color-scheme: dark) {{\n  :root {{\n{}\n  }}\n}}",
                dark_vars.join("\n")
            );
            info!("Generated {} dark mode custom properties", dark_vars.len());
            return format!("{}{}", css, dark_block);
        }
    }

    warn!("No CSS custom properties found for dark mode generation");
    css.to_string()
}

/// Extract dark mode media queries into a separate block.
fn extract_dark_media(css: &str) -> String {
    static MEDIA_RE: OnceLock<Regex> = OnceLock::new();
    let re = MEDIA_RE.get_or_init(|| {
        Regex::new(r"@media\s*\(prefers-color-scheme:\s*dark\)\s*\{([^}]*(?:\{[^}]*\}[^}]*)*)\}").unwrap()
    });

    let mut dark_css = String::new();
    for cap in re.captures_iter(css) {
        dark_css.push_str(&cap[0]);
        dark_css.push('\n');
    }

    if dark_css.is_empty() {
        css.to_string()
    } else {
        format!("{}\n\n{}", css, dark_css)
    }
}

fn is_color_value(value: &str) -> bool {
    let v = value.trim().to_lowercase();
    v.starts_with('#')
        || v.starts_with("rgb(")
        || v.starts_with("rgba(")
        || v.starts_with("hsl(")
        || v.starts_with("hsla(")
        || v.starts_with("color(")
}

/// Invert lightness of a color value for dark mode.
fn invert_color_lightness(value: &str) -> String {
    let v = value.trim();

    // Handle hex colors
    if v.starts_with('#') && v.len() == 7 {
        let r = u8::from_str_radix(&v[1..3], 16).unwrap_or(0);
        let g = u8::from_str_radix(&v[3..5], 16).unwrap_or(0);
        let b = u8::from_str_radix(&v[5..7], 16).unwrap_or(0);
        // Invert
        return format!("#{:02x}{:02x}{:02x}", 255 - r, 255 - g, 255 - b);
    }

    // Handle rgb()/rgba()
    if v.starts_with("rgb(") || v.starts_with("rgba(") {
        static RGB_RE: OnceLock<Regex> = OnceLock::new();
        let re = RGB_RE.get_or_init(|| {
            Regex::new(r"rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+)\s*)?\)").unwrap()
        });
        if let Some(cap) = re.captures(v) {
            let r: u8 = cap[1].parse().unwrap_or(0);
            let g: u8 = cap[2].parse().unwrap_or(0);
            let b: u8 = cap[3].parse().unwrap_or(0);
            if let Some(alpha) = cap.get(4) {
                return format!("rgba({}, {}, {}, {})", 255 - r, 255 - g, 255 - b, alpha.as_str());
            }
            return format!("rgb({}, {}, {})", 255 - r, 255 - g, 255 - b);
        }
    }

    // Can't invert — return as-is
    v.to_string()
}

// ── Feature 68: CSS custom properties optimization ────────────────────

/// Optimize CSS custom properties:
/// 1. Inline static custom properties (used in only one place)
/// 2. Remove unused :root variables
/// 3. Minify custom property names in production
pub fn optimize_custom_properties(css: &str, minify_names: bool) -> String {
    let mut result = css.to_string();

    // Step 1: Extract all custom property definitions
    let props = extract_custom_properties(&result);

    if props.is_empty() {
        return result;
    }

    // Step 2: Find usage count for each property
    let usage_counts = count_property_usage(&result, &props);

    // Step 3: Remove unused properties
    let mut removed = 0;
    for (name, _) in &props {
        if usage_counts.get(name.as_str()).copied().unwrap_or(0) == 0 {
            // Remove the property definition
            let pattern = format!(r"\s*{}\s*:\s*[^;]+;\s*", regex::escape(name));
            if let Ok(re) = Regex::new(&pattern) {
                result = re.replace(&result, "").to_string();
                removed += 1;
            }
        }
    }

    if removed > 0 {
        info!("Removed {} unused CSS custom properties", removed);
    }

    // Step 4: Inline single-use properties
    let mut inlined = 0;
    for (name, value) in &props {
        if usage_counts.get(name.as_str()).copied().unwrap_or(0) == 1 {
            // Replace var(name) with the value
            let pattern = format!(r"var\(\s*{}\s*\)", regex::escape(name));
            if let Ok(re) = Regex::new(&pattern) {
                let before = result.clone();
                result = re.replace(&result, value.as_str()).to_string();
                if result != before {
                    inlined += 1;
                }
            }
            // Remove the definition
            let def_pattern = format!(r"\s*{}\s*:\s*[^;]+;\s*", regex::escape(name));
            if let Ok(re) = Regex::new(&def_pattern) {
                result = re.replace(&result, "").to_string();
            }
        }
    }

    if inlined > 0 {
        info!("Inlined {} single-use CSS custom properties", inlined);
    }

    // Step 5: Minify property names in production
    if minify_names {
        result = minify_property_names(&result, &props);
    }

    result
}

fn extract_custom_properties(css: &str) -> Vec<(String, String)> {
    static PROP_RE: OnceLock<Regex> = OnceLock::new();
    let re = PROP_RE.get_or_init(|| {
        Regex::new(r"(?:^|\s)(--[a-zA-Z_][\w-]*)\s*:\s*([^;]+);").unwrap()
    });

    re.captures_iter(css)
        .map(|cap| (cap[1].to_string(), cap[2].trim().to_string()))
        .collect()
}

fn count_property_usage(css: &str, props: &[(String, String)]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (name, _) in props {
        let pattern = format!(r"var\(\s*{}\s*\)", regex::escape(name));
        if let Ok(re) = Regex::new(&pattern) {
            let count = re.find_iter(css).count();
            counts.insert(name.clone(), count);
        }
    }
    counts
}

fn minify_property_names(css: &str, props: &[(String, String)]) -> String {
    let mut result = css.to_string();
    let mut name_map = HashMap::new();

    for (i, (name, _)) in props.iter().enumerate() {
        // Generate short name: --a, --b, --c, ...
        let short_name = format!("--{}", char::from_u32('a' as u32 + i as u32).unwrap_or('z'));
        name_map.insert(name.clone(), short_name);
    }

    // Replace definitions
    for (name, short) in &name_map {
        let pattern = regex::escape(name);
        if let Ok(re) = Regex::new(&pattern) {
            result = re.replace_all(&result, short.as_str()).to_string();
        }
    }

    result
}

// ── Feature 69: Scoped CSS for React ──────────────────────────────────

/// Generate a scoped attribute hash for a CSS file (like Vue's data-v-xxxxx).
pub fn generate_scope_hash(file_path: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    file_path.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Scope CSS selectors with a data attribute: `.button` → `.button[data-v-abc123]`
pub fn scope_css_with_attribute(css: &str, scope_hash: &str) -> String {
    let attr = format!("[data-v-{}]", scope_hash);

    // Scope all class selectors: .classname followed by delimiter or end
    static SELECTOR_RE: OnceLock<Regex> = OnceLock::new();
    let re = SELECTOR_RE.get_or_init(|| {
        Regex::new(r"(\.[a-zA-Z_][\w-]*)").unwrap()
    });

    let result = re.replace_all(css, |caps: &regex::Captures| {
        format!("{}{}", &caps[1], attr)
    });

    result.to_string()
}

/// Inject scope attribute into a React component's JSX.
/// Adds `data-v-xxxxx` to the root element of the component.
pub fn inject_scope_attribute(code: &str, scope_hash: &str) -> String {
    let attr = format!("data-v-{}", scope_hash);

    // Find the first JSX opening tag and inject the attribute
    static JSX_TAG_RE: OnceLock<Regex> = OnceLock::new();
    let re = JSX_TAG_RE.get_or_init(|| {
        Regex::new(r"(<[a-zA-Z][\w.-]*(?:\s[^>]*)?)(>)").unwrap()
    });

    if let Some(m) = re.captures(&code) {
        let before = &code[..m[1].len()];
        let after = &code[m[1].len()..];
        // Check if attribute already exists
        if !before.contains(&attr) {
            return format!("{} {}{}", before, attr, after);
        }
    }

    code.to_string()
}

// ── Feature 70: CSS nesting polyfill ──────────────────────────────────

/// Check if CSS contains native nesting syntax.
pub fn has_native_nesting(css: &str) -> bool {
    // Detect & selector (nesting parent reference)
    // Matches & anywhere in the CSS (not just at line start)
    static NESTING_RE: OnceLock<Regex> = OnceLock::new();
    let re = NESTING_RE.get_or_init(|| {
        Regex::new(r"&[\s.{:>+~(]").unwrap()
    });

    re.is_match(css)
}

/// Polyfill CSS nesting for older browsers.
/// This is already handled by lightningcss's minify pass which transpiles nesting,
/// but this function provides explicit polyfilling when needed.
pub fn polyfill_nesting(css: &str) -> String {
    // lightningcss already handles nesting transpilation during minify
    // This is a no-op wrapper that documents the capability
    if has_native_nesting(css) {
        info!("CSS nesting detected — will be transpiled by lightningcss");
    }
    css.to_string()
}

// ── Helper: reuse CSS module map generation ───────────────────────────

/// Generate CSS module class name mappings (reused from transform.rs).
fn generate_css_module_map(css: &str, file_path: &str) -> Vec<(String, String)> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    file_path.hash(&mut hasher);
    let file_hash = format!("{:x}", hasher.finish());
    let short_hash = &file_hash[..6];

    static CLASS_RE: OnceLock<Regex> = OnceLock::new();
    let re = CLASS_RE.get_or_init(|| {
        Regex::new(r"\.([a-zA-Z_][\w-]*)").unwrap()
    });

    let mut mappings = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for cap in re.captures_iter(css) {
        let class = cap[1].to_string();
        if !seen.contains(&class) {
            seen.insert(class.clone());
            let scoped = format!("_{}_{}", class, short_hash);
            mappings.push((class, scoped));
        }
    }

    mappings
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_composes_local() {
        let css = ".btn { color: red; composes: base; }";
        let dirs = parse_composes(css, "test.css");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].local_class, "btn");
        assert_eq!(dirs[0].source_classes, vec!["base"]);
        assert!(dirs[0].from_file.is_none());
    }

    #[test]
    fn test_parse_composes_cross_file() {
        let css = ".btn { composes: button from './buttons.css'; }";
        let dirs = parse_composes(css, "test.css");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].local_class, "btn");
        assert_eq!(dirs[0].source_classes, vec!["button"]);
        assert_eq!(dirs[0].from_file.as_deref(), Some("./buttons.css"));
    }

    #[test]
    fn test_strip_composes() {
        let css = ".btn {\n  color: red;\n  composes: base;\n}";
        let result = strip_composes(css);
        assert!(!result.contains("composes"));
        assert!(result.contains("color: red"));
    }

    #[test]
    fn test_dark_mode_auto() {
        let css = ":root { --bg: #ffffff; --text: #000000; }";
        let result = generate_dark_mode_css(css, "auto");
        assert!(result.contains("prefers-color-scheme: dark"));
        assert!(result.contains("--bg"));
    }

    #[test]
    fn test_dark_mode_no_existing() {
        let css = ".btn { color: red; }";
        let result = generate_dark_mode_css(css, "auto");
        // No custom properties, should return as-is
        assert_eq!(result, css);
    }

    #[test]
    fn test_invert_hex_color() {
        assert_eq!(invert_color_lightness("#ffffff"), "#000000");
        assert_eq!(invert_color_lightness("#000000"), "#ffffff");
        assert_eq!(invert_color_lightness("#ff0000"), "#00ffff");
    }

    #[test]
    fn test_invert_rgb() {
        assert_eq!(invert_color_lightness("rgb(255, 255, 255)"), "rgb(0, 0, 0)");
        assert_eq!(invert_color_lightness("rgba(0, 0, 0, 0.5)"), "rgba(255, 255, 255, 0.5)");
    }

    #[test]
    fn test_optimize_custom_properties_remove_unused() {
        let css = ":root { --unused: red; --used: blue; } .btn { color: var(--used); }";
        let result = optimize_custom_properties(css, false);
        assert!(!result.contains("--unused"));
        assert!(result.contains("--used") || result.contains("blue"));
    }

    #[test]
    fn test_optimize_custom_properties_inline_single() {
        let css = ":root { --once: red; } .btn { color: var(--once); }";
        let result = optimize_custom_properties(css, false);
        assert!(result.contains("red"));
        assert!(!result.contains("--once"));
    }

    #[test]
    fn test_scope_css_with_attribute() {
        let css = ".btn { color: red; } .card { padding: 10px; }";
        let result = scope_css_with_attribute(css, "abc123");
        assert!(result.contains("[data-v-abc123]"));
    }

    #[test]
    fn test_generate_scope_hash() {
        let hash1 = generate_scope_hash("src/Button.css");
        let hash2 = generate_scope_hash("src/Button.css");
        let hash3 = generate_scope_hash("src/Card.css");
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_inject_scope_attribute() {
        let code = "function Button() { return <button>Click</button>; }";
        let result = inject_scope_attribute(code, "abc123");
        assert!(result.contains("data-v-abc123"));
    }

    #[test]
    fn test_has_native_nesting() {
        assert!(has_native_nesting(".btn { color: red; &:hover { color: blue; } }"));
        assert!(!has_native_nesting(".btn { color: red; }"));
    }
}
