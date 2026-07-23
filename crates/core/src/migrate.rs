// Config migration — converts Vite, webpack, CRA, and Next.js configs to pledge.config.ts

use std::path::Path;
use anyhow::Result;

/// Migration result containing the new config content and a summary of what was migrated.
pub struct MigrationResult {
    pub config_content: String,
    pub config_path: String,
    pub warnings: Vec<String>,
    pub migrated_fields: Vec<String>,
}

/// Detect and migrate config from existing build tools.
pub fn migrate_config(root: &Path) -> Result<MigrationResult> {
    // Try Vite config
    for ext in &["ts", "js", "mjs"] {
        let path = root.join(format!("vite.config.{}", ext));
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            return migrate_vite_config(&content);
        }
    }

    // Try webpack config
    for ext in &["ts", "js", "cjs", "mjs"] {
        let path = root.join(format!("webpack.config.{}", ext));
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            return migrate_webpack_config(&content);
        }
    }

    // Try CRA (config-overrides.js or react-scripts in package.json)
    let cra_path = root.join("config-overrides.js");
    if cra_path.exists() {
        let content = std::fs::read_to_string(&cra_path)?;
        return migrate_cra_config(&content);
    }

    // Try Next.js config
    for ext in &["ts", "js", "mjs"] {
        let path = root.join(format!("next.config.{}", ext));
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            return migrate_next_config(&content);
        }
    }

    anyhow::bail!("No recognized config file found. Looking for: vite.config.{{ts,js,mjs}}, webpack.config.{{ts,js,cjs,mjs}}, config-overrides.js, next.config.{{ts,js,mjs}}")
}

fn migrate_vite_config(content: &str) -> Result<MigrationResult> {
    let mut warnings = Vec::new();
    let mut migrated_fields = Vec::new();
    let mut entry = "src/index.tsx".to_string();
    let mut port = 3000u16;
    let mut host = "localhost".to_string();
    let mut proxy_entries = Vec::new();
    let mut alias_entries = Vec::new();
    let mut plugins = Vec::new();

    // Extract defineConfig({...}) or export default {...}
    let config_obj = extract_object_literal(content, "defineConfig")
        .or_else(|| extract_export_default_object(content));

    if let Some(ref obj) = config_obj {
        // Extract root
        if let Some(root_val) = extract_string_field(obj, "root") {
            warnings.push(format!("Vite `root` option '{}' — Pledgepack uses `root` from CLI flag instead", root_val));
        }

        // Extract base
        if let Some(base_val) = extract_string_field(obj, "base") {
            warnings.push(format!("Vite `base` option '{}' — configure in your HTML or server instead", base_val));
        }

        // Extract server config
        if let Some(server_obj) = extract_nested_object(obj, "server") {
            if let Some(p) = extract_number_field(&server_obj, "port") {
                port = p as u16;
                migrated_fields.push("server.port".to_string());
            }
            if let Some(h) = extract_string_field(&server_obj, "host") {
                host = h;
                migrated_fields.push("server.host".to_string());
            }
            // Extract proxy
            if let Some(proxy_obj) = extract_nested_object(&server_obj, "proxy") {
                for (key, value) in extract_key_value_pairs(&proxy_obj) {
                    if let Some(target) = extract_string_field(&value, "target") {
                        proxy_entries.push(format!(
                            "    {{ path: '/{}', target: '{}', rewrite: {} }}",
                            key, target,
                            extract_bool_field(&value, "rewrite").unwrap_or(true)
                        ));
                    }
                }
                migrated_fields.push("server.proxy".to_string());
            }
        }

        // Extract resolve.alias
        if let Some(resolve_obj) = extract_nested_object(obj, "resolve") {
            if let Some(alias_obj) = extract_nested_object(&resolve_obj, "alias") {
                for (key, value) in extract_key_value_pairs(&alias_obj) {
                    if !value.is_empty() {
                        alias_entries.push(format!("    {{ from: '{}', to: '{}' }}", key, value));
                        migrated_fields.push("resolve.alias".to_string());
                    }
                }
            }
        }

        // Extract build.outDir
        if let Some(build_obj) = extract_nested_object(obj, "build") {
            if let Some(out_dir) = extract_string_field(&build_obj, "outDir") {
                warnings.push(format!("Vite `build.outDir` '{}' — Pledgepack uses `outDir` in config", out_dir));
                migrated_fields.push("build.outDir".to_string());
            }
        }

        // Extract plugins
        if let Some(plugins_str) = extract_array_field(obj, "plugins") {
            for plugin_name in parse_plugin_names(&plugins_str) {
                plugins.push(plugin_name);
            }
            migrated_fields.push("plugins".to_string());
        }
    }

    // Build pledge.config.ts
    let mut config = String::from("import { defineConfig } from 'pledgepack';\n\nexport default defineConfig({\n");
    config.push_str(&format!("  entry: ['{}'],\n", entry));
    config.push_str("  framework: 'auto',\n");
    config.push_str(&format!("  devServer: {{\n    port: {},\n    host: '{}',\n    hmr: true,\n  }},\n", port, host));

    if !alias_entries.is_empty() {
        config.push_str("  resolveAlias: [\n");
        config.push_str(&alias_entries.join(",\n"));
        config.push_str("\n  ],\n");
    }

    if !proxy_entries.is_empty() {
        config.push_str("  proxy: [\n");
        config.push_str(&proxy_entries.join(",\n"));
        config.push_str("\n  ],\n");
    }

    if !plugins.is_empty() {
        config.push_str("  plugins: [\n");
        for p in &plugins {
            config.push_str(&format!("    '{}',\n", p));
        }
        config.push_str("  ],\n");
    }

    config.push_str("});\n");

    if migrated_fields.is_empty() {
        warnings.push("No configurable fields were found in the Vite config. A default Pledgepack config was generated.".to_string());
    }

    Ok(MigrationResult {
        config_content: config,
        config_path: "pledge.config.ts".to_string(),
        warnings,
        migrated_fields,
    })
}

fn migrate_webpack_config(content: &str) -> Result<MigrationResult> {
    let mut warnings = Vec::new();
    let mut migrated_fields = Vec::new();

    // Extract entry
    let entry = extract_string_field(content, "entry")
        .unwrap_or_else(|| "./src/index.tsx".to_string());
    migrated_fields.push("entry".to_string());

    // Extract output path
    let out_dir = if let Some(output_obj) = extract_nested_object(content, "output") {
        extract_string_field(&output_obj, "path")
            .unwrap_or_else(|| ".pledge".to_string())
    } else {
        ".pledge".to_string()
    };
    if out_dir != ".pledge" {
        warnings.push(format!("Webpack `output.path` '{}' mapped to Pledgepack `outDir`", out_dir));
    }

    // Extract devServer
    let (port, host) = if let Some(dev_server) = extract_nested_object(content, "devServer") {
        let port = extract_number_field(&dev_server, "port").unwrap_or(3000.0) as u16;
        let host = extract_string_field(&dev_server, "host").unwrap_or_else(|| "localhost".to_string());
        migrated_fields.push("devServer.port".to_string());
        migrated_fields.push("devServer.host".to_string());
        (port, host)
    } else {
        (3000, "localhost".to_string())
    };

    // Extract resolve.alias
    let mut alias_entries = Vec::new();
    if let Some(resolve_obj) = extract_nested_object(content, "resolve") {
        if let Some(alias_obj) = extract_nested_object(&resolve_obj, "alias") {
            for (key, value) in extract_key_value_pairs(&alias_obj) {
                if !value.is_empty() {
                    alias_entries.push(format!("    {{ from: '{}', to: '{}' }}", key, value));
                }
            }
            migrated_fields.push("resolve.alias".to_string());
        }
    }

    // Extract module.rules for CSS preprocessors
    if content.contains("sass-loader") || content.contains("scss") {
        warnings.push("Sass/SCSS detected in webpack config — Pledgepack handles .scss/.sass natively".to_string());
    }
    if content.contains("postcss-loader") {
        warnings.push("PostCSS detected in webpack config — Pledgepack has built-in PostCSS support".to_string());
    }

    // Extract plugins
    if content.contains("HtmlWebpackPlugin") {
        migrated_fields.push("html plugin (HtmlWebpackPlugin → built-in HTML processing)".to_string());
    }
    if content.contains("MiniCssExtractPlugin") {
        migrated_fields.push("css extract (MiniCssExtractPlugin → built-in CSS extraction)".to_string());
    }
    if content.contains("DefinePlugin") {
        warnings.push("webpack DefinePlugin detected — use Pledgepack's env injection (import.meta.env.*) instead".to_string());
    }

    let mut config = String::from("import { defineConfig } from 'pledgepack';\n\nexport default defineConfig({\n");
    config.push_str(&format!("  entry: ['{}'],\n", entry.trim_start_matches("./")));
    config.push_str("  framework: 'auto',\n");
    config.push_str(&format!("  outDir: '{}',\n", out_dir));
    config.push_str(&format!("  devServer: {{\n    port: {},\n    host: '{}',\n    hmr: true,\n  }},\n", port, host));

    if !alias_entries.is_empty() {
        config.push_str("  resolveAlias: [\n");
        config.push_str(&alias_entries.join(",\n"));
        config.push_str("\n  ],\n");
    }

    config.push_str("});\n");

    Ok(MigrationResult {
        config_content: config,
        config_path: "pledge.config.ts".to_string(),
        warnings,
        migrated_fields,
    })
}

fn migrate_cra_config(content: &str) -> Result<MigrationResult> {
    let mut warnings = Vec::new();
    let mut migrated_fields = Vec::new();

    migrated_fields.push("entry (CRA uses src/index.tsx)".to_string());

    if content.contains("rewireCss") || content.contains("sass") {
        warnings.push("CSS overrides detected — Pledgepack handles Sass/SCSS natively".to_string());
    }

    if content.contains("rewireWebpack") || content.contains("webpack") {
        warnings.push("Webpack overrides detected — review if equivalent Pledgepack config is needed".to_string());
    }

    let config = r#"import { defineConfig } from 'pledgepack';

export default defineConfig({
  entry: ['src/index.tsx'],
  framework: 'react',
  devServer: {
    port: 3000,
    hmr: true,
  },
});
"#.to_string();

    Ok(MigrationResult {
        config_content: config,
        config_path: "pledge.config.ts".to_string(),
        warnings,
        migrated_fields,
    })
}

fn migrate_next_config(content: &str) -> Result<MigrationResult> {
    let mut warnings = Vec::new();
    let mut migrated_fields = Vec::new();

    migrated_fields.push("framework (Next.js → React adapter)".to_string());

    // Check for common Next.js config options
    if content.contains("images") {
        warnings.push("Next.js `images` config — use Pledgepack's `image` config field instead".to_string());
    }
    if content.contains("rewrites") {
        warnings.push("Next.js `rewrites` — configure as proxy rules in Pledgepack's devServer".to_string());
    }
    if content.contains("redirects") {
        warnings.push("Next.js `redirects` — handle at your server/edge level".to_string());
    }
    if content.contains("experimental") && content.contains("appDir") {
        warnings.push("Next.js App Router detected — use Pledgepack's Next.js adapter".to_string());
    }

    let config = r#"import { defineConfig } from 'pledgepack';

export default defineConfig({
  entry: ['src/app/root.tsx'],
  framework: 'react',
  devServer: {
    port: 3000,
    hmr: true,
  },
  // Next.js adapter handles SSR, API routes, and App/Pages router
  plugins: ['@pledge/adapter-next'],
});
"#.to_string();

    Ok(MigrationResult {
        config_content: config,
        config_path: "pledge.config.ts".to_string(),
        warnings,
        migrated_fields,
    })
}

// === Helper functions for parsing JS/TS config objects ===

fn extract_object_literal(content: &str, fn_name: &str) -> Option<String> {
    let pattern = format!("{}(", fn_name);
    let start = content.find(&pattern)?;
    let after_fn = start + pattern.len();
    // Find the opening brace
    let brace_start = content[after_fn..].find('{')?;
    let abs_brace_start = after_fn + brace_start;

    // Find matching closing brace
    let mut depth = 0;
    let mut end = abs_brace_start;
    for (i, c) in content[abs_brace_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = abs_brace_start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    Some(content[abs_brace_start..=end].to_string())
}

fn extract_export_default_object(content: &str) -> Option<String> {
    let pattern = "export default";
    let start = content.find(pattern)?;
    let after = start + pattern.len();
    let brace_start = content[after..].find('{')?;
    let abs_brace_start = after + brace_start;

    let mut depth = 0;
    let mut end = abs_brace_start;
    for (i, c) in content[abs_brace_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = abs_brace_start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    Some(content[abs_brace_start..=end].to_string())
}

fn extract_string_field(obj: &str, field: &str) -> Option<String> {
    // Look for field: "value" or field: 'value'
    let patterns = [
        format!("{}:", field),
        format!("{} :", field),
        format!("\"{}\":", field),
        format!("\"{}\" :", field),
    ];

    for pattern in &patterns {
        if let Some(pos) = obj.find(pattern) {
            let after = pos + pattern.len();
            let rest = obj[after..].trim_start();
            if rest.starts_with('"') || rest.starts_with('\'') {
                let quote = rest.chars().next().unwrap();
                let str_start = 1;
                if let Some(end) = rest[str_start..].find(quote) {
                    return Some(rest[str_start..str_start + end].to_string());
                }
            }
        }
    }
    None
}

fn extract_number_field(obj: &str, field: &str) -> Option<f64> {
    let patterns = [format!("{}:", field), format!("\"{}\":", field)];
    for pattern in &patterns {
        if let Some(pos) = obj.find(pattern) {
            let after = pos + pattern.len();
            let rest = obj[after..].trim_start();
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(n) = num_str.parse::<f64>() {
                return Some(n);
            }
        }
    }
    None
}

fn extract_bool_field(obj: &str, field: &str) -> Option<bool> {
    let patterns = [format!("{}:", field), format!("\"{}\":", field)];
    for pattern in &patterns {
        if let Some(pos) = obj.find(pattern) {
            let after = pos + pattern.len();
            let rest = obj[after..].trim_start();
            if rest.starts_with("true") {
                return Some(true);
            }
            if rest.starts_with("false") {
                return Some(false);
            }
        }
    }
    None
}

fn extract_nested_object(obj: &str, field: &str) -> Option<String> {
    let patterns = [format!("{}:", field), format!("\"{}\":", field)];
    for pattern in &patterns {
        if let Some(pos) = obj.find(pattern) {
            let after = pos + pattern.len();
            let rest = obj[after..].trim_start();
            if rest.starts_with('{') {
                let brace_start = after + (rest.len() - rest.trim_start().len());
                let mut depth = 0;
                for (i, c) in obj[brace_start..].char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                return Some(obj[brace_start..=brace_start + i].to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    None
}

fn extract_array_field(obj: &str, field: &str) -> Option<String> {
    let patterns = [format!("{}:", field), format!("\"{}\":", field)];
    for pattern in &patterns {
        if let Some(pos) = obj.find(pattern) {
            let after = pos + pattern.len();
            let rest = obj[after..].trim_start();
            if rest.starts_with('[') {
                let bracket_start = after + (rest.len() - rest.trim_start().len());
                let mut depth = 0;
                for (i, c) in obj[bracket_start..].char_indices() {
                    match c {
                        '[' => depth += 1,
                        ']' => {
                            depth -= 1;
                            if depth == 0 {
                                return Some(obj[bracket_start..=bracket_start + i].to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    None
}

fn extract_key_value_pairs(obj: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    // Simple extraction: look for key: value or "key": value patterns
    let mut chars = obj.chars().peekable();
    let mut _depth = 0;

    // This is a simplified parser — for production, use a proper JS parser
    // For now, we extract top-level key: "value" pairs
    let mut i = 0;
    let bytes = obj.as_bytes();

    while i < bytes.len() {
        // Skip whitespace
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        // Read key (quoted or unquoted)
        let key_start = i;
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // skip closing quote
            }
        } else {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-' || bytes[i] == b'@' || bytes[i] == b'/') {
                i += 1;
            }
        }
        let key = &obj[key_start..i];
        let key_clean = key.trim_matches(|c| c == '"' || c == '\'');

        // Skip whitespace
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        // Expect colon
        if i >= bytes.len() || bytes[i] != b':' {
            continue;
        }
        i += 1;

        // Skip whitespace
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        if i >= bytes.len() {
            break;
        }

        // Read value
        let value_start = i;
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            let value = &obj[value_start..i];
            let value_clean = value.trim_matches(|c| c == '"' || c == '\'');
            pairs.push((key_clean.to_string(), value_clean.to_string()));
        } else if bytes[i] == b'{' {
            let mut depth = 0;
            while i < bytes.len() {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            let value = &obj[value_start..i];
            pairs.push((key_clean.to_string(), value.to_string()));
        } else {
            // Skip to next comma or closing brace
            while i < bytes.len() && bytes[i] != b',' && bytes[i] != b'}' {
                i += 1;
            }
            let value = obj[value_start..i].trim();
            pairs.push((key_clean.to_string(), value.to_string()));
        }

        // Skip comma
        if i < bytes.len() && bytes[i] == b',' {
            i += 1;
        }
    }

    let _ = chars;
    let _ = _depth;
    pairs
}

fn parse_plugin_names(plugins_str: &str) -> Vec<String> {
    let mut names = Vec::new();
    // Look for patterns like vue() or reactPlugin() or just string identifiers
    let mut i = 0;
    let bytes = plugins_str.as_bytes();

    while i < bytes.len() {
        // Find identifier-like sequences
        while i < bytes.len() && !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-') {
            i += 1;
        }
        let name = &plugins_str[start..i];
        if !name.is_empty() && name != "plugins" {
            names.push(name.to_string());
        }

        // Skip to next comma or end
        while i < bytes.len() && bytes[i] != b',' && bytes[i] != b']' {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b',' {
            i += 1;
        }
    }

    names
}
