// Config validation — checks for unknown fields, typos, and provides "Did you mean...?" suggestions.

/// All valid top-level config field names (camelCase as they appear in pledge.config.ts).
pub const VALID_FIELDS: &[&str] = &[
    "entry", "outDir", "root", "mode", "framework", "alias", "extensions",
    "cache", "devServer", "sourceMaps", "resolveAlias", "proxy", "profile",
    "outputFormat", "conditions", "envPrefix", "envDts", "htmlEntry",
    "compressGzip", "compressBrotli", "image", "edgeTarget", "plugins",
];

/// Valid devServer fields.
pub const VALID_DEV_SERVER_FIELDS: &[&str] = &[
    "port", "host", "hmr", "open", "https",
];

/// Valid cache fields.
pub const VALID_CACHE_FIELDS: &[&str] = &[
    "enabled", "dir",
];

/// Valid image fields.
pub const VALID_IMAGE_FIELDS: &[&str] = &[
    "enabled", "quality", "webp", "avif", "maxWidth", "maxHeight",
];

/// Valid framework values.
pub const VALID_FRAMEWORKS: &[&str] = &[
    "react", "vue", "svelte", "solid", "auto",
];

/// Valid output format values.
pub const VALID_OUTPUT_FORMATS: &[&str] = &[
    "esm", "cjs", "iife",
];

/// Valid edge target values.
pub const VALID_EDGE_TARGETS: &[&str] = &[
    "cloudflare", "vercel", "deno",
];

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Validate a raw JSON config object (parsed from pledge.config.ts or pledge.json).
/// Returns a list of errors and warnings.
pub fn validate_config_json(config: &serde_json::Value) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if let Some(obj) = config.as_object() {
        for (key, _value) in obj {
            // Check top-level fields
            if !VALID_FIELDS.contains(&key.as_str()) {
                let suggestion = find_closest_match(key, VALID_FIELDS);
                errors.push(ValidationError {
                    field: key.clone(),
                    message: format!("Unknown config field: '{}'", key),
                    suggestion: suggestion.map(|s| format!("Did you mean '{}'?", s)),
                });
            }

            // Validate nested devServer fields
            if key == "devServer" {
                if let Some(ds_obj) = _value.as_object() {
                    for ds_key in ds_obj.keys() {
                        if !VALID_DEV_SERVER_FIELDS.contains(&ds_key.as_str()) {
                            let suggestion = find_closest_match(ds_key, VALID_DEV_SERVER_FIELDS);
                            errors.push(ValidationError {
                                field: format!("devServer.{}", ds_key),
                                message: format!("Unknown devServer field: '{}'", ds_key),
                                suggestion: suggestion.map(|s| format!("Did you mean '{}'?", s)),
                            });
                        }
                    }
                }
            }

            // Validate nested cache fields
            if key == "cache" {
                if let Some(c_obj) = _value.as_object() {
                    for c_key in c_obj.keys() {
                        if !VALID_CACHE_FIELDS.contains(&c_key.as_str()) {
                            let suggestion = find_closest_match(c_key, VALID_CACHE_FIELDS);
                            errors.push(ValidationError {
                                field: format!("cache.{}", c_key),
                                message: format!("Unknown cache field: '{}'", c_key),
                                suggestion: suggestion.map(|s| format!("Did you mean '{}'?", s)),
                            });
                        }
                    }
                }
            }

            // Validate nested image fields
            if key == "image" {
                if let Some(i_obj) = _value.as_object() {
                    for i_key in i_obj.keys() {
                        if !VALID_IMAGE_FIELDS.contains(&i_key.as_str()) {
                            let suggestion = find_closest_match(i_key, VALID_IMAGE_FIELDS);
                            errors.push(ValidationError {
                                field: format!("image.{}", i_key),
                                message: format!("Unknown image field: '{}'", i_key),
                                suggestion: suggestion.map(|s| format!("Did you mean '{}'?", s)),
                            });
                        }
                    }
                }
            }

            // Validate framework value
            if key == "framework" {
                if let Some(fw) = _value.as_str() {
                    if !VALID_FRAMEWORKS.contains(&fw) {
                        let suggestion = find_closest_match(fw, VALID_FRAMEWORKS);
                        errors.push(ValidationError {
                            field: key.clone(),
                            message: format!("Invalid framework: '{}'", fw),
                            suggestion: suggestion.map(|s| format!("Did you mean '{}'? Valid: {}", s, VALID_FRAMEWORKS.join(", "))),
                        });
                    }
                }
            }

            // Validate outputFormat value
            if key == "outputFormat" {
                if let Some(of) = _value.as_str() {
                    if !VALID_OUTPUT_FORMATS.contains(&of) {
                        let suggestion = find_closest_match(of, VALID_OUTPUT_FORMATS);
                        errors.push(ValidationError {
                            field: key.clone(),
                            message: format!("Invalid outputFormat: '{}'", of),
                            suggestion: suggestion.map(|s| format!("Did you mean '{}'? Valid: {}", s, VALID_OUTPUT_FORMATS.join(", "))),
                        });
                    }
                }
            }

            // Validate edgeTarget value
            if key == "edgeTarget" {
                if let Some(et) = _value.as_str() {
                    if !VALID_EDGE_TARGETS.contains(&et) {
                        let suggestion = find_closest_match(et, VALID_EDGE_TARGETS);
                        errors.push(ValidationError {
                            field: key.clone(),
                            message: format!("Invalid edgeTarget: '{}'", et),
                            suggestion: suggestion.map(|s| format!("Did you mean '{}'? Valid: {}", s, VALID_EDGE_TARGETS.join(", "))),
                        });
                    }
                }
            }
        }
    }

    errors
}

/// Find the closest matching string using Levenshtein distance.
pub fn find_closest_match(input: &str, candidates: &[&str]) -> Option<String> {
    let input_lower = input.to_lowercase();
    let mut best: Option<(usize, &str)> = None;

    for candidate in candidates {
        let dist = levenshtein(&input_lower, &candidate.to_lowercase());
        if best.is_none() || dist < best.unwrap().0 {
            best = Some((dist, candidate));
        }
    }

    // Only suggest if distance is reasonable (<= half the input length)
    if let Some((dist, candidate)) = best {
        let max_dist = (input.len() / 2).max(2);
        if dist <= max_dist {
            return Some(candidate.to_string());
        }
    }

    None
}

/// Levenshtein distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 { return b_len; }
    if b_len == 0 { return a_len; }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr: Vec<usize> = vec![0; b_len + 1];

    for i in 1..=a_len {
        curr[0] = i;
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}

/// Format validation errors for CLI output.
pub fn format_errors(errors: &[ValidationError]) -> String {
    let mut output = String::new();
    for err in errors {
        output.push_str(&format!(
            "  \x1b[33m⚠\x1b[0m {}\n     {}",
            err.field, err.message
        ));
        if let Some(ref suggestion) = err.suggestion {
            output.push_str(&format!("\n     \x1b[36m{}\x1b[0m", suggestion));
        }
        output.push('\n');
    }
    output
}
