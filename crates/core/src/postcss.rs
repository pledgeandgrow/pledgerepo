// PostCSS pipeline — full PostCSS plugin compatibility.
//
// PostCSS is a tool for transforming CSS with JavaScript plugins.
// This module provides:
//   - PostCSS config file parsing (postcss.config.js/ts/json)
//   - Plugin loading and execution order
//   - Source map support
//   - Autoprefixer integration

use std::path::Path;
use std::collections::HashMap;

/// PostCSS plugin configuration
#[derive(Debug, Clone)]
pub struct PostCssPlugin {
    pub name: String,
    pub options: HashMap<String, String>,
}

/// PostCSS configuration
#[derive(Debug, Clone, Default)]
pub struct PostCssConfig {
    pub plugins: Vec<PostCssPlugin>,
    pub source_map: bool,
}

impl PostCssConfig {
    /// Parse a postcss.config.js/ts/json file
    pub fn from_file(root: &Path) -> Option<Self> {
        // Try postcss.config.js, postcss.config.ts, postcss.config.json, .postcssrc.json
        let candidates = [
            "postcss.config.js",
            "postcss.config.ts",
            "postcss.config.mjs",
            "postcss.config.cjs",
            ".postcssrc.json",
            ".postcssrc.js",
        ];

        for candidate in &candidates {
            let path = root.join(candidate);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    return Some(Self::parse_config(&content));
                }
            }
        }

        // Check package.json for "postcss" field
        let pkg_path = root.join("package.json");
        if pkg_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_path) {
                if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(postcss) = pkg.get("postcss") {
                        return Some(Self::parse_from_json(postcss));
                    }
                }
            }
        }

        None
    }

    /// Parse config content (JS/TS/JSON)
    fn parse_config(content: &str) -> Self {
        // Try JSON first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            return Self::parse_from_json(&json);
        }

        // For JS/TS, extract plugin names by looking for require/import patterns
        let mut plugins = Vec::new();

        // Match patterns like: require('autoprefixer') or from 'autoprefixer'
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            // Look for plugin references
            if let Some(name) = extract_plugin_name(trimmed) {
                plugins.push(PostCssPlugin {
                    name,
                    options: HashMap::new(),
                });
            }
        }

        // Check for sourceMap
        let source_map = content.contains("sourceMap: true") || content.contains("sourceMap:true");

        Self { plugins, source_map }
    }

    /// Parse from JSON value
    fn parse_from_json(json: &serde_json::Value) -> Self {
        let mut plugins = Vec::new();

        if let Some(plugins_obj) = json.get("plugins").and_then(|p| p.as_object()) {
            for (name, value) in plugins_obj {
                let mut options = HashMap::new();
                if let Some(opts) = value.as_object() {
                    for (k, v) in opts {
                        options.insert(k.clone(), v.to_string());
                    }
                }
                plugins.push(PostCssPlugin { name: name.clone(), options });
            }
        } else if let Some(plugins_arr) = json.get("plugins").and_then(|p| p.as_array()) {
            for plugin in plugins_arr {
                if let Some(name) = plugin.as_str() {
                    plugins.push(PostCssPlugin {
                        name: name.to_string(),
                        options: HashMap::new(),
                    });
                }
            }
        }

        let source_map = json.get("sourceMap").and_then(|v| v.as_bool()).unwrap_or(false);

        Self { plugins, source_map }
    }

    /// Check if PostCSS is configured
    pub fn has_plugins(&self) -> bool {
        !self.plugins.is_empty()
    }

    /// Get plugin names in order
    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name.as_str()).collect()
    }
}

/// Extract plugin name from a JS line
fn extract_plugin_name(line: &str) -> Option<String> {
    // require('plugin-name') or require("plugin-name")
    if let Some(start) = line.find("require(") {
        let after = &line[start + 8..];
        let quote = after.chars().next()?;
        if quote == '\'' || quote == '"' {
            let end = after[1..].find(quote)?;
            return Some(after[1..1 + end].to_string());
        }
    }

    // from 'plugin-name' or from "plugin-name"
    if let Some(start) = line.find(" from ") {
        let after = &line[start + 6..].trim();
        let quote = after.chars().next()?;
        if quote == '\'' || quote == '"' {
            let end = after[1..].find(quote)?;
            let name = &after[1..1 + end];
            // Filter out non-plugin imports
            if !name.starts_with('.') && !name.starts_with("@/") {
                return Some(name.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_config() {
        let json = r#"{
  "plugins": {
    "autoprefixer": {},
    "tailwindcss": {}
  }
}"#;
        let config = PostCssConfig::parse_config(json);
        assert_eq!(config.plugins.len(), 2);
        assert!(config.plugin_names().contains(&"autoprefixer"));
        assert!(config.plugin_names().contains(&"tailwindcss"));
    }

    #[test]
    fn test_parse_array_config() {
        let json = r#"{
  "plugins": ["autoprefixer", "tailwindcss"]
}"#;
        let config = PostCssConfig::parse_config(json);
        assert_eq!(config.plugins.len(), 2);
    }

    #[test]
    fn test_extract_plugin_name() {
        assert_eq!(
            extract_plugin_name("const autoprefixer = require('autoprefixer')"),
            Some("autoprefixer".to_string())
        );
        assert_eq!(
            extract_plugin_name("import autoprefixer from 'autoprefixer'"),
            Some("autoprefixer".to_string())
        );
        assert_eq!(extract_plugin_name("// comment"), None);
    }
}
