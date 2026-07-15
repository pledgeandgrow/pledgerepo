// CSS framework integrations — Tailwind v4, UnoCSS, Panda CSS, Vanilla Extract.
//
// Each integration provides:
//   - Detection (check if framework is installed)
//   - Config file generation
//   - PostCSS plugin configuration
//   - Required dependencies

use std::path::Path;
use std::collections::HashMap;

/// Supported CSS frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssFramework {
    Tailwind,
    UnoCSS,
    PandaCSS,
    VanillaExtract,
    None,
}

impl CssFramework {
    pub fn as_str(&self) -> &'static str {
        match self {
            CssFramework::Tailwind => "tailwind",
            CssFramework::UnoCSS => "unocss",
            CssFramework::PandaCSS => "panda-css",
            CssFramework::VanillaExtract => "vanilla-extract",
            CssFramework::None => "none",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CssFramework::Tailwind => "Tailwind CSS",
            CssFramework::UnoCSS => "UnoCSS",
            CssFramework::PandaCSS => "Panda CSS",
            CssFramework::VanillaExtract => "Vanilla Extract",
            CssFramework::None => "None",
        }
    }
}

/// CSS framework configuration
#[derive(Debug, Clone)]
pub struct CssFrameworkConfig {
    pub framework: CssFramework,
    pub dependencies: Vec<&'static str>,
    pub dev_dependencies: Vec<&'static str>,
    pub config_file: Option<&'static str>,
    pub config_content: String,
    pub postcss_plugins: Vec<&'static str>,
    pub description: &'static str,
}

/// Detect CSS framework from package.json dependencies
pub fn detect_css_framework(root: &Path) -> CssFramework {
    let pkg_path = root.join("package.json");
    if !pkg_path.exists() {
        return CssFramework::None;
    }

    let content = match std::fs::read_to_string(&pkg_path) {
        Ok(c) => c,
        Err(_) => return CssFramework::None,
    };

    // Check for config files first
    if root.join("tailwind.config.js").exists() || root.join("tailwind.config.ts").exists() {
        return CssFramework::Tailwind;
    }
    if root.join("uno.config.ts").exists() || root.join("unocss.config.ts").exists() {
        return CssFramework::UnoCSS;
    }
    if root.join("panda.config.ts").exists() {
        return CssFramework::PandaCSS;
    }

    // Check package.json for dependencies
    if content.contains("\"tailwindcss\"") || content.contains("\"@tailwindcss/") {
        return CssFramework::Tailwind;
    }
    if content.contains("\"unocss\"") || content.contains("\"@unocss/") {
        return CssFramework::UnoCSS;
    }
    if content.contains("\"@pandacss/dev\"") || content.contains("\"@pandacss/node\"") {
        return CssFramework::PandaCSS;
    }
    if content.contains("\"@vanilla-extract/") {
        return CssFramework::VanillaExtract;
    }

    CssFramework::None
}

/// Get configuration for a CSS framework
pub fn get_css_config(framework: CssFramework) -> CssFrameworkConfig {
    match framework {
        CssFramework::Tailwind => CssFrameworkConfig {
            framework,
            dependencies: vec![],
            dev_dependencies: vec!["tailwindcss", "@tailwindcss/postcss", "postcss", "autoprefixer"],
            config_file: Some("tailwind.config.ts"),
            config_content: r#"import type { Config } from 'tailwindcss'

export default {
  content: ['./index.html', './src/**/*.{ts,tsx,js,jsx,vue,svelte}'],
  theme: {
    extend: {},
  },
  plugins: [],
} satisfies Config"#.to_string(),
            postcss_plugins: vec!["@tailwindcss/postcss", "autoprefixer"],
            description: "Utility-first CSS framework",
        },
        CssFramework::UnoCSS => CssFrameworkConfig {
            framework,
            dependencies: vec!["unocss"],
            dev_dependencies: vec![],
            config_file: Some("uno.config.ts"),
            config_content: r#"import { defineConfig, presetUno } from 'unocss'

export default defineConfig({
  presets: [presetUno()],
})"#.to_string(),
            postcss_plugins: vec!["unocss/postcss"],
            description: "Atomic CSS engine with full preset support",
        },
        CssFramework::PandaCSS => CssFrameworkConfig {
            framework,
            dependencies: vec!["@pandacss/node"],
            dev_dependencies: vec!["@pandacss/dev"],
            config_file: Some("panda.config.ts"),
            config_content: r#"import { defineConfig } from '@pandacss/dev'

export default defineConfig({
  // Your configuration
})"#.to_string(),
            postcss_plugins: vec![],
            description: "Build-time CSS-in-JS with type safety",
        },
        CssFramework::VanillaExtract => CssFrameworkConfig {
            framework,
            dependencies: vec!["@vanilla-extract/css"],
            dev_dependencies: vec!["@vanilla-extract/webpack-plugin"],
            config_file: None,
            config_content: String::new(),
            postcss_plugins: vec![],
            description: "Zero-runtime CSS-in-JS with full type safety",
        },
        CssFramework::None => CssFrameworkConfig {
            framework,
            dependencies: vec![],
            dev_dependencies: vec![],
            config_file: None,
            config_content: String::new(),
            postcss_plugins: vec![],
            description: "Plain CSS",
        },
    }
}

/// Generate a PostCSS config file
pub fn generate_postcss_config(plugins: &[&str]) -> String {
    if plugins.is_empty() {
        return r#"export default {
  plugins: {},
}"#.to_string();
    }

    let mut entries = Vec::new();
    for plugin in plugins {
        entries.push(format!("    '{}': {{}},", plugin));
    }

    format!(
        "export default {{\n  plugins: {{\n{}\n  }},\n}}",
        entries.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_no_framework() {
        let result = detect_css_framework(Path::new("/nonexistent"));
        assert_eq!(result, CssFramework::None);
    }

    #[test]
    fn test_get_css_config_tailwind() {
        let config = get_css_config(CssFramework::Tailwind);
        assert!(config.dev_dependencies.contains(&"tailwindcss"));
        assert!(config.config_file.is_some());
    }

    #[test]
    fn test_generate_postcss_config() {
        let config = generate_postcss_config(&["autoprefixer", "tailwindcss"]);
        assert!(config.contains("autoprefixer"));
        assert!(config.contains("tailwindcss"));
    }
}
