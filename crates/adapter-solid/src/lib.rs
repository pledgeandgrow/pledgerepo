// Solid.js adapter: JSX transform for Solid.js
//
// Handles:
//   - JSX → Solid's createElement / template literals
//   - TypeScript type stripping
//   - Solid-specific transform options (no React runtime)

use anyhow::Result;
use pledgepack_core::module::ModuleKind;
use oxc::allocator::Allocator;
use oxc::parser::{Parser, ParserReturn};
use oxc::span::SourceType;
use oxc::transformer::{Transformer, TransformOptions, JsxRuntime};
use oxc::codegen::{Codegen, CodegenOptions};
use std::path::Path;

pub struct SolidAdapter;

impl SolidAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Transform JSX/TSX for Solid.js using Oxc
    /// Solid uses automatic JSX runtime with its own jsx-runtime
    pub fn transform(
        &self,
        source: &str,
        kind: ModuleKind,
        file_path: &str,
        is_production: bool,
    ) -> Result<String> {
        let allocator = Allocator::default();
        let path = Path::new(file_path);

        let source_type = SourceType::from_path(path).unwrap_or_else(|_| {
            match kind {
                ModuleKind::Tsx | ModuleKind::Psx => SourceType::tsx(),
                ModuleKind::TypeScript => SourceType::ts(),
                ModuleKind::Jsx => SourceType::jsx(),
                _ => SourceType::mjs(),
            }
        });

        let ParserReturn { mut program, errors: parser_errors, panicked, .. } =
            Parser::new(&allocator, source, source_type).parse();

        if panicked {
            anyhow::bail!("Failed to parse {}: {}", file_path,
                parser_errors.first().map(|e| e.to_string()).unwrap_or("unknown".into()));
        }

        // Solid uses automatic JSX runtime pointing to solid-js/jsx-runtime
        let mut options = TransformOptions::default();
        options.typescript.only_remove_type_imports = false;
        options.jsx.runtime = JsxRuntime::Automatic;
        options.jsx.development = !is_production;
        // Solid's jsx import source is "solid-js/jsx-runtime"
        options.jsx.import_source = Some("solid-js".to_string());

        let semantic_result = oxc::semantic::SemanticBuilder::new()
            .with_check_syntax_error(false)
            .build(&program);

        let transformer = Transformer::new(&allocator, path, &options);
        let (symbols, scopes) = semantic_result.semantic.into_symbol_table_and_scope_tree();
        let transform_result = transformer.build_with_symbols_and_scopes(symbols, scopes, &mut program);

        if !transform_result.errors.is_empty() {
            for err in &transform_result.errors {
                tracing::warn!("Solid transform error in {}: {:?}", file_path, err);
            }
        }

        let codegen_result = Codegen::new()
            .with_options(CodegenOptions {
                minify: is_production,
                ..CodegenOptions::default()
            })
            .build(&program);

        let mut code = codegen_result.code;

        // Inject Solid HMR boundary in dev mode with reactive scope preservation
        if !is_production {
            code.push_str(r#"
// Solid HMR — reactive scope preservation
if (import.meta.hot && typeof window !== 'undefined') {
  import.meta.hot.accept((newModule) => {
    if (newModule) {
      // Solid components are reactive by default — re-executing the module
      // re-creates reactive scopes. The Solid runtime handles cleanup
      // automatically via createRoot/createEffect disposal.
      // Notify all registered Solid HMR boundaries to re-execute
      const __solid_hmr_boundaries = window.__pledge_solid_hmr;
      if (__solid_hmr_boundaries) {
        __solid_hmr_boundaries.forEach((boundary) => {
          if (boundary && typeof boundary === 'function') {
            boundary(newModule);
          }
        });
      }
    }
  });
}
"#);
        }

        Ok(code)
    }
}

impl Default for SolidAdapter {
    fn default() -> Self {
        Self::new()
    }
}
