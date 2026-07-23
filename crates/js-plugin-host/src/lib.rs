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
use boa_engine::{Context, JsValue, Source, js_string};
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

/// Middleware registered by a plugin's configureServer hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMiddleware {
    pub plugin_name: String,
    pub source: String,
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
            
            // Strip ESM syntax and evaluate the plugin source in the JS context
            // Store the exported module object as a global variable for later hook calls
            let plugin_index = self.plugins.len();
            let global_name = format!("__pledge_plugin_{}", plugin_index);
            let js_source = strip_esm_and_assign(&source, &global_name);
            if let Err(e) = self.context.eval(Source::from_bytes(js_source.as_str())) {
                warn!("Failed to evaluate plugin {}: {}", plugin.name, e);
            }
            
            self.plugins.push(plugin);
        }
        Ok(())
    }

    /// Load all plugin files from a directory
    pub fn load_from_dir(dir: &std::path::Path) -> Result<Self> {
        let mut host = Self::new();
        if !dir.is_dir() {
            return Ok(host);
        }
        let mut plugin_paths = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "js" | "ts" | "mjs" | "cjs") {
                        plugin_paths.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
        if !plugin_paths.is_empty() {
            host.load_plugins(&plugin_paths)?;
        }
        Ok(host)
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
    /// Actually calls the JS resolveId() function in each plugin that has it
    pub fn resolve_id(&mut self, source: &str, importer: &str) -> Option<ResolveIdResult> {
        for plugin in &self.plugins {
            if plugin.has_resolve_id {
                info!("[plugin:{}] resolveId: {}", plugin.name, source);

                let global_name = format!("__pledge_plugin_{}", self.plugins.iter().position(|p| p.name == plugin.name).unwrap_or(0));
                let js_code = format!(
                    r#"
                    (function() {{
                        try {{
                            var __pluginModule = globalThis['{}'];
                            if (__pluginModule && typeof __pluginModule.resolveId === 'function') {{
                                var __result = __pluginModule.resolveId({}, {});
                                if (__result) {{
                                    return JSON.stringify(__result);
                                }}
                            }}
                        }} catch(e) {{
                            console.log('Plugin resolveId error: ' + e.message);
                        }}
                        return null;
                    }})()
                    "#,
                    global_name,
                    serde_json::to_string(source).unwrap_or_else(|_| "\"\"".to_string()),
                    serde_json::to_string(importer).unwrap_or_else(|_| "\"\"".to_string())
                );

                match self.context.eval(Source::from_bytes(js_code.as_str())) {
                    Ok(val) => {
                        if !val.is_null() && !val.is_undefined() {
                            if let Ok(json_str) = val.to_string(&mut self.context) {
                                let json_str = json_str.to_std_string_escaped();
                                if let Ok(result) = serde_json::from_str::<ResolveIdResult>(&json_str) {
                                    return Some(result);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[plugin:{}] resolveId execution error: {}", plugin.name, e);
                    }
                }
            }
        }
        None
    }

    /// Check if any plugin handles load for the given id
    /// Actually calls the JS load() function in each plugin that has it
    pub fn load(&mut self, id: &str) -> Option<LoadResult> {
        for plugin in &self.plugins {
            if plugin.has_load {
                info!("[plugin:{}] load: {}", plugin.name, id);

                let global_name = format!("__pledge_plugin_{}", self.plugins.iter().position(|p| p.name == plugin.name).unwrap_or(0));
                let js_code = format!(
                    r#"
                    (function() {{
                        try {{
                            var __pluginModule = globalThis['{}'];
                            if (__pluginModule && typeof __pluginModule.load === 'function') {{
                                var __result = __pluginModule.load({});
                                if (__result && __result.code) {{
                                    return JSON.stringify(__result);
                                }}
                            }}
                        }} catch(e) {{
                            console.log('Plugin load error: ' + e.message);
                        }}
                        return null;
                    }})()
                    "#,
                    global_name,
                    serde_json::to_string(id).unwrap_or_else(|_| "\"\"".to_string())
                );

                match self.context.eval(Source::from_bytes(js_code.as_str())) {
                    Ok(val) => {
                        if !val.is_null() && !val.is_undefined() {
                            if let Ok(json_str) = val.to_string(&mut self.context) {
                                let json_str = json_str.to_std_string_escaped();
                                if let Ok(result) = serde_json::from_str::<LoadResult>(&json_str) {
                                    return Some(result);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[plugin:{}] load execution error: {}", plugin.name, e);
                    }
                }
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
                let global_name = format!("__pledge_plugin_{}", self.plugins.iter().position(|p| p.name == plugin.name).unwrap_or(0));
                let js_code = format!(
                    r#"
                    (function() {{
                        try {{
                            var __pluginModule = globalThis['{}'];
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
                    global_name,
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
    /// Actually calls the JS transformIndexHtml() function and collects HTML modifications
    pub fn transform_index_html(&mut self, html: &str) -> (String, Vec<HtmlTag>) {
        let mut result_html = html.to_string();
        let mut tags = Vec::new();

        for plugin in &self.plugins {
            if plugin.has_transform_index_html {
                info!("[plugin:{}] transformIndexHtml", plugin.name);

                let global_name = format!("__pledge_plugin_{}", self.plugins.iter().position(|p| p.name == plugin.name).unwrap_or(0));
                let js_code = format!(
                    r#"
                    (function() {{
                        try {{
                            var __pluginModule = globalThis['{}'];
                            if (__pluginModule && typeof __pluginModule.transformIndexHtml === 'function') {{
                                var __result = __pluginModule.transformIndexHtml({});
                                if (__result) {{
                                    if (typeof __result === 'string') {{
                                        return JSON.stringify({{ html: __result, tags: [] }});
                                    }} else if (Array.isArray(__result)) {{
                                        return JSON.stringify({{ html: null, tags: __result }});
                                    }} else if (__result.html || __result.tags) {{
                                        return JSON.stringify(__result);
                                    }}
                                }}
                            }}
                        }} catch(e) {{
                            console.log('Plugin transformIndexHtml error: ' + e.message);
                        }}
                        return null;
                    }})()
                    "#,
                    global_name,
                    serde_json::to_string(html).unwrap_or_else(|_| "\"\"".to_string())
                );

                match self.context.eval(Source::from_bytes(js_code.as_str())) {
                    Ok(val) => {
                        if !val.is_null() && !val.is_undefined() {
                            if let Ok(json_str) = val.to_string(&mut self.context) {
                                let json_str = json_str.to_std_string_escaped();
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                    // If html is returned as string, replace result_html
                                    if let Some(html_val) = parsed.get("html").and_then(|h| h.as_str()) {
                                        if !html_val.is_empty() {
                                            result_html = html_val.to_string();
                                        }
                                    }
                                    // Parse tags array
                                    if let Some(tags_arr) = parsed.get("tags").and_then(|t| t.as_array()) {
                                        for tag_val in tags_arr {
                                            let mut tag = HtmlTag {
                                                tag: tag_val.get("tag").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                                                attrs: HashMap::new(),
                                                children: tag_val.get("children").and_then(|c| c.as_str()).map(|s| s.to_string()),
                                                inject_to: tag_val.get("injectTo").and_then(|i| i.as_str()).map(|s| s.to_string()),
                                            };
                                            if let Some(attrs) = tag_val.get("attrs").and_then(|a| a.as_object()) {
                                                for (k, v) in attrs {
                                                    tag.attrs.insert(k.clone(), v.as_str().unwrap_or("").to_string());
                                                }
                                            }
                                            if !tag.tag.is_empty() {
                                                tags.push(tag);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[plugin:{}] transformIndexHtml execution error: {}", plugin.name, e);
                    }
                }
            }
        }

        (result_html, tags)
    }

    /// Run configureServer hooks for all plugins
    /// Executes the JS configureServer(server) function, passing a minimal server object
    /// that allows plugins to register middleware, add routes, etc.
    pub fn configure_server(&mut self) -> Vec<ServerMiddleware> {
        let mut middlewares = Vec::new();

        for plugin in &self.plugins {
            if plugin.has_configure_server {
                info!("[plugin:{}] configureServer", plugin.name);

                // Execute the configureServer hook in JS
                // The plugin can register middleware by calling server.use(fn)
                let global_name = format!("__pledge_plugin_{}", self.plugins.iter().position(|p| p.name == plugin.name).unwrap_or(0));
                let js_code = format!(
                    r#"
                    (function() {{
                        try {{
                            var __pluginModule = globalThis['{}'];
                            if (__pluginModule && typeof __pluginModule.configureServer === 'function') {{
                                var __registered = [];
                                var __server = {{
                                    use: function(fn) {{
                                        if (typeof fn === 'function') __registered.push(fn.toString());
                                    }},
                                    on: function(event, fn) {{
                                        if (typeof fn === 'function') __registered.push('on:' + event + ':' + fn.toString());
                                    }},
                                }};
                                __pluginModule.configureServer(__server);
                                if (__registered.length > 0) {{
                                    return JSON.stringify(__registered);
                                }}
                            }}
                        }} catch(e) {{
                            console.log('Plugin configureServer error: ' + e.message);
                        }}
                        return null;
                    }})()
                    "#,
                    global_name
                );

                match self.context.eval(Source::from_bytes(js_code.as_str())) {
                    Ok(val) => {
                        if !val.is_null() && !val.is_undefined() {
                            if let Ok(json_str) = val.to_string(&mut self.context) {
                                let json_str = json_str.to_std_string_escaped();
                                if let Ok(fns) = serde_json::from_str::<Vec<String>>(&json_str) {
                                    for fn_source in fns {
                                        middlewares.push(ServerMiddleware {
                                            plugin_name: plugin.name.clone(),
                                            source: fn_source,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[plugin:{}] configureServer execution error: {}", plugin.name, e);
                    }
                }
            }
        }

        middlewares
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

/// Strip ESM syntax from plugin source and assign the exported object to a global variable.
/// Converts `export default { ... }` to `globalThis['name'] = { ... }`
/// and `export const/let/var/function/class` to their non-export equivalents.
fn strip_esm_and_assign(source: &str, global_name: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut in_string = false;
    let mut string_delim = '\0';
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        if !in_string && (ch == '"' || ch == '\'' || ch == '`') {
            in_string = true;
            string_delim = ch;
            result.push(ch);
            continue;
        }
        if in_string {
            if ch == '\\' {
                result.push(ch);
                if let Some(&next) = chars.peek() {
                    result.push(next);
                    chars.next();
                }
                continue;
            }
            if ch == string_delim {
                in_string = false;
                string_delim = '\0';
            }
            result.push(ch);
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    result.push(c);
                    chars.next();
                    break;
                }
                chars.next();
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(c) = chars.next() {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
            continue;
        }

        result.push(ch);
    }

    let result = result
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("export default") {
                line.replace("export default", &format!("globalThis['{}'] =", global_name))
            } else if trimmed.starts_with("export const") || trimmed.starts_with("export let") || trimmed.starts_with("export var") {
                line.replace("export ", "")
            } else if trimmed.starts_with("export function") || trimmed.starts_with("export class") {
                line.replace("export ", "")
            } else if trimmed.starts_with("import ") {
                String::new()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    result
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
