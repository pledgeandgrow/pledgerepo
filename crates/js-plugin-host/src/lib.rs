// JS Plugin Host — Vite-compatible plugin API
//
// Provides a JavaScript plugin interface that mirrors Vite's plugin hooks:
//   - resolveId(source, importer) → { id, external } | null
//   - load(id) → { code, map } | null

pub mod test_runner;
//   - transform(code, id) → { code, map } | null
//   - transformIndexHtml(html) → html | tags[]
//   - configureServer(server) → void
//   - buildStart() → void
//   - buildEnd() → void
//   - generateBundle() → void
//
// Plugins are defined as JS/TS files exporting default objects with these hooks.
// The host loads and evaluates plugin files, then calls hooks during the build pipeline.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, warn};
use boa_engine::{Context, JsValue, JsResult, Source, js_string};
use boa_engine::object::ObjectInitializer;
use boa_engine::NativeFunction;

/// A loaded JS plugin with its hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsPlugin {
    pub name: String,
    /// Whether this plugin applies to the build
    pub apply: Option<String>,
    /// Hook: resolveId(source, importer) → { id, external } | null
    pub has_resolve_id: bool,
    /// Hook: load(id) → { code, map } | null
    pub has_load: bool,
    /// Hook: transform(code, id) → { code, map } | null
    pub has_transform: bool,
    /// Hook: transformIndexHtml(html) → html | tags[]
    pub has_transform_index_html: bool,
    /// Hook: configureServer(server)
    pub has_configure_server: bool,
    /// Hook: buildStart()
    pub has_build_start: bool,
    /// Hook: buildEnd()
    pub has_build_end: bool,
    /// Hook: generateBundle()
    pub has_generate_bundle: bool,
    /// Raw source of the plugin file (for evaluation)
    pub source: String,
    /// Path to the plugin file
    pub path: PathBuf,
}

/// Result of a resolveId hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveIdResult {
    pub id: String,
    pub external: bool,
}

/// Result of a load hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadResult {
    pub code: String,
    pub map: Option<String>,
}

/// Result of a transform hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    pub code: String,
    pub map: Option<String>,
}

/// HTML tag injection for transformIndexHtml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlTag {
    pub tag: String,
    pub attrs: HashMap<String, String>,
    pub children: Option<String>,
    pub inject_to: Option<String>,
}

/// Manages a collection of JS plugins with an embedded JS runtime
pub struct JsPluginHost {
    plugins: Vec<JsPlugin>,
    /// Boa JS runtime context for evaluating and executing plugin code
    context: Context,
}

impl JsPluginHost {
    /// Create a new empty plugin host with a JS runtime
    pub fn new() -> Self {
        let mut context = Context::default();
        
        // Inject console.log support for plugin debugging
        let console_log = NativeFunction::from_copy_closure(|_this, _args, ctx| {
            let msg = _args.iter().map(|v| v.to_string(ctx).map(|s| s.to_std_string_escaped()).unwrap_or_default()).collect::<Vec<_>>().join(" ");
            info!("[plugin console] {}", msg);
            Ok(JsValue::undefined())
        });
        let console = ObjectInitializer::new(&mut context)
            .function(console_log, js_string!("log"), 0)
            .build();
        context.register_global_property(js_string!("console"), console, boa_engine::property::Attribute::all());
        
        Self {
            plugins: Vec::new(),
            context,
        }
    }

    /// Load plugins from the given paths (JS/TS files)
    pub fn load_plugins(&mut self, plugin_paths: &[String]) -> Result<()> {
        for path in plugin_paths {
            let pathbuf = PathBuf::from(path);
            if !pathbuf.exists() {
                warn!("Plugin file not found: {}", path);
                continue;
            }

            let source = std::fs::read_to_string(&pathbuf)?;
            let plugin = Self::parse_plugin(&source, pathbuf)?;
            info!("Loaded JS plugin: {}", plugin.name);
            
            // Evaluate the plugin source in the JS context
            // This makes the exported object available for hook calls
            let js_source = Source::from_bytes(source.as_str());
            if let Err(e) = self.context.eval(js_source) {
                warn!("Failed to evaluate plugin {}: {}", plugin.name, e);
            }
            
            self.plugins.push(plugin);
        }
        Ok(())
    }

    /// Parse a plugin from source code.
    /// Extracts the plugin name and which hooks are present by scanning for
    /// hook function definitions in the exported object.
    fn parse_plugin(source: &str, path: PathBuf) -> Result<JsPlugin> {
        // Extract plugin name from `name: "..."` or `name: '...'`
        let name = Self::extract_string_field(source, "name")
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("anonymous")
                    .to_string()
            });

        // Detect which hooks are present by looking for hook names as keys
        let has_resolve_id = Self::has_hook(source, "resolveId");
        let has_load = Self::has_hook(source, "load");
        let has_transform = Self::has_hook(source, "transform");
        let has_transform_index_html = Self::has_hook(source, "transformIndexHtml");
        let has_configure_server = Self::has_hook(source, "configureServer");
        let has_build_start = Self::has_hook(source, "buildStart");
        let has_build_end = Self::has_hook(source, "buildEnd");
        let has_generate_bundle = Self::has_hook(source, "generateBundle");

        // Extract apply field if present
        let apply = Self::extract_string_field(source, "apply");

        Ok(JsPlugin {
            name,
            apply,
            has_resolve_id,
            has_load,
            has_transform,
            has_transform_index_html,
            has_configure_server,
            has_build_start,
            has_build_end,
            has_generate_bundle,
            source: source.to_string(),
            path,
        })
    }

    /// Check if a hook name appears as a key in the source
    fn has_hook(source: &str, hook_name: &str) -> bool {
        // Look for patterns like: resolveId: , resolveId(, resolveId:
        source.contains(&format!("{}:", hook_name))
            || source.contains(&format!("{}(", hook_name))
            || source.contains(&format!("{} :", hook_name))
    }

    /// Extract a string field value from source (e.g., name: "my-plugin")
    fn extract_string_field(source: &str, field: &str) -> Option<String> {
        // Look for field: "value" or field: 'value'
        for quote in ['"', '\''] {
            let pattern = format!("{}:", field);
            if let Some(pos) = source.find(&pattern) {
                let rest = &source[pos + pattern.len()..];
                let trimmed = rest.trim_start();
                if trimmed.starts_with(quote) {
                    let start = 1;
                    if let Some(end) = trimmed[start..].find(quote) {
                        return Some(trimmed[start..start + end].to_string());
                    }
                }
            }
        }
        None
    }

    /// Get all loaded plugins
    pub fn plugins(&self) -> &[JsPlugin] {
        &self.plugins
    }

    /// Run buildStart hooks for all plugins
    pub fn build_start(&self) {
        for plugin in &self.plugins {
            if plugin.has_build_start {
                info!("[plugin:{}] buildStart", plugin.name);
            }
        }
    }

    /// Run buildEnd hooks for all plugins
    pub fn build_end(&self) {
        for plugin in &self.plugins {
            if plugin.has_build_end {
                info!("[plugin:{}] buildEnd", plugin.name);
            }
        }
    }

    /// Run generateBundle hooks for all plugins
    pub fn generate_bundle(&self) {
        for plugin in &self.plugins {
            if plugin.has_generate_bundle {
                info!("[plugin:{}] generateBundle", plugin.name);
            }
        }
    }

    /// Check if any plugin handles resolveId for the given source
    pub fn resolve_id(&self, source: &str, _importer: &str) -> Option<ResolveIdResult> {
        for plugin in &self.plugins {
            if plugin.has_resolve_id {
                info!("[plugin:{}] resolveId: {}", plugin.name, source);
                // In a full implementation, this would call the JS function
                // For now, we just log that a plugin could handle it
            }
        }
        None
    }

    /// Check if any plugin handles load for the given id
    pub fn load(&self, id: &str) -> Option<LoadResult> {
        for plugin in &self.plugins {
            if plugin.has_load {
                info!("[plugin:{}] load: {}", plugin.name, id);
            }
        }
        None
    }

    /// Run transform hooks for all plugins on the given code
    /// Actually calls the JS transform() function in the plugin
    pub fn transform(&mut self, code: &str, id: &str) -> Option<TransformResult> {
        let mut result_code = code.to_string();
        let mut transformed = false;

        for plugin in &self.plugins {
            if plugin.has_transform {
                info!("[plugin:{}] transform: {}", plugin.name, id);
                
                // Try to call the plugin's transform function in JS
                let js_code = format!(
                    r#"
                    (function() {{
                        try {{
                            var __pluginModule = {};
                            if (__pluginModule && typeof __pluginModule.transform === 'function') {{
                                var __result = __pluginModule.transform({}, "{}");
                                if (__result && __result.code) {{
                                    return JSON.stringify(__result);
                                }}
                            }}
                        }} catch(e) {{
                            console.log('Plugin transform error: ' + e.message);
                        }}
                        return null;
                    }})()
                    "#,
                    plugin.source,
                    serde_json::to_string(code).unwrap_or_else(|_| "\"\"".to_string()),
                    id.replace('\\', "/").replace('"', "\\\"")
                );

                match self.context.eval(Source::from_bytes(js_code.as_str())) {
                    Ok(val) => {
                        if !val.is_null() && !val.is_undefined() {
                            if let Ok(json_str) = val.to_string(&mut self.context) {
                                let json_str = json_str.to_std_string_escaped();
                                if let Ok(result) = serde_json::from_str::<TransformResult>(&json_str) {
                                    result_code = result.code;
                                    transformed = true;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[plugin:{}] transform execution error: {}", plugin.name, e);
                    }
                }
            }
        }

        if transformed {
            Some(TransformResult {
                code: result_code,
                map: None,
            })
        } else {
            None
        }
    }

    /// Run transformIndexHtml hooks for all plugins
    pub fn transform_index_html(&self, html: &str) -> (String, Vec<HtmlTag>) {
        let mut result_html = html.to_string();
        let mut tags = Vec::new();

        for plugin in &self.plugins {
            if plugin.has_transform_index_html {
                info!("[plugin:{}] transformIndexHtml", plugin.name);
                // In a full implementation, this would call the JS function
            }
        }

        (result_html, tags)
    }

    /// Check if any plugins are loaded
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for JsPluginHost {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plugin() {
        let source = r#"
            export default {
                name: "my-plugin",
                apply: "build",
                transform(code, id) {
                    return { code, map: null };
                },
                resolveId(source, importer) {
                    return { id: source, external: false };
                }
            };
        "#;

        let plugin = JsPluginHost::parse_plugin(source, PathBuf::from("test.js")).unwrap();
        assert_eq!(plugin.name, "my-plugin");
        assert_eq!(plugin.apply, Some("build".to_string()));
        assert!(plugin.has_transform);
        assert!(plugin.has_resolve_id);
        assert!(!plugin.has_load);
    }

    #[test]
    fn test_has_hook_detection() {
        assert!(JsPluginHost::has_hook("transform(code, id) {}", "transform"));
        assert!(JsPluginHost::has_hook("transform: function(code) {}", "transform"));
        assert!(!JsPluginHost::has_hook("load(code) {}", "transform"));
    }
}
