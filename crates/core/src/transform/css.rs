// CSS transforms: Lightning CSS, CSS Modules, PostCSS/Tailwind, Sass

use super::TransformOutput;
use crate::config::PledgeConfig;
use anyhow::Result;

/// Transform CSS using Lightning CSS
/// - Minification (production)
/// - Nesting transpilation
/// - Autoprefixing (browser targets)
/// - CSS Modules (if file is *.module.css)
pub(super) fn transform_css(
    source: &str,
    file_path: &str,
    is_production: bool,
    config: &PledgeConfig,
) -> Result<TransformOutput> {
    use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};

    let is_css_module = file_path.ends_with(".module.css");

    let tw_v4 = crate::tailwind_v4::TailwindV4Theme::from_css(source);
    let processed_source = if tw_v4.is_v4 {
        crate::tailwind_v4::process_tailwind_v4(source, &config.root)
    } else {
        let postcss_config = crate::postcss::PostCssConfig::from_file(&config.root);
        if let Some(ref pc) = postcss_config {
            crate::postcss::process_css(source, file_path, pc, &config.root, is_production)
        } else {
            process_postcss(source, file_path)
        }
    };

    let mut stylesheet = StyleSheet::parse(&processed_source, ParserOptions::default())
        .map_err(|e| anyhow::anyhow!("CSS parse error in {}: {}", file_path, e))?;

    if is_production {
        stylesheet
            .minify(lightningcss::stylesheet::MinifyOptions::default())
            .map_err(|e| anyhow::anyhow!("CSS minify error in {}: {}", file_path, e))?;
    } else {
        stylesheet
            .minify(lightningcss::stylesheet::MinifyOptions::default())
            .map_err(|e| anyhow::anyhow!("CSS nesting transpile error in {}: {}", file_path, e))?;
    }

    let printer_options = PrinterOptions {
        minify: is_production,
        ..Default::default()
    };

    let result = stylesheet
        .to_css(printer_options)
        .map_err(|e| anyhow::anyhow!("CSS serialize error in {}: {}", file_path, e))?;

    let css_code = if !is_production {
        crate::css_features::polyfill_container_queries(&result.code)
    } else {
        result.code
    };

    let css_modules = if is_css_module {
        let css_module_map = generate_css_module_map(&css_code, file_path);
        Some(css_module_map)
    } else {
        None
    };

    let css_code = if config.css.dark_mode != "off" {
        crate::css_advanced::generate_dark_mode_css(&css_code, &config.css.dark_mode)
    } else {
        css_code
    };

    let css_code = if is_production && config.css.optimize_custom_properties {
        crate::css_advanced::optimize_custom_properties(
            &css_code,
            config.css.minify_custom_property_names,
        )
    } else {
        css_code
    };

    let css_code = if config.css.scoped == "attribute" {
        let scope_hash = crate::css_advanced::generate_scope_hash(file_path);
        crate::css_advanced::scope_css_with_attribute(&css_code, &scope_hash)
    } else {
        css_code
    };

    let css_code = if is_css_module {
        crate::css_advanced::strip_composes(&css_code)
    } else {
        css_code
    };

    let source_map = if !is_production && config.source_maps {
        Some(crate::css_features::generate_css_source_map(
            file_path, source, &css_code,
        ))
    } else {
        None
    };

    Ok(TransformOutput {
        code: css_code,
        source_map,
        css_modules,
        is_css: true,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
        content_hash: None,
    })
}

/// Generate CSS module class name mappings by hashing class names.
/// Each class name gets a scoped name: `original` → `_original_hash6`.
fn generate_css_module_map(css: &str, file_path: &str) -> Vec<(String, String)> {
    let mut mappings = Vec::new();

    let mut seen = std::collections::HashSet::new();
    let mut search_pos = 0;
    while let Some(pos) = css[search_pos..].find('.') {
        let abs_pos = search_pos + pos + 1;
        let rest = &css[abs_pos..];

        let end = rest
            .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .unwrap_or(rest.len());
        let class_name = &rest[..end];

        if !class_name.is_empty() && !seen.contains(class_name) {
            seen.insert(class_name.to_string());

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

/// Process CSS through a PostCSS-like pipeline
/// Supports Tailwind directives (@tailwind base/components/utilities)
/// and basic PostCSS plugins (autoprefixer is handled by Lightning CSS)
fn process_postcss(source: &str, _file_path: &str) -> String {
    let mut css = source.to_string();

    if css.contains("@tailwind") {
        css = process_tailwind_directives(&css);
    }

    if css.contains("@apply") {
        css = process_tailwind_apply(&css);
    }

    css
}

/// Replace @tailwind directives with generated utility CSS
fn process_tailwind_directives(css: &str) -> String {
    let mut result = css.to_string();

    result = result.replace("@tailwind base;", TAILWIND_BASE);
    result = result.replace("@tailwind base", TAILWIND_BASE);

    result = result.replace("@tailwind components;", TAILWIND_COMPONENTS);
    result = result.replace("@tailwind components", TAILWIND_COMPONENTS);

    result = result.replace("@tailwind utilities;", TAILWIND_UTILITIES);
    result = result.replace("@tailwind utilities", TAILWIND_UTILITIES);

    result
}

/// Process @apply directives (simplified — expands common utilities)
fn process_tailwind_apply(css: &str) -> String {
    let mut result = css.to_string();

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

    for (name, props) in &utilities {
        let pattern = format!("@apply {};", name);
        let replacement = format!("/* @apply {} */ {}", name, props);
        result = result.replace(&pattern, &replacement);
    }

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
                result.replace_range(start..start + 7 + semi + 1, expanded.trim());
            } else {
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

pub(super) fn transform_sass(
    source: &str,
    file_path: &str,
    is_production: bool,
    config: &PledgeConfig,
) -> Result<TransformOutput> {
    use grass::{Options, OutputStyle};

    let is_indented = file_path.ends_with(".sass");
    let style = if is_indented {
        grass::InputSyntax::Sass
    } else {
        grass::InputSyntax::Scss
    };

    let output_style = if is_production {
        OutputStyle::Compressed
    } else {
        OutputStyle::Expanded
    };

    let options = Options::default().style(output_style).input_syntax(style);

    let css = grass::from_string(source, &options)
        .map_err(|e| anyhow::anyhow!("Sass compilation error in {}: {}", file_path, e))?;

    let is_css_module = file_path.ends_with(".module.scss") || file_path.ends_with(".module.sass");
    let css_modules = if is_css_module {
        Some(generate_css_module_map(&css, file_path))
    } else {
        None
    };

    let source_map = if !is_production && config.source_maps {
        Some(crate::css_features::generate_css_source_map(
            file_path, source, &css,
        ))
    } else {
        None
    };

    Ok(TransformOutput {
        code: css,
        source_map,
        css_modules,
        is_css: true,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
        content_hash: None,
    })
}
