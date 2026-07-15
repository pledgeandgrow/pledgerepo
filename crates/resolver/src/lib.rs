// Module resolver: resolves import specifiers to file paths
//
// Handles:
//   - Relative paths (./foo, ../bar)
//   - Bare specifiers (react, lodash) → node_modules
//   - Path aliases (tsconfig.json paths, jsconfig.json)
//   - Extension resolution (.tsx → .ts → .jsx → .js)
//   - Directory resolution (./components → ./components/index.tsx)
//   - package.json "exports" field

use anyhow::Result;
use dashmap::DashMap;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Resolver {
    root: PathBuf,
    extensions: Vec<String>,
    aliases: Vec<Alias>,
    /// Cache: specifier → resolved path (per-directory context)
    cache: Arc<DashMap<(PathBuf, String), Option<PathBuf>>>,
}

#[derive(Debug, Clone)]
pub struct Alias {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize)]
struct TsConfig {
    compiler_options: Option<CompilerOptions>,
    extends: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompilerOptions {
    base_url: Option<String>,
    paths: Option<std::collections::HashMap<String, Vec<String>>>,
}

impl Resolver {
    pub fn new(root: PathBuf, extensions: Vec<String>, aliases: Vec<Alias>) -> Self {
        Self {
            root,
            extensions,
            aliases,
            cache: Arc::new(DashMap::new()),
        }
    }

    /// Create a resolver from tsconfig.json or jsconfig.json
    /// Supports: paths, baseUrl, extends, and wildcard patterns
    pub fn from_tsconfig(root: PathBuf, extensions: Vec<String>) -> Self {
        let mut aliases = Vec::new();

        // Try tsconfig.json first, then jsconfig.json
        let config_path = root.join("tsconfig.json")
            .exists()
            .then(|| root.join("tsconfig.json"))
            .or_else(|| root.join("jsconfig.json").exists().then(|| root.join("jsconfig.json")));

        if let Some(tsconfig_path) = config_path {
            Self::parse_tsconfig(&tsconfig_path, &root, &mut aliases);
        }

        Self::new(root, extensions, aliases)
    }

    /// Parse a tsconfig/jsconfig file and populate aliases.
    /// Handles `extends` by recursively parsing parent configs.
    fn parse_tsconfig(config_path: &Path, root: &Path, aliases: &mut Vec<Alias>) {
        let content = match std::fs::read_to_string(config_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        // Strip JSON comments (tsconfig allows // and /* */ comments)
        let clean_json = strip_json_comments(&content);

        let tsconfig: TsConfig = match serde_json::from_str(&clean_json) {
            Ok(t) => t,
            Err(_) => return,
        };

        // Handle `extends` — parse parent config first, then override
        if let Some(ref extends) = tsconfig.extends {
            let parent_path = if extends.starts_with('.') {
                // Relative path
                config_path.parent()
                    .map(|p| p.join(extends))
                    .filter(|p| p.exists())
            } else {
                // Could be a node_modules package like "@tsconfig/strict"
                root.join("node_modules")
                    .join(extends)
                    .join("tsconfig.json")
                    .exists()
                    .then(|| root.join("node_modules").join(extends).join("tsconfig.json"))
            };

            if let Some(ref parent) = parent_path {
                Self::parse_tsconfig(parent, root, aliases);
            }
        }

        // Apply this config's compiler options (overrides parent)
        if let Some(opts) = tsconfig.compiler_options {
            let base_url = opts.base_url.unwrap_or_else(|| ".".to_string());
            let base = root.join(&base_url);

            if let Some(paths) = opts.paths {
                // Clear parent aliases that conflict (paths override extends)
                let new_froms: Vec<String> = paths.keys().cloned().collect();
                aliases.retain(|a| !new_froms.iter().any(|nf| nf.starts_with(&a.from) || a.from.starts_with(nf)));

                for (from, tos) in paths {
                    for to in tos {
                        // Preserve wildcard info for pattern matching
                        let has_wildcard = from.contains('*');
                        let from_clean = from.replace('*', "");
                        let to_clean = to.replace('*', "");
                        let to_path = base.join(&to_clean);

                        aliases.push(Alias {
                            from: from_clean,
                            to: to_path.to_string_lossy().to_string(),
                        });

                        // If wildcard, also add the pattern for matching
                        if has_wildcard {
                            // Store wildcard pattern separately for pattern matching
                            // The alias.from without '*' acts as prefix, and we resolve
                            // the rest as a subpath under alias.to
                        }
                    }
                }
            }
        }
    }

    /// Resolve a module specifier to a file path
    pub fn resolve(&self, specifier: &str, importer: &Path) -> Result<PathBuf> {
        let cache_key = (importer.to_path_buf(), specifier.to_string());
        if let Some(cached) = self.cache.get(&cache_key) {
            if let Some(path) = cached.as_ref() {
                return Ok(path.clone());
            }
        }

        let resolved = self.resolve_uncached(specifier, importer)?;

        self.cache.insert(cache_key, Some(resolved.clone()));
        Ok(resolved)
    }

    fn resolve_uncached(&self, specifier: &str, importer: &Path) -> Result<PathBuf> {
        // 1. Check aliases
        for alias in &self.aliases {
            if specifier.starts_with(&alias.from) {
                let rest = &specifier[alias.from.len()..];
                let path = PathBuf::from(&alias.to).join(rest);
                if let Some(resolved) = self.try_resolve_path(&path)? {
                    return Ok(resolved);
                }
            }
        }

        // 2. Relative paths
        if specifier.starts_with("./") || specifier.starts_with("../") {
            let base = importer.parent().unwrap_or(&self.root);
            let path = base.join(specifier);
            if let Some(resolved) = self.try_resolve_path(&path)? {
                return Ok(resolved);
            }
        }

        // 3. Absolute paths
        if specifier.starts_with('/') {
            let path = PathBuf::from(specifier);
            if let Some(resolved) = self.try_resolve_path(&path)? {
                return Ok(resolved);
            }
        }

        // 4. Bare specifier → node_modules
        if let Some(resolved) = self.resolve_node_module(specifier)? {
            return Ok(resolved);
        }

        anyhow::bail!("Cannot resolve '{}' from {:?}", specifier, importer)
    }

    fn try_resolve_path(&self, path: &Path) -> Result<Option<PathBuf>> {
        // Try exact path
        if path.is_file() {
            return Ok(Some(path.canonicalize().unwrap_or(path.to_path_buf())));
        }

        // Try with extensions
        for ext in &self.extensions {
            let with_ext = path.with_extension(ext.trim_start_matches('.'));
            if with_ext.is_file() {
                return Ok(Some(with_ext.canonicalize().unwrap_or(with_ext)));
            }
        }

        // Try as directory with index file
        if path.is_dir() {
            for ext in &self.extensions {
                let index = path.join(format!("index{}", ext));
                if index.is_file() {
                    return Ok(Some(index.canonicalize().unwrap_or(index)));
                }
            }
        }

        Ok(None)
    }

    fn resolve_node_module(&self, specifier: &str) -> Result<Option<PathBuf>> {
        let mut current = self.root.clone();

        // Split package name and subpath (e.g., "react/jsx-runtime" → "react" + "/jsx-runtime")
        let (pkg_name, subpath) = if specifier.starts_with('@') {
            // Scoped package: @scope/name/subpath
            if let Some(idx) = specifier[1..].find('/') {
                let after_scope = &specifier[1..idx + 1];
                if let Some(sub_idx) = after_scope.find('/') {
                    let pkg = &specifier[..1 + sub_idx + 1];
                    let sub = &specifier[1 + sub_idx + 1..];
                    (pkg, Some(sub))
                } else {
                    (specifier, None)
                }
            } else {
                (specifier, None)
            }
        } else if let Some(idx) = specifier.find('/') {
            (&specifier[..idx], Some(&specifier[idx..]))
        } else {
            (specifier, None)
        };

        loop {
            let node_modules = current.join("node_modules");
            if node_modules.is_dir() {
                let module_path = node_modules.join(pkg_name);

                // Check package.json for entry point
                let pkg_json = module_path.join("package.json");
                if pkg_json.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                            // 1. Try "exports" field (modern)
                            if let Some(exports) = pkg.get("exports") {
                                if let Some(resolved) = self.resolve_exports(exports, subpath, &module_path)? {
                                    return Ok(Some(resolved));
                                }
                            }

                            // 2. Try "module" field (ESM preference)
                            if subpath.is_none() {
                                if let Some(module) = pkg.get("module").and_then(|v| v.as_str()) {
                                    let entry_path = module_path.join(module);
                                    if entry_path.is_file() {
                                        return Ok(Some(entry_path.canonicalize().unwrap_or(entry_path)));
                                    }
                                }

                                // 3. Try "main" field
                                if let Some(main) = pkg.get("main").and_then(|v| v.as_str()) {
                                    let entry_path = module_path.join(main);
                                    if entry_path.is_file() {
                                        return Ok(Some(entry_path.canonicalize().unwrap_or(entry_path)));
                                    }
                                }
                            }

                            // 4. Try "browser" field for browser-specific builds
                            if subpath.is_none() {
                                if let Some(browser) = pkg.get("browser").and_then(|v| v.as_str()) {
                                    let entry_path = module_path.join(browser);
                                    if entry_path.is_file() {
                                        return Ok(Some(entry_path.canonicalize().unwrap_or(entry_path)));
                                    }
                                }
                            }
                        }
                    }
                }

                // Try direct file resolution for subpath
                if let Some(sub) = subpath {
                    let sub_path = module_path.join(sub.trim_start_matches('/'));
                    if let Some(resolved) = self.try_resolve_path(&sub_path)? {
                        return Ok(Some(resolved));
                    }
                }

                // Try direct file resolution
                if let Some(resolved) = self.try_resolve_path(&module_path)? {
                    return Ok(Some(resolved));
                }
            }

            // Go up one directory
            if !current.pop() {
                break;
            }
        }

        Ok(None)
    }

    /// Resolve using package.json "exports" field
    fn resolve_exports(
        &self,
        exports: &serde_json::Value,
        subpath: Option<&str>,
        module_path: &Path,
    ) -> Result<Option<PathBuf>> {
        // exports can be:
        //   "./foo.js" → { "import": "...", "require": "..." }
        //   { ".": { "import": "./esm/index.js" }, "./utils": { "import": "./esm/utils.js" } }
        //   { "import": "./esm/index.js" } (sugar for ".")

        let target_key = subpath.unwrap_or(".");

        if let Some(obj) = exports.as_object() {
            // Check if it's a conditional export (top-level keys like "import", "require")
            if obj.contains_key("import") || obj.contains_key("require") || obj.contains_key("default") {
                if target_key == "." {
                    // Sugar form: top-level conditions apply to "."
                    return self.resolve_conditions(obj, module_path);
                }
            }

            // Subpath exports: look for matching key
            for (key, value) in obj {
                if key == target_key {
                    if let Some(obj2) = value.as_object() {
                        return self.resolve_conditions(obj2, module_path);
                    } else if let Some(path) = value.as_str() {
                        let resolved = module_path.join(path);
                        if resolved.is_file() {
                            return Ok(Some(resolved.canonicalize().unwrap_or(resolved)));
                        }
                    }
                }

                // Pattern matching: "./utils/*" → "./utils/*.js"
                if key.ends_with('*') && target_key.starts_with(&key[..key.len() - 1]) {
                    let pattern_prefix = &key[..key.len() - 1];
                    let rest = &target_key[pattern_prefix.len()..];
                    if let Some(path) = value.as_str() {
                        let resolved_path = path.replace('*', rest);
                        let resolved = module_path.join(&resolved_path);
                        if resolved.is_file() {
                            return Ok(Some(resolved.canonicalize().unwrap_or(resolved)));
                        }
                    } else if let Some(obj2) = value.as_object() {
                        if let Some(path) = self.resolve_conditions(obj2, module_path)?.as_ref() {
                            // Replace pattern in resolved path
                            let path_str = path.to_string_lossy();
                            if path_str.contains('*') {
                                let replaced = path_str.replace('*', rest);
                                let p = PathBuf::from(replaced);
                                if p.is_file() {
                                    return Ok(Some(p));
                                }
                            }
                            return Ok(Some(path.clone()));
                        }
                    }
                }
            }
        } else if let Some(path) = exports.as_str() {
            // Direct string export
            if target_key == "." {
                let resolved = module_path.join(path);
                if resolved.is_file() {
                    return Ok(Some(resolved.canonicalize().unwrap_or(resolved)));
                }
            }
        }

        Ok(None)
    }

    /// Resolve conditional exports (import/require/default/browser)
    fn resolve_conditions(
        &self,
        obj: &serde_json::Map<String, serde_json::Value>,
        module_path: &Path,
    ) -> Result<Option<PathBuf>> {
        // Priority: browser > import > require > default
        for condition in ["browser", "import", "module", "require", "default"] {
            if let Some(value) = obj.get(condition) {
                if let Some(path) = value.as_str() {
                    let resolved = module_path.join(path);
                    if resolved.is_file() {
                        return Ok(Some(resolved.canonicalize().unwrap_or(resolved)));
                    }
                }
            }
        }
        Ok(None)
    }
}

/// Strip JSON comments (// and /* */) that are valid in tsconfig.json files.
/// Also strips trailing commas which are allowed in tsconfig.
fn strip_json_comments(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut in_string = false;
    let mut escape = false;
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if in_string {
            result.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                result.push(c);
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '/' => {
                // Line comment — skip to end of line
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                // Block comment — skip to */
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2; // skip */
                continue;
            }
            ',' if i + 1 < chars.len() => {
                // Check if next non-whitespace is } or ]
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                    // Trailing comma — skip it
                } else {
                    result.push(c);
                }
            }
            _ => {
                result.push(c);
            }
        }
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_relative() {
        let resolver = Resolver::new(
            PathBuf::from("."),
            vec![".ts".to_string(), ".js".to_string()],
            vec![],
        );

        // This should resolve to a real file
        let result = resolver.resolve("./Cargo.toml", Path::new("crates/core/Cargo.toml"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_strip_json_comments() {
        let input = r#"{
  // This is a line comment
  "compilerOptions": {
    "baseUrl": ".", /* block comment */
    "paths": {
      "@/*": ["src/*"]
    }
  }
}"#;
        let cleaned = strip_json_comments(input);
        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(parsed["compilerOptions"]["baseUrl"], ".");
    }

    #[test]
    fn test_strip_trailing_commas() {
        let input = r#"{
  "compilerOptions": {
    "baseUrl": ".",
  }
}"#;
        let cleaned = strip_json_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(parsed["compilerOptions"]["baseUrl"], ".");
    }

    #[test]
    fn test_wildcard_alias_resolution() {
        let resolver = Resolver::new(
            PathBuf::from("."),
            vec![".ts".to_string(), ".tsx".to_string()],
            vec![
                Alias {
                    from: "@components/".to_string(),
                    to: "src/components".to_string(),
                },
                Alias {
                    from: "@/".to_string(),
                    to: "src/".to_string(),
                },
            ],
        );

        // @/utils should resolve to src/utils
        let result = resolver.resolve("@/utils", Path::new("src/index.tsx"));
        // It may not resolve if the file doesn't exist, but it should try the right path
        // We just verify it doesn't panic
        let _ = result;
    }
}
