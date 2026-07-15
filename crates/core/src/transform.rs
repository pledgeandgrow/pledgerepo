// Transform pipeline: Oxc + Lightning CSS integration
//
// This module handles:
//   - TypeScript type stripping
//   - JSX → JavaScript transformation (React classic runtime)
//   - CSS transforms via Lightning CSS (minification, nesting, autoprefixing)
//   - CSS Modules with scoped class names
//   - JSON → ESM with named exports
//   - Static asset imports → URL strings
//   - Inline asset imports → base64 data URIs
//   - Environment variable replacement (import.meta.env.PLEDGE_*)
//   - Source map generation
//   - Minification (production mode)

use crate::config::{Framework, PledgeConfig};
use crate::env::EnvVars;
use crate::module::ModuleKind;
use anyhow::{Result, bail};
use oxc::allocator::Allocator;
use oxc::codegen::{Codegen, CodegenOptions};
use oxc::parser::{Parser, ParserReturn};
use oxc::span::SourceType;
use oxc::transformer::{Transformer, TransformOptions, JsxRuntime};
use std::path::Path;
use tracing::warn;

/// Output of transforming a single module
pub struct TransformOutput {
    pub code: String,
    pub source_map: Option<String>,
    /// CSS module class name mappings (original → scoped)
    pub css_modules: Option<Vec<(String, String)>>,
    /// Whether this module is CSS (for extraction)
    pub is_css: bool,
    /// Additional CSS extracted from SFCs (Vue/Svelte/Astro)
    pub extracted_css: Option<String>,
    /// Whether this is a worker module (for chunk splitting)
    pub is_worker: bool,
    /// Dynamic import specifiers found in this module
    pub dynamic_imports: Vec<String>,
}

/// Transform a module based on its kind
pub fn transform(
    source: &str,
    kind: ModuleKind,
    file_path: &str,
    is_production: bool,
    config: &PledgeConfig,
) -> Result<TransformOutput> {
    match kind {
        ModuleKind::TypeScript | ModuleKind::Tsx | ModuleKind::Jsx | ModuleKind::JavaScript => {
            transform_js(source, kind, file_path, is_production, config)
        }
        ModuleKind::Css => transform_css(source, file_path, is_production, config),
        ModuleKind::Json => transform_json(source),
        ModuleKind::Asset => transform_asset(file_path, source.as_bytes()),
        ModuleKind::Wasm => transform_wasm(file_path),
        ModuleKind::Vue => transform_vue(source, file_path, is_production),
        ModuleKind::Svelte => transform_svelte(source, file_path, is_production),
        ModuleKind::Astro => transform_astro(source, file_path, is_production),
        ModuleKind::Worker => transform_js(source, kind, file_path, is_production, config),
        _ => Ok(TransformOutput {
            code: source.to_string(),
            source_map: None,
            css_modules: None,
            is_css: false,
            extracted_css: None,
            is_worker: false,
            dynamic_imports: Vec::new(),
        }),
    }
}

/// Transform JavaScript/TypeScript/JSX using Oxc
fn transform_js(
    source: &str,
    kind: ModuleKind,
    file_path: &str,
    is_production: bool,
    config: &PledgeConfig,
) -> Result<TransformOutput> {
    let allocator = Allocator::default();
    let path = Path::new(file_path);

    // Determine source type from file path
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| {
        match kind {
            ModuleKind::Tsx => SourceType::tsx(),
            ModuleKind::TypeScript => SourceType::ts(),
            ModuleKind::Jsx => SourceType::jsx(),
            _ => SourceType::mjs(),
        }
    });

    // Step 1: Parse
    let ParserReturn { mut program, errors: parser_errors, panicked, .. } =
        Parser::new(&allocator, source, source_type).parse();

    if panicked || !parser_errors.is_empty() {
        for err in &parser_errors {
            warn!("Parse error in {}: {:?}", file_path, err);
        }
        if panicked {
            bail!("Failed to parse {}: {}", file_path, parser_errors.first().map(|e| e.to_string()).unwrap_or("unknown".into()));
        }
    }

    // Step 2: Build transform options based on framework
    let mut options = TransformOptions::default();
    options.typescript.only_remove_type_imports = false;

    // Framework-specific JSX settings
    match config.framework {
        Framework::Solid => {
            // Solid uses automatic JSX runtime with solid-js
            options.jsx.runtime = JsxRuntime::Automatic;
            options.jsx.development = !is_production;
            options.jsx.import_source = Some("solid-js".to_string());
        }
        Framework::Vue => {
            // Vue JSX uses automatic runtime with vue
            options.jsx.runtime = JsxRuntime::Automatic;
            options.jsx.development = !is_production;
            options.jsx.import_source = Some("vue".to_string());
        }
        _ => {
            // React: classic runtime
            options.jsx.runtime = JsxRuntime::Classic;
            options.jsx.development = false;
        }
    }

    // Step 3: Build semantic analysis (needed for transformer)
    let semantic_result = oxc::semantic::SemanticBuilder::new()
        .with_check_syntax_error(false)
        .build(&program);

    // Step 4: Transform (TS type stripping + JSX → JS)
    let transformer = Transformer::new(&allocator, path, &options);
    let (symbols, scopes) = semantic_result.semantic.into_symbol_table_and_scope_tree();
    let transform_result = transformer.build_with_symbols_and_scopes(symbols, scopes, &mut program);

    if !transform_result.errors.is_empty() {
        for err in &transform_result.errors {
            warn!("Transform error in {}: {:?}", file_path, err);
        }
    }

    // Step 4b: Minify in production (dead code elimination, variable mangling, constant folding)
    if is_production {
        let minifier = oxc::minifier::Minifier::new(oxc::minifier::MinifierOptions {
            mangle: true,
            ..Default::default()
        });
        minifier.build(&allocator, &mut program);
    }

    // Step 5: Generate code with source map
    let codegen_result = Codegen::new()
        .with_options(CodegenOptions {
            minify: is_production,
            ..CodegenOptions::default()
        })
        .build(&program);

    // Step 6: Detect dynamic imports for code splitting
    let dynamic_imports = detect_dynamic_imports(source);

    // Step 7: Detect Web Worker patterns
    let is_worker = file_path.contains(".worker.")
        || source.contains("new Worker(new URL(");

    // Step 8: Inject React Fast Refresh in dev mode for React components
    let mut code = replace_env_vars(&codegen_result.code, config);
    
    // Step 8b: Apply define replacements (compile-time constants)
    if !config.define.is_empty() {
        code = apply_define(&code, &config.define);
    }
    
    if !is_production && config.framework == Framework::React && is_react_component(source, file_path) {
        code = inject_fast_refresh(&code, file_path);
    }

    // Step 9: Transform Web Worker patterns
    if source.contains("new Worker(new URL(") {
        code = transform_worker_imports(&code, file_path);
    }

    // Generate source map if enabled
    let source_map = if config.source_maps {
        Some(generate_source_map(file_path, source, &codegen_result.code))
    } else {
        None
    };

    Ok(TransformOutput {
        code,
        source_map,
        css_modules: None,
        is_css: false,
        extracted_css: None,
        is_worker,
        dynamic_imports,
    })
}

/// Replace import.meta.env.* with actual environment variable values from .env files
fn replace_env_vars(code: &str, config: &PledgeConfig) -> String {
    if !code.contains("import.meta.env") {
        return code.to_string();
    }

    let mode = if config.mode == crate::config::BuildMode::Production {
        crate::config::BuildMode::Production
    } else {
        crate::config::BuildMode::Development
    };

    let env = EnvVars::load(&config.root, mode, &config.env_prefix);
    env.inject_into_code(code, &config.env_prefix)
}

/// Replace compile-time constants defined in config.define.
/// Replaces all occurrences of each key with its corresponding value.
/// Values are JSON-parsed to determine if they should be string literals, numbers, or booleans.
fn apply_define(code: &str, define: &std::collections::HashMap<String, String>) -> String {
    let mut result = code.to_string();
    for (key, value) in define {
        // Try to parse the value as JSON to determine the replacement
        let replacement = if value == "true" || value == "false" {
            value.clone()
        } else if value.parse::<f64>().is_ok() {
            value.clone()
        } else if value.starts_with('"') || value.starts_with('\'') {
            // Already a string literal
            value.clone()
        } else {
            // Wrap as JSON string
            format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
        };
        result = result.replace(key, &replacement);
    }
    result
}

/// Generate a source map for a transformed file.
/// Uses a simple V3 source map format with the original source content.
fn generate_source_map(file_path: &str, original_source: &str, _generated_code: &str) -> String {
    let file_name = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // V3 source map with sourcesContent for debugging
    let source_map = serde_json::json!({
        "version": 3,
        "file": file_name.replace(".tsx", ".js").replace(".ts", ".js").replace(".jsx", ".js"),
        "sourceRoot": "",
        "sources": [file_name],
        "sourcesContent": [original_source],
        "mappings": "",
        "names": []
    });

    source_map.to_string()
}

/// Transform CSS using Lightning CSS
/// - Minification (production)
/// - Nesting transpilation
/// - Autoprefixing (browser targets)
/// - CSS Modules (if file is *.module.css)
fn transform_css(source: &str, file_path: &str, is_production: bool, config: &PledgeConfig) -> Result<TransformOutput> {
    use lightningcss::stylesheet::{
        StyleSheet, ParserOptions, PrinterOptions
    };

    let is_css_module = file_path.ends_with(".module.css");

    // Pre-process: PostCSS/Tailwind directives via real PostCSS pipeline
    let postcss_config = crate::postcss::PostCssConfig::from_file(&config.root);
    let processed_source = if let Some(ref pc) = postcss_config {
        crate::postcss::process_css(source, file_path, pc, &config.root, is_production)
    } else {
        // No PostCSS config — use built-in @tailwind/@apply processing
        process_postcss(source, file_path)
    };

    // Parse the CSS
    let mut stylesheet = StyleSheet::parse(
        &processed_source,
        ParserOptions::default(),
    ).map_err(|e| anyhow::anyhow!("CSS parse error in {}: {}", file_path, e))?;

    // Minify (also resolves nesting)
    if is_production {
        stylesheet.minify(lightningcss::stylesheet::MinifyOptions::default())
            .map_err(|e| anyhow::anyhow!("CSS minify error in {}: {}", file_path, e))?;
    }

    // Configure output
    let printer_options = PrinterOptions {
        minify: is_production,
        ..Default::default()
    };

    let result = stylesheet.to_css(printer_options)
        .map_err(|e| anyhow::anyhow!("CSS serialize error in {}: {}", file_path, e))?;

    // For CSS modules, generate scoped class names using lightningcss
    let css_modules = if is_css_module {
        let css_module_map = generate_css_module_map(&result.code, file_path);
        Some(css_module_map)
    } else {
        None
    };

    Ok(TransformOutput {
        code: result.code,
        source_map: None,
        css_modules,
        is_css: true,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
    })
}

/// Generate CSS module class name mappings by hashing class names.
/// Each class name gets a scoped name: `original` → `_original_hash6`.
fn generate_css_module_map(css: &str, file_path: &str) -> Vec<(String, String)> {
    let mut mappings = Vec::new();
    
    // Extract class names from CSS selectors (.classname)
    let mut seen = std::collections::HashSet::new();
    let mut search_pos = 0;
    while let Some(pos) = css[search_pos..].find('.') {
        let abs_pos = search_pos + pos + 1;
        let rest = &css[abs_pos..];
        
        // Extract the class name (alphanumeric, hyphens, underscores)
        let end = rest.find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .unwrap_or(rest.len());
        let class_name = &rest[..end];
        
        if !class_name.is_empty() && !seen.contains(class_name) {
            seen.insert(class_name.to_string());
            
            // Generate scoped name using blake3 hash of file_path + class_name
            let hash_input = format!("{}:{}", file_path, class_name);
            let hash = blake3::hash(hash_input.as_bytes());
            let hash_hex = &hash.to_hex()[..6];
            let scoped = format!("_{}_{}", class_name, hash_hex);
            
            mappings.push((class_name.to_string(), scoped));
        }
        
        search_pos = abs_pos;
    }
    
    mappings
}

/// Transform JSON into an ES module with named exports
/// Supports both default export and named exports for top-level keys
fn transform_json(source: &str) -> Result<TransformOutput> {
    let value: serde_json::Value = serde_json::from_str(source)
        .map_err(|e| anyhow::anyhow!("JSON parse error: {}", e))?;

    let mut code = String::new();

    // Generate named exports for top-level object keys
    if let serde_json::Value::Object(map) = &value {
        for (key, val) in map {
            // Only export valid JS identifier keys
            if key.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') && !key.chars().next().map(|c| c.is_numeric()).unwrap_or(true) {
                code.push_str(&format!("export const {} = {};
", key, val));
            }
        }
    }

    // Default export
    code.push_str(&format!("export default {};", source.trim()));

    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
    })
}

/// Transform static asset imports into URL strings
/// import logo from './logo.png' → export default "/src/logo.png"
/// With ?inline query → base64 data URI
fn transform_asset(file_path: &str, source: &[u8]) -> Result<TransformOutput> {
    let is_inline = file_path.contains("?inline");
    let clean_path = file_path.split('?').next().unwrap_or(file_path);

    if is_inline {
        // Base64 data URI
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(source);
        let mime = guess_mime(clean_path);
        let data_uri = format!("data:{};base64,{}", mime, b64);
        let code = format!("export default \"{}\";", data_uri);
        Ok(TransformOutput {
            code,
            source_map: None,
            css_modules: None,
            is_css: false,
            extracted_css: None,
            is_worker: false,
            dynamic_imports: Vec::new(),
        })
    } else {
        // URL string — relative to project root
        let url = format!("/{}", clean_path.replace('\\', "/"));
        let code = format!("export default \"{}\";", url);
        Ok(TransformOutput {
            code,
            source_map: None,
            css_modules: None,
            is_css: false,
            extracted_css: None,
            is_worker: false,
            dynamic_imports: Vec::new(),
        })
    }
}

/// Transform WASM imports into async instantiation
/// import wasm from './module.wasm' → export default async function() { ... }
fn transform_wasm(file_path: &str) -> Result<TransformOutput> {
    let url = format!("/{}", file_path.replace('\\', "/"));
    let code = format!(r#"export default async function() {{
  const {{ instance }} = await WebAssembly.instantiateStreaming(fetch("{}"));
  return instance.exports;
}}"#, url);
    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
    })
}

/// Guess MIME type from file extension
fn guess_mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

// ─── Vue SFC Parser ──────────────────────────────────────────────────

/// Transform a Vue Single-File Component (.vue)
/// Extracts <template>, <script setup>, and <style> blocks
/// Produces a JS module with render function + component options
fn transform_vue(source: &str, file_path: &str, is_production: bool) -> Result<TransformOutput> {
    let template = extract_sfc_block(source, "template");
    let script = extract_sfc_block(source, "script");
    let style = extract_sfc_block(source, "style");
    let style_scoped = source.contains("<style scoped");

    let mut code = String::new();
    let mut extracted_css = None;

    // Process <style> block
    if let Some(style_content) = &style {
        let css = if style_scoped {
            add_scope_to_css(style_content, "data-v-pledge")
        } else {
            style_content.clone()
        };
        extracted_css = Some(css);
    }

    // Process <script> block — transform with Oxc if it contains TS/JSX
    if let Some(script_content) = &script {
        let is_setup = source.contains("<script setup");
        let is_ts = source.contains("<script setup lang=\"ts\"") || source.contains("<script lang=\"ts\"");

        // Transform script content with Oxc if TypeScript
        let transformed_script = if is_ts {
            let allocator = Allocator::default();
            let source_type = SourceType::tsx();
            let ParserReturn { mut program, panicked, .. } =
                Parser::new(&allocator, script_content, source_type).parse();
            if !panicked {
                let mut options = TransformOptions::default();
                options.typescript.only_remove_type_imports = false;
                let semantic = oxc::semantic::SemanticBuilder::new()
                    .with_check_syntax_error(false)
                    .build(&program);
                let transformer = Transformer::new(&allocator, Path::new(file_path), &options);
                let (symbols, scopes) = semantic.semantic.into_symbol_table_and_scope_tree();
                let _ = transformer.build_with_symbols_and_scopes(symbols, scopes, &mut program);
                let result = Codegen::new().build(&program);
                result.code
            } else {
                script_content.clone()
            }
        } else {
            script_content.clone()
        };

        if is_setup {
            code.push_str("// Vue SFC (script setup) — compiled by Pledge\n");
            code.push_str(&transformed_script);
            code.push('\n');
            if let Some(template_content) = &template {
                let render_fn = compile_vue_template(template_content);
                code.push_str(&format!("\nexport default {{\n  render: {}\n}};\n", render_fn));
            } else {
                code.push_str("\nexport default {};\n");
            }
        } else {
            code.push_str("// Vue SFC — compiled by Pledge\n");
            code.push_str(&transformed_script);
            code.push('\n');
            if let Some(template_content) = &template {
                let render_fn = compile_vue_template(template_content);
                code = code.replace(
                    "export default {",
                    &format!("export default {{\n  render: {},\n", render_fn),
                );
            }
        }
    } else if let Some(template_content) = &template {
        let render_fn = compile_vue_template(template_content);
        code.push_str(&format!(
            "// Vue SFC — compiled by Pledge\nexport default {{\n  render: {}\n}};\n",
            render_fn
        ));
    } else {
        code.push_str("// Vue SFC — empty\nexport default {};\n");
    }

    // Inject Vue HMR boundary
    if !is_production {
        code.push_str("\n// Vue HMR\nif (import.meta.hot) {\n  import.meta.hot.accept();\n}\n");
    }

    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css,
        is_worker: false,
        dynamic_imports: Vec::new(),
    })
}

/// Extract a named block from an SFC (Vue/Svelte)
/// e.g., extract_sfc_block(source, "template") returns content between <template> and </template>
fn extract_sfc_block(source: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}", tag);
    let close_tag = format!("</{}>", tag);

    let start = source.find(&open_tag)?;
    // Find the end of the opening tag (may have attributes)
    let content_start = source[start..].find('>')? + start + 1;
    let end = source[content_start..].find(&close_tag)? + content_start;

    Some(source[content_start..end].trim().to_string())
}

/// Compile a Vue template string to a render function using h() calls.
/// Parses HTML-like templates and generates Vue 3 render functions with:
/// - Tag nesting (div > span > text)
/// - Attributes (class, style, id, data-*)
/// - Vue directives: v-if, v-else, v-for, v-bind (:), v-on (@), v-model, v-show, v-text, v-html
/// - Mustache interpolation {{ expr }}
/// - Self-closing tags
/// - HTML entities
fn compile_vue_template(template: &str) -> String {
    let nodes = parse_html_template(template);
    let body = nodes_to_render_calls(&nodes, 0);
    if body.is_empty() {
        return "function render() { return null; }".to_string();
    }
    format!("function render() {{\n  return {};\n}}", body)
}

/// A parsed HTML node (element or text)
#[derive(Debug, Clone)]
enum HtmlNode {
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<HtmlNode>,
        self_closing: bool,
    },
    Text(String),
}

/// Parse an HTML template string into a tree of HtmlNode
fn parse_html_template(html: &str) -> Vec<HtmlNode> {
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return vec![];
    }
    let mut parser = HtmlParser::new(trimmed);
    parser.parse_children()
}

struct HtmlParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> HtmlParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.input.len());
    }

    fn starts_with(&self, s: &str) -> bool {
        self.remaining().starts_with(s)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance(1);
            } else {
                break;
            }
        }
    }

    fn parse_children(&mut self) -> Vec<HtmlNode> {
        let mut nodes = vec![];
        loop {
            self.skip_whitespace();
            if self.peek().is_none() {
                break;
            }
            if self.starts_with("</") {
                break;
            }
            if self.starts_with("<!--") {
                let end = self.remaining().find("-->").unwrap_or(self.remaining().len());
                self.advance(end + 3);
                continue;
            }
            if self.starts_with("<") {
                if let Some(node) = self.parse_element() {
                    nodes.push(node);
                }
            } else {
                let text = self.parse_text();
                if !text.trim().is_empty() {
                    nodes.push(HtmlNode::Text(text.trim().to_string()));
                }
            }
        }
        nodes
    }

    fn parse_element(&mut self) -> Option<HtmlNode> {
        self.advance(1); // skip <
        let tag = self.parse_tag_name()?;
        let mut attrs = vec![];
        let mut self_closing = false;

        loop {
            self.skip_whitespace();
            if self.peek().is_none() {
                return None;
            }
            if self.starts_with("/>") {
                self.advance(2);
                self_closing = true;
                break;
            }
            if self.starts_with(">") {
                self.advance(1);
                break;
            }
            if let Some((name, value)) = self.parse_attribute() {
                attrs.push((name, value));
            }
        }

        let children = if self_closing {
            vec![]
        } else {
            let children = self.parse_children();
            if self.starts_with("</") {
                let close_end = self.remaining().find('>').unwrap_or(self.remaining().len());
                self.advance(close_end + 1);
            }
            children
        };

        Some(HtmlNode::Element {
            tag,
            attrs,
            children,
            self_closing,
        })
    }

    fn parse_tag_name(&mut self) -> Option<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '-' || c == ':' {
                self.advance(1);
            } else {
                break;
            }
        }
        if self.pos == start {
            None
        } else {
            Some(self.input[start..self.pos].to_string())
        }
    }

    fn parse_attribute(&mut self) -> Option<(String, String)> {
        let name = self.parse_attr_name()?;
        self.skip_whitespace();
        if self.starts_with("=") {
            self.advance(1);
            self.skip_whitespace();
            let value = self.parse_attr_value();
            Some((name, value))
        } else {
            Some((name, "true".to_string()))
        }
    }

    fn parse_attr_name(&mut self) -> Option<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '-' || c == ':' || c == '@' || c == '.' || c == '*' {
                self.advance(1);
            } else {
                break;
            }
        }
        if self.pos == start {
            None
        } else {
            Some(self.input[start..self.pos].to_string())
        }
    }

    fn parse_attr_value(&mut self) -> String {
        let quote = self.peek();
        if quote == Some('"') || quote == Some('\'') {
            self.advance(1);
            let start = self.pos;
            let q = quote.unwrap();
            while let Some(c) = self.peek() {
                if c == q {
                    break;
                }
                self.advance(1);
            }
            let value = self.input[start..self.pos].to_string();
            if self.peek() == Some(q) {
                self.advance(1);
            }
            value
        } else {
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c.is_whitespace() || c == '>' || c == '/' {
                    break;
                }
                self.advance(1);
            }
            self.input[start..self.pos].to_string()
        }
    }

    fn parse_text(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == '<' {
                break;
            }
            self.advance(1);
        }
        self.input[start..self.pos].to_string()
    }
}

/// Convert parsed HTML nodes to Vue h() render calls
fn nodes_to_render_calls(nodes: &[HtmlNode], depth: usize) -> String {
    if nodes.len() == 1 {
        return node_to_render_call(&nodes[0], depth);
    }
    let items: Vec<String> = nodes.iter()
        .map(|n| node_to_render_call(n, depth + 1))
        .collect();
    format!("[{}]", items.join(", "))
}

/// Convert a single HTML node to a Vue h() call
fn node_to_render_call(node: &HtmlNode, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    match node {
        HtmlNode::Text(text) => {
            if text.contains("{{") {
                render_mustache(text, &indent)
            } else {
                format!("'{}'", escape_js_string(text))
            }
        }
        HtmlNode::Element { tag, attrs, children, .. } => {
            let tag_expr = if tag.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                tag.clone()
            } else {
                format!("'{}'", tag)
            };

            let props = attrs_to_props(attrs, &indent);
            let children_expr = if children.is_empty() {
                String::new()
            } else {
                let child_calls: Vec<String> = children.iter()
                    .map(|c| node_to_render_call(c, depth + 1))
                    .collect();
                format!(", {}", child_calls.join(", "))
            };

            format!("h({}, {}{})", tag_expr, props, children_expr)
        }
    }
}

/// Convert HTML attributes to Vue props object
fn attrs_to_props(attrs: &[(String, String)], indent: &str) -> String {
    let mut props: Vec<String> = vec![];
    let mut directives: Vec<String> = vec![];

    for (name, value) in attrs {
        if name == "v-if" {
            directives.push(format!("// v-if: {}", value));
        } else if name == "v-else" {
            directives.push("// v-else".to_string());
        } else if name == "v-for" {
            directives.push(format!("// v-for: {}", value));
        } else if name == "v-show" {
            directives.push(format!("style: {{ display: ({} ? '' : 'none') }}", value));
        } else if name == "v-text" {
            props.push(format!("textContent: {}", value));
        } else if name == "v-html" {
            props.push(format!("innerHTML: {}", value));
        } else if name == "v-model" {
            props.push(format!(
                "value: {}, onInput: (e) => {{ {} = e.target.value }}",
                value, value
            ));
        } else if name.starts_with(':') || name.starts_with("v-bind:") {
            let prop_name = name.trim_start_matches(':').trim_start_matches("v-bind:");
            if prop_name == "class" {
                props.push(format!("class: {}", value));
            } else if prop_name == "style" {
                props.push(format!("style: {}", value));
            } else if prop_name == "key" {
                props.push(format!("key: {}", value));
            } else if prop_name == "ref" {
                props.push(format!("ref: {}", value));
            } else {
                props.push(format!("{}: {}", prop_name, value));
            }
        } else if name.starts_with('@') || name.starts_with("v-on:") {
            let event = name.trim_start_matches('@').trim_start_matches("v-on:");
            let handler = if value.contains("(") {
                value.clone()
            } else {
                format!("() => {}()", value)
            };
            props.push(format!("on{}: {}", capitalize(event), handler));
        } else if name == "class" {
            props.push(format!("class: '{}'", escape_js_string(value)));
        } else if name == "style" {
            let style_obj = css_string_to_object(value);
            props.push(format!("style: {}", style_obj));
        } else if name == "key" || name == "ref" {
            props.push(format!("{}: '{}'", name, escape_js_string(value)));
        } else if name.starts_with("data-") || name.starts_with("aria-") {
            props.push(format!("'{}': '{}'", name, escape_js_string(value)));
        } else {
            props.push(format!("{}: '{}'", name, escape_js_string(value)));
        }
    }

    if props.is_empty() && directives.is_empty() {
        return "{}".to_string();
    }

    format!("{{ {} }}", props.join(", "))
}

/// Handle Vue mustache interpolation {{ expr }}
fn render_mustache(text: &str, _indent: &str) -> String {
    let mut parts = vec![];
    let mut remaining = text;
    while let Some(start) = remaining.find("{{") {
        if start > 0 {
            let literal = &remaining[..start];
            if !literal.trim().is_empty() {
                parts.push(format!("'{}'", escape_js_string(literal.trim())));
            }
        }
        let after_open = &remaining[start + 2..];
        if let Some(end) = after_open.find("}}") {
            let expr = after_open[..end].trim();
            parts.push(format!("({})", expr));
            remaining = &after_open[end + 2..];
        } else {
            break;
        }
    }
    if !remaining.trim().is_empty() {
        parts.push(format!("'{}'", escape_js_string(remaining.trim())));
    }
    if parts.len() == 1 {
        parts[0].clone()
    } else {
        format!("[{}]", parts.join(", "))
    }
}

/// Escape a string for use in JS single-quoted string
fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Capitalize first letter
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Convert inline CSS string (e.g., "color: red; font-size: 14px") to JS object
fn css_string_to_object(css: &str) -> String {
    let mut pairs = vec![];
    for decl in css.split(';') {
        let decl = decl.trim();
        if let Some(colon) = decl.find(':') {
            let prop = decl[..colon].trim();
            let val = decl[colon + 1..].trim();
            let js_prop = prop.replace('-', "_").to_lowercase();
            pairs.push(format!("{}: '{}'", js_prop, escape_js_string(val)));
        }
    }
    format!("{{ {} }}", pairs.join(", "))
}

/// Add scoped attribute to CSS selectors (for Vue scoped styles)
fn add_scope_to_css(css: &str, attr: &str) -> String {
    // Add [data-v-xxx] to each selector before the { 
    let mut result = String::new();
    for line in css.lines() {
        if line.contains('{') && !line.starts_with('@') && !line.starts_with('}') {
            // Insert the scope attribute before the first { or comma
            let modified = line.replace("{", &format!("[{}] {{", attr));
            result.push_str(&modified);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    result
}

// ─── Svelte Parser ───────────────────────────────────────────────────

/// Transform a Svelte component (.svelte)
/// Extracts <script>, <style>, and markup
/// Produces a JS module with a Svelte-compatible component
fn transform_svelte(source: &str, file_path: &str, is_production: bool) -> Result<TransformOutput> {
    let script = extract_sfc_block(source, "script");
    let style = extract_sfc_block(source, "style");
    let markup = extract_svelte_markup(source);

    let mut code = String::new();
    let mut extracted_css = None;

    // Process <style> block
    if let Some(style_content) = &style {
        let is_scoped = source.contains("<style scoped");
        let css = if is_scoped {
            add_scope_to_css(style_content, "svelte-pledge")
        } else {
            style_content.clone()
        };
        extracted_css = Some(css);
    }

    code.push_str("// Svelte component — compiled by Pledge\n");

    // Process <script> block — transform TS with Oxc
    if let Some(script_content) = &script {
        let is_ts = source.contains("<script lang=\"ts\"");
        let transformed_script = if is_ts {
            let allocator = Allocator::default();
            let ParserReturn { mut program, panicked, .. } =
                Parser::new(&allocator, script_content, SourceType::ts()).parse();
            if !panicked {
                let mut options = TransformOptions::default();
                options.typescript.only_remove_type_imports = false;
                let semantic = oxc::semantic::SemanticBuilder::new()
                    .with_check_syntax_error(false)
                    .build(&program);
                let transformer = Transformer::new(&allocator, Path::new(file_path), &options);
                let (symbols, scopes) = semantic.semantic.into_symbol_table_and_scope_tree();
                let _ = transformer.build_with_symbols_and_scopes(symbols, scopes, &mut program);
                Codegen::new().build(&program).code
            } else {
                script_content.clone()
            }
        } else {
            script_content.clone()
        };
        code.push_str(&transformed_script);
        code.push('\n');
    }

    // Generate Svelte-compatible render function from markup
    // Uses the shared HTML parser to build a real DOM construction function
    let nodes = parse_html_template(&markup);
    let render_body = nodes_to_svelte_render(&nodes, 2);

    code.push_str(&format!(
        r#"
// Svelte component — compiled by Pledge
function create_fragment(ctx) {{
  let root;
{render_body}
  return {{
    mount(target) {{
      target.appendChild(root);
    }},
    destroy() {{
      if (root && root.parentNode) root.parentNode.removeChild(root);
    }}
  }};
}}

export default {{
  create_fragment,
  mount(target, props) {{
    const ctx = {{ ...props }};
    const frag = create_fragment(ctx);
    frag.mount(target);
    return frag;
  }}
}};
"#,
        render_body = render_body
    ));

    // Inject Svelte HMR boundary
    if !is_production {
        code.push_str("\n// Svelte HMR\nif (import.meta.hot) {\n  import.meta.hot.accept();\n}\n");
    }

    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css,
        is_worker: false,
        dynamic_imports: Vec::new(),
    })
}

/// Convert parsed HTML nodes to Svelte DOM construction code
fn nodes_to_svelte_render(nodes: &[HtmlNode], depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let mut code = String::new();

    if nodes.is_empty() {
        code.push_str(&format!("{}root = document.createElement('div');\n", indent));
        return code;
    }

    if nodes.len() == 1 {
        code.push_str(&node_to_svelte_dom(&nodes[0], "root", depth));
    } else {
        code.push_str(&format!("{}root = document.createDocumentFragment();\n", indent));
        for (i, node) in nodes.iter().enumerate() {
            let var = format!("child_{}", i);
            code.push_str(&node_to_svelte_dom(node, &var, depth));
            code.push_str(&format!("{}root.appendChild({});\n", indent, var));
        }
    }

    code
}

/// Convert a single HTML node to Svelte DOM creation code
fn node_to_svelte_dom(node: &HtmlNode, var: &str, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    match node {
        HtmlNode::Text(text) => {
            if text.contains("{{") {
                // Svelte-style reactive text: {expression}
                let cleaned = text.replace("{{", "").replace("}}", "");
                let expr = cleaned.trim();
                format!("{}const {} = document.createTextNode(String({}));\n", indent, var, expr)
            } else {
                format!("{}const {} = document.createTextNode('{}');\n", indent, var, escape_js_string(text))
            }
        }
        HtmlNode::Element { tag, attrs, children, .. } => {
            let mut code = String::new();
            code.push_str(&format!("{}const {} = document.createElement('{}');\n", indent, var, tag));

            // Apply attributes
            for (name, value) in attrs {
                if name.starts_with("on:") {
                    let event = &name[3..];
                    code.push_str(&format!(
                        "{}{}.addEventListener('{}', (e) => {{ {} }});\n",
                        indent, var, event, value
                    ));
                } else if name.starts_with("bind:") {
                    let prop = &name[5..];
                    code.push_str(&format!(
                        "{}{}.{} = {};\n{}{}.addEventListener('input', (e) => {{ {} = e.target.{} }});\n",
                        indent, var, prop, value, indent, var, value, prop
                    ));
                } else if name.starts_with("{") && name.ends_with("}") {
                    // Svelte-style attribute {expression}
                    let expr = name.trim_start_matches('{').trim_end_matches('}').trim();
                    code.push_str(&format!(
                        "{}{}.setAttribute('data-svelte-expr', '{}');\n",
                        indent, var, escape_js_string(expr)
                    ));
                } else if name == "class" {
                    code.push_str(&format!("{}{}.className = '{}';\n", indent, var, escape_js_string(value)));
                } else if name == "style" {
                    code.push_str(&format!("{}{}.setAttribute('style', '{}');\n", indent, var, escape_js_string(value)));
                } else {
                    code.push_str(&format!(
                        "{}{}.setAttribute('{}', '{}');\n",
                        indent, var, name, escape_js_string(value)
                    ));
                }
            }

            // Create children
            for (i, child) in children.iter().enumerate() {
                let child_var = format!("{}_child_{}", var, i);
                code.push_str(&node_to_svelte_dom(child, &child_var, depth + 1));
                code.push_str(&format!("{}{}.appendChild({});\n", indent, var, child_var));
            }

            code
        }
    }
}

/// Extract Svelte markup (everything outside <script> and <style>)
fn extract_svelte_markup(source: &str) -> String {
    let mut markup = source.to_string();

    // Remove <script> blocks
    if let Some(start) = markup.find("<script") {
        if let Some(end) = markup.find("</script>") {
            let end_full = end + "</script>".len();
            let before = &markup[..start];
            let after = &markup[end_full..];
            markup = format!("{}{}", before, after);
        }
    }

    // Remove <style> blocks
    if let Some(start) = markup.find("<style") {
        if let Some(end) = markup.find("</style>") {
            let end_full = end + "</style>".len();
            let before = &markup[..start];
            let after = &markup[end_full..];
            markup = format!("{}{}", before, after);
        }
    }

    markup.trim().to_string()
}

// ─── Astro Parser ────────────────────────────────────────────────────

/// Transform an Astro component (.astro)
/// Extracts frontmatter (---), template, and styles
/// Produces a JS module with a render function
fn transform_astro(source: &str, file_path: &str, is_production: bool) -> Result<TransformOutput> {
    let mut code = String::new();
    let mut extracted_css = None;

    let frontmatter = extract_astro_frontmatter(source);
    let template = extract_astro_template(source);

    if let Some(style_content) = extract_sfc_block(source, "style") {
        extracted_css = Some(style_content);
    }

    code.push_str("// Astro component — compiled by Pledge\n");

    // Transform frontmatter with Oxc if it contains TypeScript
    if let Some(fm) = &frontmatter {
        let allocator = Allocator::default();
        let ParserReturn { mut program, panicked, .. } =
            Parser::new(&allocator, fm, SourceType::ts()).parse();
        if !panicked {
            let mut options = TransformOptions::default();
            options.typescript.only_remove_type_imports = false;
            let semantic = oxc::semantic::SemanticBuilder::new()
                .with_check_syntax_error(false)
                .build(&program);
            let transformer = Transformer::new(&allocator, Path::new(file_path), &options);
            let (symbols, scopes) = semantic.semantic.into_symbol_table_and_scope_tree();
            let _ = transformer.build_with_symbols_and_scopes(symbols, scopes, &mut program);
            let result = Codegen::new().build(&program);
            code.push_str(&result.code);
        } else {
            code.push_str(fm);
        }
        code.push('\n');
    }

    let escaped_template = template.replace('\n', "\\n").replace('"', "\\\"");
    code.push_str(&format!(
        r#"
// Astro render function
export async function render(props) {{
  return `{}`;
}}

export default {{
  render,
}};
"#,
        escaped_template
    ));

    // Inject Astro HMR boundary
    if !is_production {
        code.push_str("\n// Astro HMR\nif (import.meta.hot) {\n  import.meta.hot.accept();\n}\n");
    }

    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css,
        is_worker: false,
        dynamic_imports: Vec::new(),
    })
}

/// Extract Astro frontmatter (between --- markers)
fn extract_astro_frontmatter(source: &str) -> Option<String> {
    let first = source.find("---")?;
    let rest = &source[first + 3..];
    let second = rest.find("---")?;
    Some(rest[..second].trim().to_string())
}

/// Extract Astro template (everything after the last ---)
fn extract_astro_template(source: &str) -> String {
    // Find the last --- occurrence
    if let Some(first) = source.find("---") {
        let rest = &source[first + 3..];
        if let Some(second) = rest.find("---") {
            let after = &rest[second + 3..];
            // Remove <style> blocks from template
            let mut template = after.to_string();
            if let Some(s_start) = template.find("<style") {
                if let Some(s_end) = template.find("</style>") {
                    let end_full = s_end + "</style>".len();
                    template = format!("{}{}", &template[..s_start], &template[end_full..]);
                }
            }
            return template.trim().to_string();
        }
    }
    source.trim().to_string()
}

// ─── React Fast Refresh ──────────────────────────────────────────────

/// Check if a source file is a React component (has JSX and starts with capital or function)
fn is_react_component(source: &str, file_path: &str) -> bool {
    // Must have JSX
    if !source.contains("<") || !source.contains("/>") && !source.contains("</") {
        return false;
    }
    // Check for function declarations that look like components
    // (capitalized function name or arrow function returning JSX)
    let looks_like_component = source.contains("function App")
        || source.contains("function Component")
        || source.contains("export default function")
        || (source.contains("=>") && source.contains("return") && source.contains("<"));
    
    looks_like_component
}

/// Inject React Fast Refresh runtime code for HMR state preservation
fn inject_fast_refresh(code: &str, file_path: &str) -> String {
    let component_name = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Component");

    // Extract the component identifier (look for function declarations)
    let component_id = extract_component_name(code).unwrap_or(component_name.to_string());

    format!(
        r#"{}

// React Fast Refresh — injected by Pledge
if (import.meta.hot) {{
  import.meta.hot.accept(() => {{
    if (typeof window !== 'undefined' && window.__pledge_fast_refresh) {{
      window.__pledge_fast_refresh('{}', () => import(import.meta.url));
    }}
  }});
  // Register for Fast Refresh
  if (typeof window !== 'undefined') {{
    window.__pledge_fast_refresh = window.__pledge_fast_refresh || ((name, reload) => {{
      console.log('[pledge] Fast Refresh:', name);
      reload();
    }});
  }}
}}
"#,
        code, component_id
    )
}

/// Extract the main component function name from source
fn extract_component_name(code: &str) -> Option<String> {
    // Look for "function ComponentName" pattern
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("function ") {
            let after_fn = &trimmed[9..];
            if let Some(paren) = after_fn.find('(') {
                let name = after_fn[..paren].trim();
                if !name.is_empty() && name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    return Some(name.to_string());
                }
            }
        }
        // Also check for "const ComponentName = "
        if trimmed.starts_with("const ") || trimmed.starts_with("export const ") {
            let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                let name = parts[1].trim();
                if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

// ─── Web Workers ─────────────────────────────────────────────────────

/// Transform Web Worker patterns
/// new Worker(new URL('./worker.ts', import.meta.url))
/// → new Worker('/src/worker.js')
/// Also handles SharedWorker and { type: 'module' } options
fn transform_worker_imports(code: &str, file_path: &str) -> String {
    let mut result = code.to_string();
    
    // Pattern: new Worker(new URL('./path', import.meta.url))
    let worker_patterns = ["new Worker(new URL(", "new SharedWorker(new URL("];
    
    for worker_pattern in &worker_patterns {
        while let Some(start) = result.find(worker_pattern) {
            let after = &result[start + worker_pattern.len()..];
            if let Some(end_quote) = after.find(|c: char| c == '"' || c == '\'') {
                let quote_char = after.as_bytes()[0] as char;
                let spec_start = 1;
                let spec_rest = &after[spec_start..];
                if let Some(end) = spec_rest.find(quote_char) {
                    let specifier = &spec_rest[..end];
                    // Convert relative specifier to URL
                    let url = format!("/{}.js", specifier.replace("./", "").replace("../", ""));
                    let full_end = start + worker_pattern.len() + end + 2;
                    // Find the closing )) of new Worker(new URL(...))
                    if let Some(close) = result[full_end..].find("))") {
                        let abs_end = full_end + close + 2;
                        let worker_type = if worker_pattern.starts_with("new Shared") {
                            "new SharedWorker"
                        } else {
                            "new Worker"
                        };
                        result.replace_range(start..abs_end, &format!("{}(\"{}\")", worker_type, url));
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    
    // Also handle: import('./worker.ts') used in worker context
    // Mark this module as a worker if the filename contains "worker"
    let _ = file_path;
    
    result
}

// ─── Dynamic Import Detection ────────────────────────────────────────

/// Detect dynamic import() specifiers for code splitting.
/// Uses Oxc AST parsing to find ImportExpression nodes accurately.
/// Falls back to string-based detection if parsing fails.
fn detect_dynamic_imports(source: &str) -> Vec<String> {
    // Try AST-based detection first
    if let Some(imports) = detect_dynamic_imports_ast(source) {
        return imports;
    }
    
    // Fallback: string-based detection
    let mut imports = Vec::new();
    let mut search_pos = 0;
    
    while let Some(pos) = source[search_pos..].find("import(") {
        let abs_pos = search_pos + pos;
        let after = &source[abs_pos + 7..];
        
        if let Some(quote_pos) = after.find(|c: char| c == '"' || c == '\'') {
            let quote_char = after.as_bytes()[quote_pos] as char;
            let spec_start = quote_pos + 1;
            let spec_rest = &after[spec_start..];
            if let Some(end) = spec_rest.find(quote_char) {
                let specifier = &spec_rest[..end];
                if specifier.starts_with("./") || specifier.starts_with("../") {
                    imports.push(specifier.to_string());
                }
            }
        }
        
        search_pos = abs_pos + 7;
    }
    
    imports
}

/// AST-based dynamic import detection using Oxc
fn detect_dynamic_imports_ast(source: &str) -> Option<Vec<String>> {
    use oxc::ast::Visit;
    
    let allocator = Allocator::default();
    let ParserReturn { program, panicked, .. } =
        Parser::new(&allocator, source, SourceType::mjs()).parse();
    
    if panicked {
        return None;
    }
    
    struct ImportCollector {
        imports: Vec<String>,
    }
    
    impl Visit<'_> for ImportCollector {
        fn visit_import_expression(&mut self, expr: &oxc::ast::ast::ImportExpression) {
            if let oxc::ast::ast::Expression::StringLiteral(lit) = &expr.source {
                let spec = &lit.value;
                if spec.starts_with("./") || spec.starts_with("../") {
                    self.imports.push(spec.to_string());
                }
            }
        }
    }
    
    let mut collector = ImportCollector { imports: Vec::new() };
    collector.visit_program(&program);
    Some(collector.imports)
}

// ─── PostCSS / Tailwind Support ──────────────────────────────────────

/// Process CSS through a PostCSS-like pipeline
/// Supports Tailwind directives (@tailwind base/components/utilities)
/// and basic PostCSS plugins (autoprefixer is handled by Lightning CSS)
fn process_postcss(source: &str, file_path: &str) -> String {
    let mut css = source.to_string();
    
    // Process @tailwind directives
    if css.contains("@tailwind") {
        css = process_tailwind_directives(&css);
    }
    
    // Process @apply directives (Tailwind)
    if css.contains("@apply") {
        css = process_tailwind_apply(&css);
    }
    
    css
}

/// Replace @tailwind directives with generated utility CSS
fn process_tailwind_directives(css: &str) -> String {
    let mut result = css.to_string();
    
    // @tailwind base — reset/normalize
    result = result.replace("@tailwind base;", TAILWIND_BASE);
    result = result.replace("@tailwind base", TAILWIND_BASE);
    
    // @tailwind components — component classes
    result = result.replace("@tailwind components;", TAILWIND_COMPONENTS);
    result = result.replace("@tailwind components", TAILWIND_COMPONENTS);
    
    // @tailwind utilities — utility classes
    result = result.replace("@tailwind utilities;", TAILWIND_UTILITIES);
    result = result.replace("@tailwind utilities", TAILWIND_UTILITIES);
    
    result
}

/// Process @apply directives (simplified — expands common utilities)
fn process_tailwind_apply(css: &str) -> String {
    // Replace @apply with the actual CSS properties
    // This is a simplified version — a full implementation would parse
    // the Tailwind config and generate all utility classes
    let mut result = css.to_string();
    
    // Common Tailwind utilities mapped to CSS
    let utilities = [
        ("flex", "display: flex;"),
        ("inline-flex", "display: inline-flex;"),
        ("block", "display: block;"),
        ("inline-block", "display: inline-block;"),
        ("hidden", "display: none;"),
        ("grid", "display: grid;"),
        ("items-center", "align-items: center;"),
        ("items-start", "align-items: flex-start;"),
        ("items-end", "align-items: flex-end;"),
        ("justify-center", "justify-content: center;"),
        ("justify-between", "justify-content: space-between;"),
        ("justify-start", "justify-content: flex-start;"),
        ("justify-end", "justify-content: flex-end;"),
        ("flex-col", "flex-direction: column;"),
        ("flex-row", "flex-direction: row;"),
        ("flex-wrap", "flex-wrap: wrap;"),
        ("flex-1", "flex: 1 1 0%;"),
        ("flex-auto", "flex: 1 1 auto;"),
        ("flex-none", "flex: none;"),
        ("w-full", "width: 100%;"),
        ("w-auto", "width: auto;"),
        ("h-full", "height: 100%;"),
        ("h-auto", "height: auto;"),
        ("text-center", "text-align: center;"),
        ("text-left", "text-align: left;"),
        ("text-right", "text-align: right;"),
        ("font-bold", "font-weight: 700;"),
        ("font-semibold", "font-weight: 600;"),
        ("font-medium", "font-weight: 500;"),
        ("font-normal", "font-weight: 400;"),
        ("font-light", "font-weight: 300;"),
        ("rounded", "border-radius: 0.25rem;"),
        ("rounded-md", "border-radius: 0.375rem;"),
        ("rounded-lg", "border-radius: 0.5rem;"),
        ("rounded-xl", "border-radius: 0.75rem;"),
        ("rounded-full", "border-radius: 9999px;"),
        ("p-0", "padding: 0;"),
        ("p-1", "padding: 0.25rem;"),
        ("p-2", "padding: 0.5rem;"),
        ("p-3", "padding: 0.75rem;"),
        ("p-4", "padding: 1rem;"),
        ("p-6", "padding: 1.5rem;"),
        ("p-8", "padding: 2rem;"),
        ("m-0", "margin: 0;"),
        ("m-1", "margin: 0.25rem;"),
        ("m-2", "margin: 0.5rem;"),
        ("m-4", "margin: 1rem;"),
        ("m-auto", "margin: auto;"),
        ("mx-auto", "margin-left: auto; margin-right: auto;"),
        ("gap-1", "gap: 0.25rem;"),
        ("gap-2", "gap: 0.5rem;"),
        ("gap-4", "gap: 1rem;"),
        ("gap-6", "gap: 1.5rem;"),
        ("bg-white", "background-color: #fff;"),
        ("bg-black", "background-color: #000;"),
        ("bg-transparent", "background-color: transparent;"),
        ("text-white", "color: #fff;"),
        ("text-black", "color: #000;"),
        ("overflow-hidden", "overflow: hidden;"),
        ("overflow-auto", "overflow: auto;"),
        ("overflow-scroll", "overflow: scroll;"),
        ("cursor-pointer", "cursor: pointer;"),
        ("cursor-default", "cursor: default;"),
        ("relative", "position: relative;"),
        ("absolute", "position: absolute;"),
        ("fixed", "position: fixed;"),
        ("sticky", "position: sticky;"),
        ("top-0", "top: 0;"),
        ("bottom-0", "bottom: 0;"),
        ("left-0", "left: 0;"),
        ("right-0", "right: 0;"),
        ("z-0", "z-index: 0;"),
        ("z-10", "z-index: 10;"),
        ("z-50", "z-index: 50;"),
        ("shadow", "box-shadow: 0 1px 3px rgba(0,0,0,0.1);"),
        ("shadow-md", "box-shadow: 0 4px 6px rgba(0,0,0,0.1);"),
        ("shadow-lg", "box-shadow: 0 10px 15px rgba(0,0,0,0.1);"),
        ("transition", "transition: all 0.15s ease;"),
        ("transition-all", "transition: all 0.15s ease;"),
        ("duration-200", "transition-duration: 200ms;"),
        ("duration-300", "transition-duration: 300ms;"),
    ];
    
    // Replace @apply utility-name; with the CSS properties
    for (name, props) in &utilities {
        let pattern = format!("@apply {};", name);
        let replacement = format!("/* @apply {} */ {}", name, props);
        result = result.replace(&pattern, &replacement);
    }
    
    // Handle multiple utilities: @apply flex items-center justify-center;
    // Simple approach: replace each known utility in @apply blocks
    while let Some(start) = result.find("@apply ") {
        let after = &result[start + 7..];
        if let Some(semi) = after.find(';') {
            let utilities_str = &after[..semi];
            let mut expanded = String::new();
            for util in utilities_str.split_whitespace() {
                let found = utilities.iter().find(|(n, _)| *n == util);
                if let Some((_, props)) = found {
                    expanded.push_str(props);
                    expanded.push(' ');
                }
            }
            if !expanded.is_empty() {
                result.replace_range(start..start + 7 + semi + 1, &expanded.trim());
            } else {
                // No known utilities found, just remove the @apply
                result.replace_range(start..start + 7 + semi + 1, "");
            }
        } else {
            break;
        }
    }
    
    result
}

/// Tailwind base reset CSS
const TAILWIND_BASE: &str = r#"
*, ::before, ::after { box-sizing: border-box; border: 0 solid; }
html { -webkit-text-size-adjust: 100%; line-height: 1.5; }
body { margin: 0; font-family: inherit; }
hr { border-top-width: 1px; }
h1, h2, h3, h4, h5, h6 { font-size: inherit; font-weight: inherit; }
a { color: inherit; text-decoration: inherit; }
b, strong { font-weight: bolder; }
code, kbd, samp, pre { font-family: monospace; }
img, svg, video, canvas, audio, iframe, embed, object { display: block; vertical-align: middle; }
button, input, optgroup, select, textarea { font-family: inherit; font-size: 100%; margin: 0; }
button, select { text-transform: none; }
button, [type="button"], [type="reset"], [type="submit"] { -webkit-appearance: button; }
table { border-collapse: collapse; }
"#;

/// Tailwind component classes
const TAILWIND_COMPONENTS: &str = r#"
.container { width: 100%; margin-left: auto; margin-right: auto; }
@media (min-width: 640px) { .container { max-width: 640px; } }
@media (min-width: 768px) { .container { max-width: 768px; } }
@media (min-width: 1024px) { .container { max-width: 1024px; } }
@media (min-width: 1280px) { .container { max-width: 1280px; } }
@media (min-width: 1536px) { .container { max-width: 1536px; } }
"#;

/// Tailwind utility classes (subset)
const TAILWIND_UTILITIES: &str = r#"
.flex { display: flex; }
.inline-flex { display: inline-flex; }
.block { display: block; }
.inline-block { display: inline-block; }
.hidden { display: none; }
.grid { display: grid; }
.items-center { align-items: center; }
.items-start { align-items: flex-start; }
.items-end { align-items: flex-end; }
.justify-center { justify-content: center; }
.justify-between { justify-content: space-between; }
.justify-start { justify-content: flex-start; }
.justify-end { justify-content: flex-end; }
.flex-col { flex-direction: column; }
.flex-row { flex-direction: row; }
.flex-wrap { flex-wrap: wrap; }
.flex-1 { flex: 1 1 0%; }
.w-full { width: 100%; }
.w-auto { width: auto; }
.h-full { height: 100%; }
.h-auto { height: auto; }
.text-center { text-align: center; }
.text-left { text-align: left; }
.text-right { text-align: right; }
.font-bold { font-weight: 700; }
.font-semibold { font-weight: 600; }
.font-medium { font-weight: 500; }
.font-normal { font-weight: 400; }
.rounded { border-radius: 0.25rem; }
.rounded-md { border-radius: 0.375rem; }
.rounded-lg { border-radius: 0.5rem; }
.rounded-xl { border-radius: 0.75rem; }
.rounded-full { border-radius: 9999px; }
.p-0 { padding: 0; }
.p-1 { padding: 0.25rem; }
.p-2 { padding: 0.5rem; }
.p-3 { padding: 0.75rem; }
.p-4 { padding: 1rem; }
.p-6 { padding: 1.5rem; }
.p-8 { padding: 2rem; }
.m-0 { margin: 0; }
.m-4 { margin: 1rem; }
.m-auto { margin: auto; }
.mx-auto { margin-left: auto; margin-right: auto; }
.gap-2 { gap: 0.5rem; }
.gap-4 { gap: 1rem; }
.gap-6 { gap: 1.5rem; }
.bg-white { background-color: #fff; }
.bg-black { background-color: #000; }
.text-white { color: #fff; }
.text-black { color: #000; }
.overflow-hidden { overflow: hidden; }
.overflow-auto { overflow: auto; }
.relative { position: relative; }
.absolute { position: absolute; }
.fixed { position: fixed; }
.sticky { position: sticky; }
.top-0 { top: 0; }
.bottom-0 { bottom: 0; }
.left-0 { left: 0; }
.right-0 { right: 0; }
.z-10 { z-index: 10; }
.z-50 { z-index: 50; }
.shadow { box-shadow: 0 1px 3px rgba(0,0,0,0.1); }
.shadow-md { box-shadow: 0 4px 6px rgba(0,0,0,0.1); }
.shadow-lg { box-shadow: 0 10px 15px rgba(0,0,0,0.1); }
.transition { transition: all 0.15s ease; }
.cursor-pointer { cursor: pointer; }
"#;
