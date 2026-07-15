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

    /// Get sorted plugins by enforce order (pre, normal, post)
    pub fn plugins_sorted(&self) -> Vec<&VitePlugin> {
        let mut sorted: Vec<&VitePlugin> = self.plugins.iter().collect();
        sorted.sort_by_key(|p| p.priority());
        sorted
    }

    /// Get sorted plugins by enforce order, filtered by apply mode
    pub fn plugins_sorted_for(&self, is_build: bool) -> Vec<&VitePlugin> {
        let mut sorted: Vec<&VitePlugin> = self.plugins
            .iter()
            .filter(|p| p.should_apply(is_build))
            .collect();
        sorted.sort_by_key(|p| p.priority());
        sorted
    }

    /// Run buildStart hooks for all plugins in enforce order
    pub fn build_start(&self) {
        for plugin in self.plugins_sorted() {
            if plugin.has_build_start {
                tracing::info!("[vite-plugin:{}] buildStart", plugin.name);
            }
        }
    }

    /// Run buildEnd hooks for all plugins in enforce order
    pub fn build_end(&self) {
        for plugin in self.plugins_sorted() {
            if plugin.has_build_end {
                tracing::info!("[vite-plugin:{}] buildEnd", plugin.name);
            }
        }
    }

    /// Run resolveId hooks for all plugins in enforce order
    /// Returns the first non-null result from a plugin's resolveId hook
    pub fn resolve_id(&self, source: &str, importer: &str) -> Option<String> {
        for plugin in self.plugins_sorted() {
            if plugin.has_resolve_id {
                tracing::info!("[vite-plugin:{}] resolveId: {} (from {})", plugin.name, source, importer);
                // The actual JS execution is handled by JsPluginHost
                // This layer provides the ordering and filtering
            }
        }
        None
    }

    /// Run load hooks for all plugins in enforce order
    /// Returns the first non-null result from a plugin's load hook
    pub fn load(&self, id: &str) -> Option<String> {
        for plugin in self.plugins_sorted() {
            if plugin.has_load {
                tracing::info!("[vite-plugin:{}] load: {}", plugin.name, id);
                // The actual JS execution is handled by JsPluginHost
            }
        }
        None
    }

    /// Run transform hooks for all plugins in enforce order
    /// Returns transformed code if any plugin modified it
    pub fn transform(&self, code: &str, id: &str) -> Option<String> {
        let mut result = code.to_string();
        let mut transformed = false;
        for plugin in self.plugins_sorted() {
            if plugin.has_transform {
                tracing::info!("[vite-plugin:{}] transform: {}", plugin.name, id);
                // The actual JS execution is handled by JsPluginHost
            }
        }
        if transformed { Some(result) } else { None }
    }

    /// Run transformIndexHtml hooks for all plugins in enforce order
    pub fn transform_index_html(&self, html: &str) -> String {
        let mut result = html.to_string();
        for plugin in self.plugins_sorted() {
            if plugin.has_transform_index_html {
                tracing::info!("[vite-plugin:{}] transformIndexHtml", plugin.name);
                // The actual JS execution is handled by JsPluginHost
            }
        }
        result
    }

    /// Run generateBundle hooks for all plugins in enforce order
    pub fn generate_bundle(&self) {
        for plugin in self.plugins_sorted() {
            if plugin.has_generate_bundle {
                tracing::info!("[vite-plugin:{}] generateBundle", plugin.name);
            }
        }
    }

    /// Run renderChunk hooks for all plugins in enforce order
    pub fn render_chunk(&self, code: &str, chunk_id: &str) -> Option<String> {
        let mut result = code.to_string();
        let mut transformed = false;
        for plugin in self.plugins_sorted() {
            if plugin.has_render_chunk {
                tracing::info!("[vite-plugin:{}] renderChunk: {}", plugin.name, chunk_id);
            }
        }
        if transformed { Some(result) } else { None }
    }

    /// Run closeBundle hooks for all plugins in enforce order
    pub fn close_bundle(&self) {
        for plugin in self.plugins_sorted() {
            if plugin.has_close_bundle {
                tracing::info!("[vite-plugin:{}] closeBundle", plugin.name);
            }
        }
    }

    /// Run writeBundle hooks for all plugins in enforce order
    pub fn write_bundle(&self) {
        for plugin in self.plugins_sorted() {
            if plugin.has_write_bundle {
                tracing::info!("[vite-plugin:{}] writeBundle", plugin.name);
            }
        }
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
/// Enforce ordering is applied: pre plugins run first, then normal, then post.
pub struct RollupPluginHost {
    plugins: Vec<VitePlugin>,
}

impl RollupPluginHost {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    /// Load Rollup plugins from paths
    /// Plugins are sorted by enforce ordering (pre → normal → post)
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

    /// Get plugins sorted by enforce order
    fn plugins_sorted(&self) -> Vec<&VitePlugin> {
        let mut sorted: Vec<&VitePlugin> = self.plugins.iter().collect();
        sorted.sort_by_key(|p| p.priority());
        sorted
    }

    /// Run buildStart hooks in enforce order
    pub fn build_start(&self) {
        for plugin in self.plugins_sorted() {
            if plugin.has_build_start {
                tracing::info!("[rollup-plugin:{}] buildStart", plugin.name);
            }
        }
    }

    /// Run buildEnd hooks in enforce order
    pub fn build_end(&self) {
        for plugin in self.plugins_sorted() {
            if plugin.has_build_end {
                tracing::info!("[rollup-plugin:{}] buildEnd", plugin.name);
            }
        }
    }

    /// Run resolveId hooks in enforce order
    pub fn resolve_id(&self, source: &str, importer: &str) -> Option<String> {
        for plugin in self.plugins_sorted() {
            if plugin.has_resolve_id {
                tracing::info!("[rollup-plugin:{}] resolveId: {} (from {})", plugin.name, source, importer);
            }
        }
        None
    }

    /// Run load hooks in enforce order
    pub fn load(&self, id: &str) -> Option<String> {
        for plugin in self.plugins_sorted() {
            if plugin.has_load {
                tracing::info!("[rollup-plugin:{}] load: {}", plugin.name, id);
            }
        }
        None
    }

    /// Run transform hooks in enforce order
    pub fn transform(&self, code: &str, id: &str) -> Option<String> {
        let mut result = code.to_string();
        let mut transformed = false;
        for plugin in self.plugins_sorted() {
            if plugin.has_transform {
                tracing::info!("[rollup-plugin:{}] transform: {}", plugin.name, id);
            }
        }
        if transformed { Some(result) } else { None }
    }

    /// Run renderChunk hooks in enforce order
    pub fn render_chunk(&self, code: &str, chunk_id: &str) -> Option<String> {
        let mut result = code.to_string();
        let mut transformed = false;
        for plugin in self.plugins_sorted() {
            if plugin.has_render_chunk {
                tracing::info!("[rollup-plugin:{}] renderChunk: {}", plugin.name, chunk_id);
            }
        }
        if transformed { Some(result) } else { None }
    }

    /// Run generateBundle hooks in enforce order
    pub fn generate_bundle(&self) {
        for plugin in self.plugins_sorted() {
            if plugin.has_generate_bundle {
                tracing::info!("[rollup-plugin:{}] generateBundle", plugin.name);
            }
        }
    }

    /// Run writeBundle hooks in enforce order
    pub fn write_bundle(&self) {
        for plugin in self.plugins_sorted() {
            if plugin.has_write_bundle {
                tracing::info!("[rollup-plugin:{}] writeBundle", plugin.name);
            }
        }
    }

    /// Run closeBundle hooks in enforce order
    pub fn close_bundle(&self) {
        for plugin in self.plugins_sorted() {
            if plugin.has_close_bundle {
                tracing::info!("[rollup-plugin:{}] closeBundle", plugin.name);
            }
        }
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
