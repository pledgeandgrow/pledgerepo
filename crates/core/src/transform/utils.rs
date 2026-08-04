// Source maps and Web Worker import transforms

use std::path::Path;

/// Generate a source map for a transformed file.
/// Uses a simple V3 source map format with the original source content.
/// In "nosources" mode, sourcesContent is omitted for security.
pub(super) fn generate_source_map(
    file_path: &str,
    original_source: &str,
    _generated_code: &str,
) -> String {
    let file_name = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

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

/// Apply source map mode to an Oxc-generated source map JSON string.
/// - "nosources": removes sourcesContent for security
/// - "hidden": returns as-is (no sourceMappingURL comment is added by caller)
pub(super) fn apply_source_map_mode(map_json: &str, mode: &str) -> String {
    if mode == "nosources"
        && let Ok(mut map) = serde_json::from_str::<serde_json::Value>(map_json)
    {
        if let Some(obj) = map.as_object_mut() {
            obj.remove("sourcesContent");
        }
        return map.to_string();
    }
    map_json.to_string()
}

/// Generate a source map with configurable nosources mode
pub(super) fn generate_source_map_mode(
    file_path: &str,
    original_source: &str,
    _generated_code: &str,
    mode: &str,
) -> String {
    let file_name = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let mut map = serde_json::json!({
        "version": 3,
        "file": file_name.replace(".tsx", ".js").replace(".ts", ".js").replace(".jsx", ".js"),
        "sourceRoot": "",
        "sources": [file_name],
        "mappings": "",
        "names": []
    });

    if mode != "nosources" {
        map["sourcesContent"] = serde_json::json!([original_source]);
    }

    map.to_string()
}

/// Transform Web Worker patterns
/// new Worker(new URL('./worker.ts', import.meta.url))
/// → new Worker('/src/worker.js')
/// Also handles SharedWorker and { type: 'module' } options
/// Also handles ?worker and ?sharedworker import suffixes:
///   import MyWorker from './worker.ts?worker'
///   → const MyWorker = () => new Worker('/src/worker.js')
pub(super) fn transform_worker_imports(code: &str, file_path: &str) -> String {
    let mut result = code.to_string();

    let worker_patterns = ["new Worker(new URL(", "new SharedWorker(new URL("];

    for worker_pattern in &worker_patterns {
        while let Some(start) = result.find(worker_pattern) {
            let after = &result[start + worker_pattern.len()..];
            if let Some(_end_quote) = after.find(['"', '\'']) {
                let quote_char = after.as_bytes()[0] as char;
                let spec_start = 1;
                let spec_rest = &after[spec_start..];
                if let Some(end) = spec_rest.find(quote_char) {
                    let specifier = &spec_rest[..end];
                    let clean_spec = specifier
                        .trim_end_matches("?worker")
                        .trim_end_matches("?sharedworker");
                    let url = format!("/{}.js", clean_spec.replace("./", "").replace("../", ""));
                    let full_end = start + worker_pattern.len() + end + 2;
                    if let Some(close) = result[full_end..].find("))") {
                        let abs_end = full_end + close + 2;
                        let worker_type = if worker_pattern.starts_with("new Shared") {
                            "new SharedWorker"
                        } else {
                            "new Worker"
                        };
                        result.replace_range(
                            start..abs_end,
                            &format!("{}(\"{}\")", worker_type, url),
                        );
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

    for (suffix, constructor) in &[
        ("?worker", "new Worker"),
        ("?sharedworker", "new SharedWorker"),
    ] {
        let import_pattern = "from \"".to_string();
        let mut search_pos = 0;
        while let Some(pos) = result[search_pos..].find(&import_pattern) {
            let abs_pos = search_pos + pos;
            let after = &result[abs_pos + import_pattern.len()..];
            if let Some(end) = after.find('"') {
                let specifier = &after[..end];
                if specifier.ends_with(suffix) {
                    let clean_spec = specifier.trim_end_matches(suffix);
                    let url = format!("/{}.js", clean_spec.replace("./", "").replace("../", ""));
                    if let Some(import_start) = result[..abs_pos].rfind("import ") {
                        let import_end = abs_pos + import_pattern.len() + end + 1;
                        let between = &result[import_start + 7..abs_pos];
                        let var_name = between.trim().trim_end_matches("from").trim();
                        if !var_name.is_empty() {
                            let replacement = format!(
                                "const {} = function() {{ return {}(\"{}\"); }}",
                                var_name, constructor, url
                            );
                            result.replace_range(import_start..import_end, &replacement);
                            search_pos = import_start + replacement.len();
                            continue;
                        }
                    }
                }
            }
            search_pos = abs_pos + 1;
        }
    }

    let _ = file_path;
    result
}
