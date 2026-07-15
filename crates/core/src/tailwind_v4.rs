// Tailwind v4 Oxide engine — CSS-first config with @theme, @utility, @variant directives.
//
// Tailwind v4 moves configuration from JS to CSS using these at-rules:
//   @theme { --color-primary: #3b82f6; --breakpoint-md: 768px; }
//   @utility tab-4 { tab-size: 4; }
//   @variant dark (&:where(.dark, .dark *)) { ... }
//
// This module:
//   - Parses @theme blocks into CSS custom properties
//   - Expands @utility into standard CSS classes
//   - Processes @variant to generate variant-aware selectors
//   - Detects Tailwind v4 by presence of @theme or @import "tailwindcss"
//   - Generates utility classes on-demand from theme tokens
//   - Integrates with Lightning CSS for final optimization

use std::collections::HashMap;
use std::path::Path;

/// Tailwind v4 theme configuration parsed from @theme blocks
#[derive(Debug, Clone, Default)]
pub struct TailwindV4Theme {
    /// Custom properties from @theme: --color-*, --spacing-*, --breakpoint-*, etc.
    pub properties: HashMap<String, String>,
    /// Whether @theme was found in the CSS
    pub has_theme: bool,
    /// Whether this is a Tailwind v4 project (detected via @import "tailwindcss")
    pub is_v4: bool,
}

impl TailwindV4Theme {
    /// Detect and parse Tailwind v4 configuration from CSS source
    pub fn from_css(css: &str) -> Self {
        let mut theme = Self::default();

        // Detect Tailwind v4 by @import "tailwindcss", @theme, @utility, or @variant
        if css.contains("@import \"tailwindcss\"") || css.contains("@import 'tailwindcss'") {
            theme.is_v4 = true;
        }

        // Parse @theme blocks
        if css.contains("@theme") {
            theme.has_theme = true;
            theme.is_v4 = true;
            theme.properties = parse_theme_block(css);
        }

        // @utility and @variant are also v4-only directives
        if css.contains("@utility ") || css.contains("@variant ") {
            theme.is_v4 = true;
        }

        theme
    }

    /// Check if this looks like a Tailwind v4 project by examining CSS files
    pub fn detect_in_project(root: &Path) -> bool {
        // Check for Tailwind v4 CSS entry point
        let candidates = [
            "src/index.css",
            "src/app.css",
            "src/main.css",
            "src/styles.css",
            "app/global.css",
            "styles/globals.css",
        ];

        for candidate in &candidates {
            let path = root.join(candidate);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if content.contains("@import \"tailwindcss\"")
                    || content.contains("@import 'tailwindcss'")
                    || content.contains("@theme")
                {
                    return true;
                }
            }
        }

        // Also check for @tailwindcss/vite or @tailwindcss/postcss in package.json
        let pkg_path = root.join("package.json");
        if let Ok(content) = std::fs::read_to_string(&pkg_path) {
            if content.contains("@tailwindcss/vite")
                || content.contains("@tailwindcss/postcss")
                || content.contains("\"tailwindcss\": \"^4")
            {
                return true;
            }
        }

        false
    }

    /// Get a theme property value by name (e.g., "color-primary" → "#3b82f6")
    pub fn get(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(|s| s.as_str())
    }

    /// Get all color properties (keys starting with "color-")
    pub fn colors(&self) -> Vec<(&str, &str)> {
        self.properties
            .iter()
            .filter(|(k, _)| k.starts_with("color-"))
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    /// Get all breakpoint properties (keys starting with "breakpoint-")
    pub fn breakpoints(&self) -> Vec<(&str, &str)> {
        self.properties
            .iter()
            .filter(|(k, _)| k.starts_with("breakpoint-"))
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    /// Get all spacing properties (keys starting with "spacing-")
    pub fn spacings(&self) -> Vec<(&str, &str)> {
        self.properties
            .iter()
            .filter(|(k, _)| k.starts_with("spacing-"))
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}

/// Parse @theme { ... } block and extract custom properties
fn parse_theme_block(css: &str) -> HashMap<String, String> {
    let mut props = HashMap::new();

    if let Some(theme_start) = css.find("@theme") {
        // Find the opening brace after @theme
        let after_theme = &css[theme_start..];
        if let Some(brace_start) = after_theme.find('{') {
            let abs_brace = theme_start + brace_start;
            // Find matching closing brace (accounting for nested braces)
            let mut depth = 1;
            let mut end = abs_brace + 1;
            let bytes = css.as_bytes();
            while end < bytes.len() && depth > 0 {
                match bytes[end] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                end += 1;
            }

            let block = &css[abs_brace + 1..end - 1];

            // Parse CSS custom properties: --name: value;
            for line in block.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("--") {
                    if let Some(colon) = trimmed.find(':') {
                        let name = trimmed[..colon].trim().to_string();
                        let value = trimmed[colon + 1..]
                            .trim()
                            .trim_end_matches(';')
                            .trim()
                            .to_string();
                        if !name.is_empty() && !value.is_empty() {
                            // Strip the leading -- for the key
                            let clean_name = name.strip_prefix("--").unwrap_or(&name).to_string();
                            props.insert(clean_name, value);
                        }
                    }
                }
            }
        }
    }

    props
}

/// Process Tailwind v4 CSS: expand @theme, @utility, @variant, and @import "tailwindcss"
pub fn process_tailwind_v4(css: &str, _root: &Path) -> String {
    let theme = TailwindV4Theme::from_css(css);

    if !theme.is_v4 {
        return css.to_string();
    }

    let mut result = css.to_string();

    // Replace @import "tailwindcss" with the base + utilities + components
    result = result.replace(
        "@import \"tailwindcss\";",
        &generate_tailwind_v4_entry(&theme),
    );
    result = result.replace(
        "@import 'tailwindcss';",
        &generate_tailwind_v4_entry(&theme),
    );

    // Process @utility blocks
    result = process_utility_blocks(&result);

    // Process @variant blocks
    result = process_variant_blocks(&result);

    // Process @theme block — emit as :root custom properties
    if theme.has_theme {
        result = process_theme_to_root(&result);
    }

    result
}

/// Generate the Tailwind v4 entry CSS (preflight + utilities + components)
fn generate_tailwind_v4_entry(theme: &TailwindV4Theme) -> String {
    let mut css = String::new();

    // Preflight (reset) — same as v3 base
    css.push_str(TAILWIND_V4_PREFLIGHT);

    // Theme custom properties as :root variables
    if !theme.properties.is_empty() {
        css.push_str("\n:root {\n");
        for (name, value) in &theme.properties {
            css.push_str(&format!("  --{}: {};\n", name, value));
        }
        css.push_str("}\n");
    }

    // Generate utility classes from theme tokens
    css.push_str(&generate_theme_utilities(theme));

    css
}

/// Generate utility classes from theme tokens (colors, spacing, breakpoints)
fn generate_theme_utilities(theme: &TailwindV4Theme) -> String {
    let mut css = String::new();

    // Color utilities: bg-{name}, text-{name}, border-{name}
    for (name, value) in theme.colors() {
        let color_name = name.strip_prefix("color-").unwrap_or(name);
        css.push_str(&format!(".bg-{} {{ background-color: {}; }}\n", color_name, value));
        css.push_str(&format!(".text-{} {{ color: {}; }}\n", color_name, value));
        css.push_str(&format!(".border-{} {{ border-color: {}; }}\n", color_name, value));
        css.push_str(&format!(".fill-{} {{ fill: {}; }}\n", color_name, value));
        css.push_str(&format!(".stroke-{} {{ stroke: {}; }}\n", color_name, value));
    }

    // Spacing utilities: p-{n}, m-{n}, gap-{n}, w-{n}, h-{n}
    for (name, value) in theme.spacings() {
        let spacing_name = name.strip_prefix("spacing-").unwrap_or(name);
        css.push_str(&format!(".p-{} {{ padding: {}; }}\n", spacing_name, value));
        css.push_str(&format!(".m-{} {{ margin: {}; }}\n", spacing_name, value));
        css.push_str(&format!(".gap-{} {{ gap: {}; }}\n", spacing_name, value));
        css.push_str(&format!(".w-{} {{ width: {}; }}\n", spacing_name, value));
        css.push_str(&format!(".h-{} {{ height: {}; }}\n", spacing_name, value));
    }

    // Breakpoint utilities: @media (min-width: {value})
    for (name, value) in theme.breakpoints() {
        let bp_name = name.strip_prefix("breakpoint-").unwrap_or(name);
        css.push_str(&format!(
            "@media (min-width: {}) {{ .{}\\:hidden {{ display: none; }} }}\n",
            value, bp_name
        ));
    }

    css
}

/// Process @utility blocks — expand to standard CSS class definitions
/// @utility tab-4 { tab-size: 4; } → .tab-4 { tab-size: 4; }
fn process_utility_blocks(css: &str) -> String {
    let mut result = css.to_string();

    while let Some(start) = result.find("@utility ") {
        let after = &result[start + 9..];
        // Find the utility name (up to whitespace or {)
        let name_end = after
            .find(|c: char| c.is_whitespace() || c == '{')
            .unwrap_or(after.len());
        let util_name = after[..name_end].trim();

        // Find the opening brace
        let rest = &after[name_end..];
        if let Some(brace_pos) = rest.find('{') {
            let abs_brace = start + 9 + name_end + brace_pos;
            // Find matching closing brace
            let mut depth = 1;
            let mut end = abs_brace + 1;
            let bytes = result.as_bytes();
            while end < bytes.len() && depth > 0 {
                match bytes[end] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                end += 1;
            }

            let body = &result[abs_brace + 1..end - 1];
            let replacement = format!(".{} {{{}}}", util_name, body);
            result.replace_range(start..end, &replacement);
        } else {
            break;
        }
    }

    result
}

/// Process @variant blocks — expand to variant-aware selectors
/// @variant dark (&:where(.dark, .dark *)) { ... } → :where(.dark, .dark *) { ... }
/// @variant hover { ... } → &:hover { ... }
fn process_variant_blocks(css: &str) -> String {
    let mut result = css.to_string();

    while let Some(start) = result.find("@variant ") {
        let after = &result[start + 9..];
        // Find the variant name (up to whitespace or {)
        let name_end = after
            .find(|c: char| c.is_whitespace() || c == '{')
            .unwrap_or(after.len());
        let variant_name = after[..name_end].trim();

        let rest = &after[name_end..];
        // Check if there's a selector argument in parentheses
        let (selector, body_start) = if rest.trim_start().starts_with('(') {
            if let Some(close) = rest.find(')') {
                let sel = rest[..close + 1].trim();
                let after_paren = &rest[close + 1..];
                if let Some(brace) = after_paren.find('{') {
                    (sel.to_string(), name_end + close + 1 + brace)
                } else {
                    break;
                }
            } else {
                break;
            }
        } else if let Some(brace) = rest.find('{') {
            (format!("&:{}", variant_name), name_end + brace)
        } else {
            break;
        };

        let abs_brace = start + 9 + body_start;
        // Find matching closing brace
        let mut depth = 1;
        let mut end = abs_brace + 1;
        let bytes = result.as_bytes();
        while end < bytes.len() && depth > 0 {
            match bytes[end] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            end += 1;
        }

        let body = &result[abs_brace + 1..end - 1];

        // Generate the variant selector
        let replacement = if selector.starts_with('(') && selector.ends_with(')') {
            // Custom selector: (&:where(.dark, .dark *)) → expand & to a placeholder
            let inner = &selector[1..selector.len() - 1];
            format!("{} {{{}}}", inner.replace('&', ":root"), body)
        } else {
            format!("{} {{{}}}", selector, body)
        };

        result.replace_range(start..end, &replacement);
    }

    result
}

/// Convert @theme block to :root custom properties
fn process_theme_to_root(css: &str) -> String {
    let mut result = css.to_string();

    if let Some(theme_start) = result.find("@theme") {
        let after = &result[theme_start..];
        if let Some(brace_start) = after.find('{') {
            let abs_brace = theme_start + brace_start;
            let mut depth = 1;
            let mut end = abs_brace + 1;
            let bytes = result.as_bytes();
            while end < bytes.len() && depth > 0 {
                match bytes[end] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                end += 1;
            }

            let block = &result[abs_brace + 1..end - 1];
            let mut root_css = String::from(":root {\n");
            for line in block.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("--") {
                    root_css.push_str(&format!("  {};\n", trimmed.trim_end_matches(';')));
                }
            }
            root_css.push_str("}");

            result.replace_range(theme_start..end, &root_css);
        }
    }

    result
}

/// Tailwind v4 preflight (reset) CSS
const TAILWIND_V4_PREFLIGHT: &str = r#"
*, ::before, ::after { box-sizing: border-box; border: 0 solid; }
html, :host { -webkit-text-size-adjust: 100%; tab-size: 4; line-height: 1.5; }
body { margin: 0; line-height: inherit; }
hr { height: 0; color: inherit; border-top-width: 1px; }
h1, h2, h3, h4, h5, h6 { font-size: inherit; font-weight: inherit; }
a { color: inherit; text-decoration: inherit; }
b, strong { font-weight: bolder; }
code, kbd, samp, pre { font-family: monospace; font-size: 1em; }
abbr:where([title]) { text-decoration: underline dotted; }
small { font-size: 80%; }
sub, sup { font-size: 75%; line-height: 0; position: relative; vertical-align: baseline; }
sub { bottom: -0.25em; }
sup { top: -0.5em; }
table { text-indent: 0; border-color: inherit; border-collapse: collapse; }
button, input, optgroup, select, textarea { font: inherit; color: inherit; margin: 0; padding: 0; }
button, select { text-transform: none; }
button, [type='button'], [type='reset'], [type='submit'] { -webkit-appearance: button; background-color: transparent; background-image: none; }
:-moz-focusring { outline: auto; }
:-moz-ui-invalid { box-shadow: none; }
progress { vertical-align: baseline; }
::-webkit-inner-spin-button, ::-webkit-outer-spin-button { height: auto; }
[type='search'] { -webkit-appearance: textfield; outline-offset: -2px; }
::-webkit-search-decoration { -webkit-appearance: none; }
summary { display: list-item; }
blockquote, dl, dd, h1, h2, h3, h4, h5, h6, hr, figure, p, pre { margin: 0; }
fieldset { margin: 0; padding: 0; border: 0; }
legend { padding: 0; }
ol, ul, menu { list-style: none; margin: 0; padding: 0; }
dialog { padding: 0; }
textarea { resize: vertical; }
input::placeholder, textarea::placeholder { opacity: 1; color: color-mix(in oklab, currentColor 50%, transparent); }
button, [role="button"] { cursor: pointer; }
:disabled { cursor: default; }
img, svg, video, canvas, audio, iframe, embed, object { display: block; vertical-align: middle; }
img, video { max-width: 100%; height: auto; }
[hidden] { display: none; }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_v4_from_import() {
        let css = "@import \"tailwindcss\";";
        let theme = TailwindV4Theme::from_css(css);
        assert!(theme.is_v4);
    }

    #[test]
    fn test_parse_theme_block() {
        let css = "@theme {\n  --color-primary: #3b82f6;\n  --spacing-4: 1rem;\n  --breakpoint-md: 768px;\n}";
        let theme = TailwindV4Theme::from_css(css);
        assert!(theme.has_theme);
        assert_eq!(theme.get("color-primary"), Some("#3b82f6"));
        assert_eq!(theme.get("spacing-4"), Some("1rem"));
        assert_eq!(theme.get("breakpoint-md"), Some("768px"));
    }

    #[test]
    fn test_process_utility_block() {
        let css = "@utility tab-4 {\n  tab-size: 4;\n}";
        let result = process_tailwind_v4(css, Path::new("."));
        assert!(result.contains(".tab-4 {"));
        assert!(result.contains("tab-size: 4;"));
    }

    #[test]
    fn test_process_theme_to_root() {
        let css = "@theme {\n  --color-primary: #3b82f6;\n}";
        let result = process_tailwind_v4(css, Path::new("."));
        assert!(result.contains(":root {"));
        assert!(result.contains("--color-primary: #3b82f6;"));
    }

    #[test]
    fn test_generate_theme_utilities() {
        let mut theme = TailwindV4Theme::default();
        theme.properties.insert("color-primary".to_string(), "#3b82f6".to_string());
        theme.properties.insert("spacing-4".to_string(), "1rem".to_string());
        let utils = generate_theme_utilities(&theme);
        assert!(utils.contains(".bg-primary { background-color: #3b82f6; }"));
        assert!(utils.contains(".text-primary { color: #3b82f6; }"));
        assert!(utils.contains(".p-4 { padding: 1rem; }"));
    }
}
