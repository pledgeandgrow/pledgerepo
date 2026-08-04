// JavaScript/TypeScript/JSX transforms via Oxc

use crate::config::{Framework, PledgeConfig};
use crate::module::ModuleKind;
use super::TransformOutput;
use super::env;
use super::utils;
use anyhow::{Result, bail};
use oxc::allocator::Allocator;
use oxc::codegen::{Codegen, CodegenOptions};
use oxc::parser::{Parser, ParserReturn};
use oxc::span::SourceType;
use oxc::transformer::{JsxRuntime, TransformOptions, Transformer};
use std::path::Path;
use tracing::warn;

/// Transform JavaScript/TypeScript/JSX using Oxc
pub(super) fn transform_js(
    source: &str,
    kind: ModuleKind,
    file_path: &str,
    is_production: bool,
    config: &PledgeConfig,
) -> Result<TransformOutput> {
    let allocator = Allocator::default();
    let path = Path::new(file_path);

    let source_type = SourceType::from_path(path).unwrap_or_else(|_| match kind {
        ModuleKind::Tsx => SourceType::tsx(),
        ModuleKind::TypeScript => SourceType::ts(),
        ModuleKind::Jsx => SourceType::jsx(),
        _ => SourceType::mjs(),
    });

    let ParserReturn {
        mut program,
        diagnostics: parser_errors,
        panicked,
        ..
    } = Parser::new(&allocator, source, source_type).parse();

    if panicked || !parser_errors.is_empty() {
        for err in &parser_errors {
            warn!("Parse error in {}: {:?}", file_path, err);
        }
        if panicked {
            bail!(
                "Failed to parse {}: {}",
                file_path,
                parser_errors
                    .first()
                    .map(|e| e.to_string())
                    .unwrap_or("unknown".into())
            );
        }
    }

    let mut options = TransformOptions::default();
    options.typescript.only_remove_type_imports = false;

    match config.framework {
        Framework::Solid => {
            options.jsx.runtime = JsxRuntime::Automatic;
            options.jsx.development = !is_production;
            options.jsx.import_source = Some("solid-js".to_string());
        }
        Framework::Vue => {
            options.jsx.runtime = JsxRuntime::Automatic;
            options.jsx.development = !is_production;
            options.jsx.import_source = Some("vue".to_string());
        }
        Framework::Next | Framework::TanStack | Framework::PledgeStack => {
            options.jsx.runtime = JsxRuntime::Automatic;
            options.jsx.development = !is_production;
            options.jsx.import_source = Some("react".to_string());
        }
        _ => {
            options.jsx.runtime = JsxRuntime::Automatic;
            options.jsx.development = !is_production;
            options.jsx.import_source = Some("react".to_string());
        }
    }

    let semantic_result = oxc::semantic::SemanticBuilder::new()
        .with_check_syntax_error(false)
        .build(&program);

    let scoping = semantic_result.semantic.into_scoping();
    let transformer = Transformer::new(&allocator, path, &options);
    let transform_result = transformer.build_with_scoping(scoping, &mut program);

    if !transform_result.diagnostics.is_empty() {
        for err in &transform_result.diagnostics {
            warn!("Transform error in {}: {:?}", file_path, err);
        }
    }

    if is_production {
        let minifier = oxc::minifier::Minifier::new(oxc::minifier::MinifierOptions {
            mangle: Some(Default::default()),
            ..Default::default()
        });
        minifier.minify(&allocator, &mut program);
    }

    let codegen_result = Codegen::new()
        .with_options(CodegenOptions {
            minify: is_production,
            source_map_path: config
                .source_maps
                .then(|| Path::new(file_path).to_path_buf()),
            ..CodegenOptions::default()
        })
        .build(&program);

    let dynamic_imports = detect_dynamic_imports(source);

    let is_worker = file_path.contains(".worker.")
        || file_path.contains("?worker")
        || file_path.contains("?sharedworker")
        || source.contains("new Worker(new URL(")
        || source.contains("new SharedWorker(new URL(");

    let mut code = env::replace_env_vars(&codegen_result.code, config);

    if config.build.env_inline {
        code = env::inline_process_env(&code, is_production);
    }

    code = env::expand_import_meta_glob(&code, file_path, config);

    if !config.define.is_empty() {
        code = env::apply_define(&code, &config.define);
    }

    if !is_production
        && matches!(
            config.framework,
            Framework::React | Framework::Next | Framework::TanStack | Framework::PledgeStack
        )
        && is_react_component(source, file_path)
    {
        code = inject_fast_refresh(&code, file_path);
    }

    if source.contains("new Worker(new URL(")
        || source.contains("new SharedWorker(new URL(")
        || source.contains("?worker")
        || source.contains("?sharedworker")
    {
        code = utils::transform_worker_imports(&code, file_path);
    }

    let extracted_css =
        if let Some(extraction) = crate::css_in_js::extract_css_in_js(source, file_path) {
            code = extraction.code;
            if extraction.css.is_empty() {
                None
            } else {
                Some(extraction.css)
            }
        } else {
            None
        };

    let source_map = if config.source_maps {
        let mode = &config.build.source_map_mode;
        let oxc_map = codegen_result.map.as_ref().map(|m| m.to_json_string());

        match mode.as_str() {
            "hidden" | "nosources" => {
                if let Some(ref map_json) = oxc_map {
                    Some(utils::apply_source_map_mode(map_json, mode))
                } else {
                    Some(utils::generate_source_map_mode(
                        file_path,
                        source,
                        &codegen_result.code,
                        mode,
                    ))
                }
            }
            "inline" => {
                let map = oxc_map.unwrap_or_else(|| {
                    utils::generate_source_map(file_path, source, &codegen_result.code)
                });
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(map.as_bytes());
                code.push_str(&format!(
                    "\n//# sourceMappingURL=data:application/json;base64,{}",
                    b64
                ));
                None
            }
            _ => {
                let map = if let Some(map_json) = oxc_map {
                    map_json
                } else {
                    utils::generate_source_map(file_path, source, &codegen_result.code)
                };
                let file_name = Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let map_name = file_name
                    .replace(".tsx", ".js")
                    .replace(".ts", ".js")
                    .replace(".jsx", ".js")
                    + ".map";
                code.push_str(&format!("\n//# sourceMappingURL={}", map_name));
                Some(map)
            }
        }
    } else {
        None
    };

    Ok(TransformOutput {
        code,
        source_map,
        css_modules: None,
        is_css: false,
        extracted_css,
        is_worker,
        dynamic_imports,
        content_hash: None,
    })
}

/// Check if a source file is a React component (has JSX and starts with capital or function)
fn is_react_component(source: &str, _file_path: &str) -> bool {
    if !source.contains("<") || !source.contains("/>") && !source.contains("</") {
        return false;
    }

    source.contains("function App")
        || source.contains("function Component")
        || source.contains("export default function")
        || (source.contains("=>") && source.contains("return") && source.contains("<"))
}

/// Inject React Fast Refresh runtime code for HMR state preservation
fn inject_fast_refresh(code: &str, file_path: &str) -> String {
    let component_name = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Component");

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
    for line in code.lines() {
        let trimmed = line.trim();
        if let Some(after_fn) = trimmed.strip_prefix("function ")
            && let Some(paren) = after_fn.find('(')
        {
            let name = after_fn[..paren].trim();
            if !name.is_empty()
                && name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
            {
                return Some(name.to_string());
            }
        }
        if trimmed.starts_with("const ") || trimmed.starts_with("export const ") {
            let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                let name = parts[1].trim();
                if name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// Detect dynamic import() specifiers from a pre-parsed Oxc AST Program.
/// This is used by the AstPool and task_transform modules to extract
/// dynamic imports from cached ASTs without re-parsing.
pub fn detect_dynamic_imports_from_program(program: &oxc::ast::ast::Program) -> Vec<String> {
    use oxc::ast_visit::Visit;

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

    let mut collector = ImportCollector {
        imports: Vec::new(),
    };
    collector.visit_program(program);
    collector.imports
}

/// Detect dynamic import() specifiers for code splitting.
/// Uses Oxc AST parsing to find ImportExpression nodes accurately.
/// Falls back to string-based detection if parsing fails.
fn detect_dynamic_imports(source: &str) -> Vec<String> {
    if let Some(imports) = detect_dynamic_imports_ast(source) {
        return imports;
    }

    let mut imports = Vec::new();
    let mut search_pos = 0;

    while let Some(pos) = source[search_pos..].find("import(") {
        let abs_pos = search_pos + pos;
        let after = &source[abs_pos + 7..];

        if let Some(quote_pos) = after.find(['"', '\'']) {
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
    use oxc::ast_visit::Visit;

    let allocator = Allocator::default();
    let ParserReturn {
        program, panicked, ..
    } = Parser::new(&allocator, source, SourceType::mjs()).parse();

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

    let mut collector = ImportCollector {
        imports: Vec::new(),
    };
    collector.visit_program(&program);
    Some(collector.imports)
}
