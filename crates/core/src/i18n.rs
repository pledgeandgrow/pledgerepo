// i18n-aware bundling (#106)
//
// Splits bundles by locale. Only loads the current locale's strings.
// Handles `import messages from './messages.${locale}.json'` pattern.

use crate::config::I18nConfig;
use std::path::PathBuf;
use tracing::info;

/// Resolve locale-specific message file path
pub fn resolve_locale_file(pattern: &str, locale: &str, root: &PathBuf) -> PathBuf {
    let path = pattern.replace("${locale}", locale);
    if path.starts_with("./") {
        root.join(path.strip_prefix("./").unwrap_or(&path))
    } else {
        PathBuf::from(path)
    }
}

/// Transform import statements that use ${locale} pattern
/// Replaces `import messages from './messages.${locale}.json'`
/// with a dynamic import based on the detected locale at runtime
pub fn transform_i18n_imports(code: &str, config: &I18nConfig) -> String {
    if !config.enabled || config.locales.is_empty() {
        return code.to_string();
    }

    let mut result = code.to_string();
    let pattern = &config.message_pattern;

    // Replace static imports of ${locale} pattern with runtime locale detection
    let _import_pattern = pattern.replace("${locale}", "\\$\\{locale\\}");

    // Transform: import messages from './messages.${locale}.json'
    // Into: const messages = await import(`./messages.${locale}.json`)
    // with a runtime locale detection shim
    if result.contains(pattern) {
        // Generate locale detection shim + dynamic import
        let locales_json = serde_json::to_string(&config.locales).unwrap_or_default();
        let default_locale = &config.default_locale;

        let shim = format!(
            r#"// i18n locale detection (#106)
const __pledge_locales = {};
const __pledge_defaultLocale = "{}";
const __pledge_locale = (typeof navigator !== 'undefined' && navigator.language)
  ? navigator.language.split('-')[0]
  : __pledge_defaultLocale;
const __pledge_currentLocale = __pledge_locales.includes(__pledge_locale) ? __pledge_locale : __pledge_defaultLocale;
"#,
            locales_json, default_locale,
        );

        // Replace the pattern in import statements
        let resolved_pattern = pattern.replace("${locale}", "${__pledge_currentLocale}");
        result = result.replace(pattern, &resolved_pattern);

        // Prepend the shim
        result = format!("{}\n{}", shim, result);
    }

    info!("i18n transform: {} locales configured", config.locales.len());
    result
}

/// Generate locale-specific entry chunks
pub fn generate_locale_entries(config: &I18nConfig) -> Vec<String> {
    if !config.enabled {
        return Vec::new();
    }
    config.locales.clone()
}

/// Check if a file path matches the i18n message pattern
pub fn is_locale_message_file(path: &str, config: &I18nConfig) -> bool {
    if !config.enabled {
        return false;
    }
    // Check if path matches the pattern (with any locale substituted)
    let pattern = &config.message_pattern;
    let pattern_base = pattern.replace("${locale}", "");

    // Normalize path for comparison
    let path_normalized = path.replace('\\', "/");

    for locale in &config.locales {
        let expected = pattern.replace("${locale}", locale);
        let expected_normalized = expected.replace('\\', "/");
        if path_normalized.ends_with(&expected_normalized)
            || path_normalized == expected_normalized
        {
            return true;
        }
    }

    // Also check if the path contains the pattern base
    if path_normalized.contains(&pattern_base.replace("./", "")) {
        return true;
    }

    false
}
