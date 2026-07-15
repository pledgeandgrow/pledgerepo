// Vite plugin compatibility layer — allows running existing Vite plugins unmodified.
//
// Vite plugins follow the Rollup plugin interface with Vite-specific extensions:
//   - name, enforce ('pre' | 'post'), apply ('build' | 'serve')
//   - resolveId, load, transform, transformIndexHtml
//   - configureServer, configurePreview
//   - renderChunk, generateBundle, writeBundle, closeBundle
//   - buildStart, buildEnd, options
//
// This adapter translates Vite plugin hooks to Pledgepack's internal pipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vite plugin enforce order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Enforce {
    Pre,
    Post,
    None,
}

impl Default for Enforce {
    fn default() -> Self {
        Self::None
    }
}

/// Vite plugin apply target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Apply {
    Build,
    Serve,
    Both,
}

impl Default for Apply {
    fn default() -> Self {
        Self::Both
    }
}

/// A Vite plugin descriptor (parsed from JS plugin metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VitePlugin {
    pub name: String,
    #[serde(default)]
    pub enforce: Enforce,
    #[serde(default)]
    pub apply: Apply,
    /// Whether the plugin has a resolveId hook
    #[serde(default)]
    pub has_resolve_id: bool,
    /// Whether the plugin has a load hook
    #[serde(default)]
    pub has_load: bool,
    /// Whether the plugin has a transform hook
    #[serde(default)]
    pub has_transform: bool,
    /// Whether the plugin has a transformIndexHtml hook
    #[serde(default)]
    pub has_transform_index_html: bool,
    /// Whether the plugin has a configureServer hook
    #[serde(default)]
    pub has_configure_server: bool,
    /// Whether the plugin has a buildStart hook
    #[serde(default)]
    pub has_build_start: bool,
    /// Whether the plugin has a buildEnd hook
    #[serde(default)]
    pub has_build_end: bool,
    /// Whether the plugin has a generateBundle hook
    #[serde(default)]
    pub has_generate_bundle: bool,
    /// Whether the plugin has a renderChunk hook
    #[serde(default)]
    pub has_render_chunk: bool,
    /// Whether the plugin has an options hook
    #[serde(default)]
    pub has_options: bool,
    /// Whether the plugin has a closeBundle hook
    #[serde(default)]
    pub has_close_bundle: bool,
    /// Whether the plugin has a writeBundle hook
    #[serde(default)]
    pub has_write_bundle: bool,
    /// Additional metadata
    #[serde(default)]
    pub meta: HashMap<String, String>,
}

impl VitePlugin {
    /// Check if this plugin should run in the given mode
    pub fn should_apply(&self, is_build: bool) -> bool {
        match self.apply {
            Apply::Build => is_build,
            Apply::Serve => !is_build,
            Apply::Both => true,
        }
    }

    /// Get plugin execution order priority (lower = earlier)
    pub fn priority(&self) -> i32 {
        match self.enforce {
            Enforce::Pre => -1,
            Enforce::None => 0,
            Enforce::Post => 1,
        }
    }
}

/// Manages a collection of Vite-compatible plugins
pub struct VitePluginHost {
    plugins: Vec<VitePlugin>,
}

impl VitePluginHost {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    /// Load Vite plugins from a JS/TS config file's plugin array.
    /// In a full implementation, this would use the JS runtime to evaluate
    /// the plugin files and extract their hook metadata.
    pub fn load_from_config(&mut self, plugin_paths: &[String]) -> anyhow::Result<()> {
        for path in plugin_paths {
            let plugin = self.parse_plugin_metadata(path)?;
            self.plugins.push(plugin);
        }
        // Sort by enforce priority (pre first, then normal, then post)
        self.plugins.sort_by_key(|p| p.priority());
        Ok(())
    }

    /// Parse plugin metadata from a JS/TS file.
    /// Extracts the plugin name, hooks, and configuration.
    fn parse_plugin_metadata(&self, path: &str) -> anyhow::Result<VitePlugin> {
        let content = std::fs::read_to_string(path)?;
        let name = path
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(path)
            .trim_end_matches(".js")
            .trim_end_matches(".ts")
            .trim_end_matches(".mjs")
            .to_string();

        // Detect hooks by looking for function names in the source
        let has_resolve_id = content.contains("resolveId") || content.contains("resolveId(");
        let has_load = content.contains("load(") || content.contains("load:");
        let has_transform = content.contains("transform(") || content.contains("transform:");
        let has_transform_index_html = content.contains("transformIndexHtml");
        let has_configure_server = content.contains("configureServer");
        let has_build_start = content.contains("buildStart");
        let has_build_end = content.contains("buildEnd");
        let has_generate_bundle = content.contains("generateBundle");
        let has_render_chunk = content.contains("renderChunk");
        let has_options = content.contains("options(") || content.contains("options:");
        let has_close_bundle = content.contains("closeBundle");
        let has_write_bundle = content.contains("writeBundle");

        // Detect enforce
        let enforce = if content.contains("enforce: 'pre'") || content.contains("enforce: \"pre\"") {
            Enforce::Pre
        } else if content.contains("enforce: 'post'") || content.contains("enforce: \"post\"") {
            Enforce::Post
        } else {
            Enforce::None
        };

        // Detect apply
        let apply = if content.contains("apply: 'build'") || content.contains("apply: \"build\"") {
            Apply::Build
        } else if content.contains("apply: 'serve'") || content.contains("apply: \"serve\"") {
            Apply::Serve
        } else {
            Apply::Both
        };

        Ok(VitePlugin {
            name,
            enforce,
            apply,
            has_resolve_id,
            has_load,
            has_transform,
            has_transform_index_html,
            has_configure_server,
            has_build_start,
            has_build_end,
            has_generate_bundle,
            has_render_chunk,
            has_options,
            has_close_bundle,
            has_write_bundle,
            meta: HashMap::new(),
        })
    }

    /// Get sorted plugins for a given phase
    pub fn plugins_for_build(&self, is_build: bool) -> Vec<&VitePlugin> {
        self.plugins
            .iter()
            .filter(|p| p.should_apply(is_build))
            .collect()
    }

    /// Get all loaded plugins
    pub fn plugins(&self) -> &[VitePlugin] {
        &self.plugins
    }

    /// Check if any plugins are loaded
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for VitePluginHost {
    fn default() -> Self {
        Self::new()
    }
}

/// Rollup plugin adapter — accepts Rollup plugins directly.
/// Rollup plugins are a subset of Vite plugins (no Vite-specific hooks).
pub struct RollupPluginHost {
    plugins: Vec<VitePlugin>,
}

impl RollupPluginHost {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    /// Load Rollup plugins from paths
    pub fn load_from_config(&mut self, plugin_paths: &[String]) -> anyhow::Result<()> {
        let mut host = VitePluginHost::new();
        host.load_from_config(plugin_paths)?;
        self.plugins = host.plugins;
        Ok(())
    }

    pub fn plugins(&self) -> &[VitePlugin] {
        &self.plugins
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for RollupPluginHost {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_priority_ordering() {
        let mut host = VitePluginHost::new();
        // Simulate plugins with different enforce values
        host.plugins.push(VitePlugin {
            name: "normal".to_string(),
            enforce: Enforce::None,
            apply: Apply::Both,
            has_resolve_id: false,
            has_load: false,
            has_transform: false,
            has_transform_index_html: false,
            has_configure_server: false,
            has_build_start: false,
            has_build_end: false,
            has_generate_bundle: false,
            has_render_chunk: false,
            has_options: false,
            has_close_bundle: false,
            has_write_bundle: false,
            meta: HashMap::new(),
        });
        host.plugins.push(VitePlugin {
            name: "pre".to_string(),
            enforce: Enforce::Pre,
            apply: Apply::Both,
            has_resolve_id: false,
            has_load: false,
            has_transform: false,
            has_transform_index_html: false,
            has_configure_server: false,
            has_build_start: false,
            has_build_end: false,
            has_generate_bundle: false,
            has_render_chunk: false,
            has_options: false,
            has_close_bundle: false,
            has_write_bundle: false,
            meta: HashMap::new(),
        });
        host.plugins.push(VitePlugin {
            name: "post".to_string(),
            enforce: Enforce::Post,
            apply: Apply::Both,
            has_resolve_id: false,
            has_load: false,
            has_transform: false,
            has_transform_index_html: false,
            has_configure_server: false,
            has_build_start: false,
            has_build_end: false,
            has_generate_bundle: false,
            has_render_chunk: false,
            has_options: false,
            has_close_bundle: false,
            has_write_bundle: false,
            meta: HashMap::new(),
        });

        host.plugins.sort_by_key(|p| p.priority());
        assert_eq!(host.plugins[0].name, "pre");
        assert_eq!(host.plugins[1].name, "normal");
        assert_eq!(host.plugins[2].name, "post");
    }

    #[test]
    fn test_apply_filtering() {
        let plugin = VitePlugin {
            name: "test".to_string(),
            enforce: Enforce::None,
            apply: Apply::Build,
            has_resolve_id: false,
            has_load: false,
            has_transform: false,
            has_transform_index_html: false,
            has_configure_server: false,
            has_build_start: false,
            has_build_end: false,
            has_generate_bundle: false,
            has_render_chunk: false,
            has_options: false,
            has_close_bundle: false,
            has_write_bundle: false,
            meta: HashMap::new(),
        };

        assert!(plugin.should_apply(true));  // build mode
        assert!(!plugin.should_apply(false)); // serve mode
    }
}
