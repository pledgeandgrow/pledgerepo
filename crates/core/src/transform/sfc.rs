// Single-File Component transforms: Vue, Svelte, Astro

use super::TransformOutput;
use anyhow::Result;
use oxc::allocator::Allocator;
use oxc::codegen::Codegen;
use oxc::parser::{Parser, ParserReturn};
use oxc::span::SourceType;
use oxc::transformer::{TransformOptions, Transformer};
use std::path::Path;

// ─── Vue SFC Parser ──────────────────────────────────────────────────

/// Transform a Vue Single-File Component (.vue)
/// Extracts <template>, <script setup>, and <style> blocks
/// Produces a JS module with render function + component options
pub(super) fn transform_vue(
    source: &str,
    file_path: &str,
    is_production: bool,
) -> Result<TransformOutput> {
    let template = extract_sfc_block(source, "template");
    let script = extract_sfc_block(source, "script");
    let style = extract_sfc_block(source, "style");
    let style_scoped = source.contains("<style scoped");

    let mut code = String::new();
    let mut extracted_css = None;

    if let Some(style_content) = &style {
        let css = if style_scoped {
            add_scope_to_css(style_content, "data-v-pledge")
        } else {
            style_content.clone()
        };
        extracted_css = Some(css);
    }

    if let Some(script_content) = &script {
        let is_setup = source.contains("<script setup");
        let is_ts =
            source.contains("<script setup lang=\"ts\"") || source.contains("<script lang=\"ts\"");

        let transformed_script = if is_ts {
            let allocator = Allocator::default();
            let source_type = SourceType::tsx();
            let ParserReturn {
                mut program,
                panicked,
                ..
            } = Parser::new(&allocator, script_content, source_type).parse();
            if !panicked {
                let mut options = TransformOptions::default();
                options.typescript.only_remove_type_imports = false;
                let semantic = oxc::semantic::SemanticBuilder::new()
                    .with_check_syntax_error(false)
                    .build(&program);
                let transformer = Transformer::new(&allocator, Path::new(file_path), &options);
                let scoping = semantic.semantic.into_scoping();
                let _ = transformer.build_with_scoping(scoping, &mut program);
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
                code.push_str(&format!(
                    "\nexport default {{\n  render: {}\n}};\n",
                    render_fn
                ));
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

    if !is_production {
        code.push_str(
            r#"
// Vue HMR — component-level hot replacement
if (import.meta.hot) {
  const __vue_component = __pledge_vue_components && __pledge_vue_components['"#,
        );
        code.push_str(file_path);
        code.push_str(
            r#"'];
  if (__vue_component && __vue_component.__hmr_id) {
    import.meta.hot.accept((newModule) => {
      if (newModule && newModule.default) {
        // Swap render function on existing component instances
        const newRender = newModule.default.render;
        if (newRender) {
          __vue_component.render = newRender;
          // Force re-render of all mounted instances
          if (__vue_component.__instances) {
            __vue_component.__instances.forEach(instance => {
              if (instance && instance.forceUpdate) {
                instance.forceUpdate();
              }
            });
          }
        }
      }
    });
  }
  import.meta.hot.accept();
}
"#,
        );
    }

    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css,
        is_worker: false,
        dynamic_imports: Vec::new(),
        content_hash: None,
    })
}

/// Extract a named block from an SFC (Vue/Svelte)
/// e.g., extract_sfc_block(source, "template") returns content between <template> and </template>
fn extract_sfc_block(source: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}", tag);
    let close_tag = format!("</{}>", tag);

    let start = source.find(&open_tag)?;
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
        #[allow(dead_code)]
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
                let end = self
                    .remaining()
                    .find("-->")
                    .unwrap_or(self.remaining().len());
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
            self.peek()?;
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
    let items: Vec<String> = nodes
        .iter()
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
        HtmlNode::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            let tag_expr = if tag
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                tag.clone()
            } else {
                format!("'{}'", tag)
            };

            let props = attrs_to_props(attrs, &indent);
            let children_expr = if children.is_empty() {
                String::new()
            } else {
                let child_calls: Vec<String> = children
                    .iter()
                    .map(|c| node_to_render_call(c, depth + 1))
                    .collect();
                format!(", {}", child_calls.join(", "))
            };

            format!("h({}, {}{})", tag_expr, props, children_expr)
        }
    }
}

/// Convert HTML attributes to Vue props object
fn attrs_to_props(attrs: &[(String, String)], _indent: &str) -> String {
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
    let mut result = String::new();
    for line in css.lines() {
        if line.contains('{') && !line.starts_with('@') && !line.starts_with('}') {
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
pub(super) fn transform_svelte(
    source: &str,
    file_path: &str,
    is_production: bool,
) -> Result<TransformOutput> {
    let script = extract_sfc_block(source, "script");
    let style = extract_sfc_block(source, "style");
    let markup = extract_svelte_markup(source);

    let mut code = String::new();
    let mut extracted_css = None;

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

    if let Some(script_content) = &script {
        let is_ts = source.contains("<script lang=\"ts\"");
        let transformed_script = if is_ts {
            let allocator = Allocator::default();
            let ParserReturn {
                mut program,
                panicked,
                ..
            } = Parser::new(&allocator, script_content, SourceType::ts()).parse();
            if !panicked {
                let mut options = TransformOptions::default();
                options.typescript.only_remove_type_imports = false;
                let semantic = oxc::semantic::SemanticBuilder::new()
                    .with_check_syntax_error(false)
                    .build(&program);
                let transformer = Transformer::new(&allocator, Path::new(file_path), &options);
                let scoping = semantic.semantic.into_scoping();
                let _ = transformer.build_with_scoping(scoping, &mut program);
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

    if !is_production {
        code.push_str(
            r#"
// Svelte HMR — component-level hot replacement
if (import.meta.hot) {
  import.meta.hot.accept((newModule) => {
    if (newModule && newModule.default) {
      // Find all mounted Svelte components and replace them
      const __svelte_registry = window.__pledge_svelte_components;
      if (__svelte_registry) {
        for (const key of Object.keys(__svelte_registry)) {
          const entry = __svelte_registry[key];
          if (entry && entry.component === __pledge_current_component) {
            // Destroy old component
            if (entry.fragment && entry.fragment.destroy) {
              entry.fragment.destroy();
            }
            // Remount with new component
            const target = entry.target;
            if (target && newModule.default.mount) {
              const newFragment = newModule.default.mount(target, entry.props || {});
              entry.fragment = newFragment;
              entry.component = newModule.default;
            }
          }
        }
      }
    }
  });
}
"#,
        );
    }

    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css,
        is_worker: false,
        dynamic_imports: Vec::new(),
        content_hash: None,
    })
}

/// Convert parsed HTML nodes to Svelte DOM construction code
fn nodes_to_svelte_render(nodes: &[HtmlNode], depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let mut code = String::new();

    if nodes.is_empty() {
        code.push_str(&format!(
            "{}root = document.createElement('div');\n",
            indent
        ));
        return code;
    }

    if nodes.len() == 1 {
        code.push_str(&node_to_svelte_dom(&nodes[0], "root", depth));
    } else {
        code.push_str(&format!(
            "{}root = document.createDocumentFragment();\n",
            indent
        ));
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
                let cleaned = text.replace("{{", "").replace("}}", "");
                let expr = cleaned.trim();
                format!(
                    "{}const {} = document.createTextNode(String({}));\n",
                    indent, var, expr
                )
            } else {
                format!(
                    "{}const {} = document.createTextNode('{}');\n",
                    indent,
                    var,
                    escape_js_string(text)
                )
            }
        }
        HtmlNode::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            let mut code = String::new();
            code.push_str(&format!(
                "{}const {} = document.createElement('{}');\n",
                indent, var, tag
            ));

            for (name, value) in attrs {
                if let Some(event) = name.strip_prefix("on:") {
                    code.push_str(&format!(
                        "{}{}.addEventListener('{}', (e) => {{ {} }});\n",
                        indent, var, event, value
                    ));
                } else if let Some(prop) = name.strip_prefix("bind:") {
                    code.push_str(&format!(
                        "{}{}.{} = {};\n{}{}.addEventListener('input', (e) => {{ {} = e.target.{} }});\n",
                        indent, var, prop, value, indent, var, value, prop
                    ));
                } else if name.starts_with("{") && name.ends_with("}") {
                    let expr = name.trim_start_matches('{').trim_end_matches('}').trim();
                    code.push_str(&format!(
                        "{}{}.setAttribute('data-svelte-expr', '{}');\n",
                        indent,
                        var,
                        escape_js_string(expr)
                    ));
                } else if name == "class" {
                    code.push_str(&format!(
                        "{}{}.className = '{}';\n",
                        indent,
                        var,
                        escape_js_string(value)
                    ));
                } else if name == "style" {
                    code.push_str(&format!(
                        "{}{}.setAttribute('style', '{}');\n",
                        indent,
                        var,
                        escape_js_string(value)
                    ));
                } else {
                    code.push_str(&format!(
                        "{}{}.setAttribute('{}', '{}');\n",
                        indent,
                        var,
                        name,
                        escape_js_string(value)
                    ));
                }
            }

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

    if let Some(start) = markup.find("<script")
        && let Some(end) = markup.find("</script>")
    {
        let end_full = end + "</script>".len();
        let before = &markup[..start];
        let after = &markup[end_full..];
        markup = format!("{}{}", before, after);
    }

    if let Some(start) = markup.find("<style")
        && let Some(end) = markup.find("</style>")
    {
        let end_full = end + "</style>".len();
        let before = &markup[..start];
        let after = &markup[end_full..];
        markup = format!("{}{}", before, after);
    }

    markup.trim().to_string()
}

// ─── Astro Parser ────────────────────────────────────────────────────

/// Transform an Astro component (.astro)
/// Extracts frontmatter (---), template, and styles
/// Produces a JS module with a render function
pub(super) fn transform_astro(
    source: &str,
    file_path: &str,
    is_production: bool,
) -> Result<TransformOutput> {
    let mut code = String::new();
    let mut extracted_css = None;

    let frontmatter = extract_astro_frontmatter(source);
    let template = extract_astro_template(source);

    if let Some(style_content) = extract_sfc_block(source, "style") {
        extracted_css = Some(style_content);
    }

    code.push_str("// Astro component — compiled by Pledge\n");

    if let Some(fm) = &frontmatter {
        let allocator = Allocator::default();
        let ParserReturn {
            mut program,
            panicked,
            ..
        } = Parser::new(&allocator, fm, SourceType::ts()).parse();
        if !panicked {
            let mut options = TransformOptions::default();
            options.typescript.only_remove_type_imports = false;
            let semantic = oxc::semantic::SemanticBuilder::new()
                .with_check_syntax_error(false)
                .build(&program);
            let transformer = Transformer::new(&allocator, Path::new(file_path), &options);
            let scoping = semantic.semantic.into_scoping();
            let _ = transformer.build_with_scoping(scoping, &mut program);
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
        content_hash: None,
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
    if let Some(first) = source.find("---") {
        let rest = &source[first + 3..];
        if let Some(second) = rest.find("---") {
            let after = &rest[second + 3..];
            let mut template = after.to_string();
            if let Some(s_start) = template.find("<style")
                && let Some(s_end) = template.find("</style>")
            {
                let end_full = s_end + "</style>".len();
                template = format!("{}{}", &template[..s_start], &template[end_full..]);
            }
            return template.trim().to_string();
        }
    }
    source.trim().to_string()
}
