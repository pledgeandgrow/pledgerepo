// G7.6: Plugin output schema versioning
//
// Each plugin output record carries an optional `schema_version` field.
// The host validates this version against its supported versions and
// rejects outputs with incompatible schemas.
//
// This prevents silent corruption when a plugin produces output in a
// format the host doesn't understand (e.g., after a breaking change in
// the output record structure).
//
// Versioning rules:
//   - Version 0 / None: legacy output (no version checking, backwards compatible)
//   - Version 1: current output schema
//   - Breaking changes increment the version; the host must support all
//     active versions and reject unknown ones.
//   - Additive changes (new optional fields) do NOT increment the version.

use std::collections::HashMap;
use std::fmt;

/// The current output schema version supported by the host.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// The minimum schema version the host can understand.
pub const MIN_SCHEMA_VERSION: u32 = 1;

/// Error returned when a plugin output has an incompatible schema version.
#[derive(Debug, Clone)]
pub struct SchemaVersionError {
    pub plugin_name: String,
    pub hook: String,
    pub output_version: u32,
    pub supported_versions: Vec<u32>,
}

impl fmt::Display for SchemaVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Plugin '{}' returned output with schema version {} for hook '{}', \
             but host supports versions {:?}",
            self.plugin_name, self.output_version, self.hook, self.supported_versions
        )
    }
}

impl std::error::Error for SchemaVersionError {}

/// G7.6: Schema version validator for plugin outputs.
///
/// Tracks which schema versions are supported for each hook and validates
/// plugin outputs at runtime. When a plugin returns an output with an
/// unsupported schema version, the validator returns an error and the
/// host can skip the output or fail the build.
#[derive(Debug, Clone)]
pub struct SchemaVersionValidator {
    /// Map of hook name → set of supported schema versions.
    supported: HashMap<String, Vec<u32>>,
}

impl SchemaVersionValidator {
    /// Create a new validator with default supported versions for all hooks.
    pub fn new() -> Self {
        let mut supported = HashMap::new();
        let current_versions = vec![CURRENT_SCHEMA_VERSION];

        for hook in &[
            "resolve-id",
            "load",
            "transform",
            "transform-index-html",
            "render-chunk",
        ] {
            supported.insert(hook.to_string(), current_versions.clone());
        }

        SchemaVersionValidator { supported }
    }

    /// Validate a plugin output's schema version.
    ///
    /// Returns `Ok(())` if the version is supported (or `None` for legacy),
    /// or an error describing the mismatch.
    pub fn validate(
        &self,
        plugin_name: &str,
        hook: &str,
        schema_version: Option<u32>,
    ) -> Result<(), SchemaVersionError> {
        match schema_version {
            None | Some(0) => {
                // Legacy output — no version checking
                Ok(())
            }
            Some(version) => {
                let supported = self.supported.get(hook);
                match supported {
                    Some(versions) if versions.contains(&version) => Ok(()),
                    Some(versions) => Err(SchemaVersionError {
                        plugin_name: plugin_name.to_string(),
                        hook: hook.to_string(),
                        output_version: version,
                        supported_versions: versions.clone(),
                    }),
                    None => {
                        // Unknown hook — allow it (forward compatibility)
                        Ok(())
                    }
                }
            }
        }
    }

    /// Add support for an additional schema version for a hook.
    ///
    /// This is used when the host is updated to handle a new output format
    /// from plugins that have already incremented their schema version.
    pub fn add_supported_version(&mut self, hook: &str, version: u32) {
        self.supported
            .entry(hook.to_string())
            .or_default()
            .push(version);
    }

    /// Get the list of supported schema versions for a hook.
    pub fn supported_versions(&self, hook: &str) -> &[u32] {
        self.supported
            .get(hook)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a schema version is supported for a hook.
    pub fn is_supported(&self, hook: &str, version: u32) -> bool {
        self.supported
            .get(hook)
            .map(|v| v.contains(&version))
            .unwrap_or(false)
    }
}

impl Default for SchemaVersionValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_current_version() {
        let validator = SchemaVersionValidator::new();
        assert!(validator.validate("test-plugin", "transform", Some(1)).is_ok());
    }

    #[test]
    fn validate_accepts_legacy_none() {
        let validator = SchemaVersionValidator::new();
        assert!(validator.validate("test-plugin", "transform", None).is_ok());
    }

    #[test]
    fn validate_accepts_legacy_zero() {
        let validator = SchemaVersionValidator::new();
        assert!(validator.validate("test-plugin", "transform", Some(0)).is_ok());
    }

    #[test]
    fn validate_rejects_unknown_version() {
        let validator = SchemaVersionValidator::new();
        let result = validator.validate("test-plugin", "transform", Some(99));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.output_version, 99);
        assert_eq!(err.plugin_name, "test-plugin");
        assert_eq!(err.hook, "transform");
    }

    #[test]
    fn validate_allows_added_version() {
        let mut validator = SchemaVersionValidator::new();
        validator.add_supported_version("transform", 2);
        assert!(validator.validate("test-plugin", "transform", Some(2)).is_ok());
    }

    #[test]
    fn validate_all_hooks_supported() {
        let validator = SchemaVersionValidator::new();
        for hook in &[
            "resolve-id",
            "load",
            "transform",
            "transform-index-html",
            "render-chunk",
        ] {
            assert!(
                validator.validate("test-plugin", hook, Some(CURRENT_SCHEMA_VERSION)).is_ok(),
                "Hook {} should support version {}",
                hook,
                CURRENT_SCHEMA_VERSION
            );
        }
    }

    #[test]
    fn is_supported_checks_version() {
        let validator = SchemaVersionValidator::new();
        assert!(validator.is_supported("transform", 1));
        assert!(!validator.is_supported("transform", 2));
        assert!(!validator.is_supported("unknown-hook", 1));
    }

    #[test]
    fn supported_versions_returns_list() {
        let validator = SchemaVersionValidator::new();
        let versions = validator.supported_versions("transform");
        assert!(!versions.is_empty());
        assert!(versions.contains(&CURRENT_SCHEMA_VERSION));
    }

    #[test]
    fn schema_version_error_displays_correctly() {
        let err = SchemaVersionError {
            plugin_name: "my-plugin".to_string(),
            hook: "transform".to_string(),
            output_version: 99,
            supported_versions: vec![1],
        };
        let msg = err.to_string();
        assert!(msg.contains("my-plugin"));
        assert!(msg.contains("99"));
        assert!(msg.contains("transform"));
    }
}
