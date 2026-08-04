// Data format transforms: MDX, GraphQL, YAML, CSV, TSV, TOML

use super::TransformOutput;
use anyhow::Result;

pub(super) fn transform_mdx(source: &str, file_path: &str) -> Result<TransformOutput> {
    let result = crate::asset_pipeline::compile_mdx(source, file_path);
    Ok(TransformOutput {
        code: result.code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
        content_hash: None,
    })
}

pub(super) fn transform_graphql(source: &str) -> Result<TransformOutput> {
    let code = crate::asset_pipeline::graphql_to_module(source);
    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
        content_hash: None,
    })
}

pub(super) fn transform_yaml(source: &str) -> Result<TransformOutput> {
    let code = crate::asset_pipeline::transform_yaml(source);
    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
        content_hash: None,
    })
}

pub(super) fn transform_csv(source: &str) -> Result<TransformOutput> {
    let code = crate::asset_pipeline::transform_csv(source);
    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
        content_hash: None,
    })
}

pub(super) fn transform_tsv(source: &str) -> Result<TransformOutput> {
    let code = crate::asset_pipeline::transform_tsv(source);
    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
        content_hash: None,
    })
}

/// #61: Transform TOML into an ES module with named exports + default export
pub(super) fn transform_toml(source: &str) -> Result<TransformOutput> {
    let value: toml::Value =
        toml::from_str(source).map_err(|e| anyhow::anyhow!("TOML parse error: {}", e))?;

    let json_value: serde_json::Value = serde_json::to_value(&value)
        .map_err(|e| anyhow::anyhow!("TOML to JSON conversion error: {}", e))?;

    let mut code = String::new();

    if let serde_json::Value::Object(map) = &json_value {
        for (key, val) in map {
            if key
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
                && !key.chars().next().map(|c| c.is_numeric()).unwrap_or(true)
            {
                let val_str = serde_json::to_string(val).unwrap_or_else(|_| "null".to_string());
                code.push_str(&format!("export const {} = {};\n", key, val_str));
            }
        }
    }

    let default_export =
        serde_json::to_string(&json_value).unwrap_or_else(|_| source.trim().to_string());
    code.push_str(&format!("export default {};", default_export));

    Ok(TransformOutput {
        code,
        source_map: None,
        css_modules: None,
        is_css: false,
        extracted_css: None,
        is_worker: false,
        dynamic_imports: Vec::new(),
        content_hash: None,
    })
}
