// G11.6, G11.7, G11.8: Determinism verification features.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── G11.6: Determinism Provenance Tracking ────────────────────────────

/// Provenance record for a single task execution, tracking the source of
/// every input that contributed to the task's output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// The task ID (content-addressed blake3 hash).
    pub task_id: String,
    /// The function name that produced this task.
    pub function_name: String,
    /// Input provenance: (input_name, input_hash, source).
    pub inputs: Vec<InputProvenance>,
    /// The output hash.
    pub output_hash: String,
    /// Whether this execution was deterministic.
    pub deterministic: bool,
    /// Timestamp of execution.
    pub timestamp: u64,
    /// Environment variables that affected the output.
    pub env_vars: Vec<String>,
    /// File system paths that were read during execution.
    pub file_reads: Vec<String>,
}

/// Provenance for a single input.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputProvenance {
    /// The input name (argument name or file path).
    pub name: String,
    /// The content hash of the input.
    pub hash: String,
    /// The source of the input (task, file, env, network).
    pub source: InputSource,
}

/// The source of a task input.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputSource {
    /// Input came from another task's output.
    Task,
    /// Input came from a file on disk.
    File,
    /// Input came from an environment variable.
    Environment,
    /// Input came from a network request.
    Network,
    /// Input was a constant/literal.
    Constant,
}

/// Provenance tracker that records the source of every input for every task.
pub struct ProvenanceTracker {
    records: HashMap<String, ProvenanceRecord>,
}

impl ProvenanceTracker {
    /// Create a new empty provenance tracker.
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    /// Record a task execution's provenance.
    pub fn record(&mut self, record: ProvenanceRecord) {
        self.records.insert(record.task_id.clone(), record);
    }

    /// Get the provenance for a specific task.
    pub fn get(&self, task_id: &str) -> Option<&ProvenanceRecord> {
        self.records.get(task_id)
    }

    /// Check if a task was deterministic.
    pub fn is_deterministic(&self, task_id: &str) -> Option<bool> {
        self.records.get(task_id).map(|r| r.deterministic)
    }

    /// Get all non-deterministic tasks.
    pub fn non_deterministic_tasks(&self) -> Vec<&ProvenanceRecord> {
        self.records.values().filter(|r| !r.deterministic).collect()
    }

    /// Get all tasks that read from the network (potential non-determinism source).
    pub fn network_dependent_tasks(&self) -> Vec<&ProvenanceRecord> {
        self.records
            .values()
            .filter(|r| r.inputs.iter().any(|i| i.source == InputSource::Network))
            .collect()
    }

    /// Get all tasks that depend on environment variables.
    pub fn env_dependent_tasks(&self) -> Vec<&ProvenanceRecord> {
        self.records
            .values()
            .filter(|r| !r.env_vars.is_empty())
            .collect()
    }

    /// Number of recorded tasks.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the tracker is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Serialize all provenance records to JSON.
    pub fn to_json(&self) -> serde_json::Result<String> {
        let records: Vec<&ProvenanceRecord> = self.records.values().collect();
        serde_json::to_string_pretty(&records)
    }
}

impl Default for ProvenanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ─── G11.7: Determinism Lockfile (pledge.lock) ─────────────────────────

/// The determinism lockfile that pins all inputs for reproducible builds.
///
/// This file is committed to version control and ensures that every developer
/// and CI environment produces identical outputs. It contains:
/// - Content hashes of all source files
/// - Plugin versions and hashes
/// - Configuration hashes
/// - Environment variable requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeterminismLockfile {
    /// Lockfile format version.
    pub version: u32,
    /// PledgePack version that generated this lockfile.
    pub pledgepack_version: String,
    /// File hashes: (relative_path, blake3_hash).
    pub file_hashes: Vec<FileHash>,
    /// Plugin hashes: (plugin_name, version, wasm_hash).
    pub plugin_hashes: Vec<PluginHash>,
    /// Configuration hash.
    pub config_hash: String,
    /// Required environment variables (name, optional hash).
    pub env_requirements: Vec<EnvRequirement>,
    /// Toolchain hash (compiler versions, etc.).
    pub toolchain_hash: String,
}

/// A file hash entry in the lockfile.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileHash {
    pub path: String,
    pub hash: String,
}

/// A plugin hash entry in the lockfile.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginHash {
    pub name: String,
    pub version: String,
    pub wasm_hash: String,
}

/// An environment variable requirement.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvRequirement {
    pub name: String,
    /// Expected value hash (None = any value is acceptable).
    pub value_hash: Option<String>,
}

impl DeterminismLockfile {
    /// Create a new empty lockfile.
    pub fn new(pledgepack_version: &str) -> Self {
        Self {
            version: 1,
            pledgepack_version: pledgepack_version.to_string(),
            file_hashes: Vec::new(),
            plugin_hashes: Vec::new(),
            config_hash: String::new(),
            env_requirements: Vec::new(),
            toolchain_hash: String::new(),
        }
    }

    /// Add a file hash to the lockfile.
    pub fn add_file(&mut self, path: &str, hash: &str) {
        self.file_hashes.push(FileHash {
            path: path.to_string(),
            hash: hash.to_string(),
        });
    }

    /// Add a plugin hash to the lockfile.
    pub fn add_plugin(&mut self, name: &str, version: &str, wasm_hash: &str) {
        self.plugin_hashes.push(PluginHash {
            name: name.to_string(),
            version: version.to_string(),
            wasm_hash: wasm_hash.to_string(),
        });
    }

    /// Add an environment variable requirement.
    pub fn add_env_requirement(&mut self, name: &str, value_hash: Option<&str>) {
        self.env_requirements.push(EnvRequirement {
            name: name.to_string(),
            value_hash: value_hash.map(|s| s.to_string()),
        });
    }

    /// Serialize the lockfile to JSON (for writing to pledge.lock).
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a lockfile from JSON (for reading pledge.lock).
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    /// Compare this lockfile with another and return the differences.
    pub fn diff(&self, other: &DeterminismLockfile) -> LockfileDiff {
        let mut diff = LockfileDiff::default();

        // Compare file hashes
        for fh in &self.file_hashes {
            if let Some(other_fh) = other.file_hashes.iter().find(|f| f.path == fh.path) {
                if fh.hash != other_fh.hash {
                    diff.changed_files.push(fh.path.clone());
                }
            } else {
                diff.removed_files.push(fh.path.clone());
            }
        }
        for fh in &other.file_hashes {
            if !self.file_hashes.iter().any(|f| f.path == fh.path) {
                diff.added_files.push(fh.path.clone());
            }
        }

        // Compare plugin hashes
        for ph in &self.plugin_hashes {
            if let Some(other_ph) = other.plugin_hashes.iter().find(|p| p.name == ph.name) {
                if ph.wasm_hash != other_ph.wasm_hash || ph.version != other_ph.version {
                    diff.changed_plugins.push(ph.name.clone());
                }
            } else {
                diff.removed_plugins.push(ph.name.clone());
            }
        }
        for ph in &other.plugin_hashes {
            if !self.plugin_hashes.iter().any(|p| p.name == ph.name) {
                diff.added_plugins.push(ph.name.clone());
            }
        }

        // Compare config hash
        if self.config_hash != other.config_hash {
            diff.config_changed = true;
        }

        // Compare toolchain hash
        if self.toolchain_hash != other.toolchain_hash {
            diff.toolchain_changed = true;
        }

        diff
    }

    /// Check if this lockfile is compatible with another (no breaking changes).
    pub fn is_compatible_with(&self, other: &DeterminismLockfile) -> bool {
        let diff = self.diff(other);
        diff.is_compatible()
    }
}

/// Differences between two lockfiles.
#[derive(Clone, Debug, Default)]
pub struct LockfileDiff {
    pub added_files: Vec<String>,
    pub removed_files: Vec<String>,
    pub changed_files: Vec<String>,
    pub added_plugins: Vec<String>,
    pub removed_plugins: Vec<String>,
    pub changed_plugins: Vec<String>,
    pub config_changed: bool,
    pub toolchain_changed: bool,
}

impl LockfileDiff {
    /// Whether the diff is empty (lockfiles are identical).
    pub fn is_empty(&self) -> bool {
        self.added_files.is_empty()
            && self.removed_files.is_empty()
            && self.changed_files.is_empty()
            && self.added_plugins.is_empty()
            && self.removed_plugins.is_empty()
            && self.changed_plugins.is_empty()
            && !self.config_changed
            && !self.toolchain_changed
    }

    /// Whether the diff is compatible (no breaking changes).
    pub fn is_compatible(&self) -> bool {
        // Changed files and changed plugins are breaking changes.
        // Added files/plugins are non-breaking (they don't affect existing tasks).
        // Removed files/plugins are breaking (they were dependencies).
        self.changed_files.is_empty()
            && self.removed_files.is_empty()
            && self.changed_plugins.is_empty()
            && self.removed_plugins.is_empty()
            && !self.config_changed
    }
}

// ─── G11.8: Formal Verification with cargo-creusot ─────────────────────

/// Configuration for formal verification of task determinism using creusot.
///
/// Creusot is a deductive verification tool for Rust that can prove
/// properties about Rust code. PledgePack can use it to formally verify
/// that task functions are deterministic.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreusotVerificationConfig {
    /// Whether formal verification is enabled.
    pub enabled: bool,
    /// The creusot binary path (if not in PATH).
    pub creusot_path: Option<String>,
    /// Whether to verify determinism (no external state access).
    pub verify_determinism: bool,
    /// Whether to verify termination (no infinite loops).
    pub verify_termination: bool,
    /// Whether to verify memory safety (no unsafe violations).
    pub verify_memory_safety: bool,
    /// Additional creusot flags.
    pub extra_flags: Vec<String>,
}

impl Default for CreusotVerificationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            creusot_path: None,
            verify_determinism: true,
            verify_termination: false,
            verify_memory_safety: true,
            extra_flags: Vec::new(),
        }
    }
}

/// Result of formal verification for a single task.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationResult {
    /// The task function name.
    pub function_name: String,
    /// Whether verification succeeded.
    pub verified: bool,
    /// Whether determinism was proven.
    pub determinism_proven: bool,
    /// Whether termination was proven.
    pub termination_proven: bool,
    /// Whether memory safety was proven.
    pub memory_safety_proven: bool,
    /// Verification time in milliseconds.
    pub verification_time_ms: u64,
    /// Any proof obligations that failed.
    pub failed_obligations: Vec<String>,
    /// Any warnings from the verifier.
    pub warnings: Vec<String>,
}

impl VerificationResult {
    /// Create a successful verification result.
    pub fn success(function_name: &str) -> Self {
        Self {
            function_name: function_name.to_string(),
            verified: true,
            determinism_proven: true,
            termination_proven: true,
            memory_safety_proven: true,
            verification_time_ms: 0,
            failed_obligations: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed verification result.
    pub fn failure(function_name: &str, obligations: Vec<String>) -> Self {
        Self {
            function_name: function_name.to_string(),
            verified: false,
            determinism_proven: false,
            termination_proven: false,
            memory_safety_proven: false,
            verification_time_ms: 0,
            failed_obligations: obligations,
            warnings: Vec::new(),
        }
    }
}

/// Generate the creusot command for verifying a task function.
pub fn creusot_verify_command(
    config: &CreusotVerificationConfig,
    source_file: &str,
    function_name: &str,
) -> Vec<String> {
    let creusot = config.creusot_path.as_deref().unwrap_or("cargo-creusot");
    let mut cmd = vec![
        creusot.to_string(),
        "prove".to_string(),
        source_file.to_string(),
        "--function".to_string(),
        function_name.to_string(),
    ];

    if config.verify_determinism {
        cmd.push("--determinism".to_string());
    }
    if config.verify_termination {
        cmd.push("--termination".to_string());
    }
    if config.verify_memory_safety {
        cmd.push("--memory-safety".to_string());
    }

    cmd.extend(config.extra_flags.clone());
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g11_6_provenance_tracker_basic() {
        let mut tracker = ProvenanceTracker::new();
        let record = ProvenanceRecord {
            task_id: "abc123".to_string(),
            function_name: "parse_source".to_string(),
            inputs: vec![InputProvenance {
                name: "source".to_string(),
                hash: "def456".to_string(),
                source: InputSource::File,
            }],
            output_hash: "ghi789".to_string(),
            deterministic: true,
            timestamp: 1234567890,
            env_vars: vec![],
            file_reads: vec!["src/main.ts".to_string()],
        };
        tracker.record(record);

        assert_eq!(tracker.len(), 1);
        assert!(tracker.is_deterministic("abc123").unwrap());
        assert!(tracker.non_deterministic_tasks().is_empty());
    }

    #[test]
    fn g11_6_provenance_non_deterministic() {
        let mut tracker = ProvenanceTracker::new();
        let record = ProvenanceRecord {
            task_id: "abc123".to_string(),
            function_name: "fetch_data".to_string(),
            inputs: vec![InputProvenance {
                name: "url".to_string(),
                hash: "def456".to_string(),
                source: InputSource::Network,
            }],
            output_hash: "ghi789".to_string(),
            deterministic: false,
            timestamp: 1234567890,
            env_vars: vec!["API_KEY".to_string()],
            file_reads: vec![],
        };
        tracker.record(record);

        assert!(!tracker.is_deterministic("abc123").unwrap());
        assert_eq!(tracker.non_deterministic_tasks().len(), 1);
        assert_eq!(tracker.network_dependent_tasks().len(), 1);
        assert_eq!(tracker.env_dependent_tasks().len(), 1);
    }

    #[test]
    fn g11_7_lockfile_creation_and_serialization() {
        let mut lockfile = DeterminismLockfile::new("0.2.9");
        lockfile.add_file("src/main.ts", "abc123");
        lockfile.add_file("src/utils.ts", "def456");
        lockfile.add_plugin("@pledge/css", "1.0.0", "hash123");
        lockfile.add_env_requirement("NODE_ENV", Some("production_hash"));
        lockfile.config_hash = "config_hash_abc".to_string();
        lockfile.toolchain_hash = "toolchain_hash_xyz".to_string();

        let json = lockfile.to_json().unwrap();
        let deserialized = DeterminismLockfile::from_json(&json).unwrap();

        assert_eq!(deserialized.pledgepack_version, "0.2.9");
        assert_eq!(deserialized.file_hashes.len(), 2);
        assert_eq!(deserialized.plugin_hashes.len(), 1);
        assert_eq!(deserialized.env_requirements.len(), 1);
    }

    #[test]
    fn g11_7_lockfile_diff_compatible() {
        let mut lockfile1 = DeterminismLockfile::new("0.2.9");
        lockfile1.add_file("src/main.ts", "abc123");
        lockfile1.config_hash = "config1".to_string();

        let mut lockfile2 = lockfile1.clone();
        lockfile2.add_file("src/new.ts", "new_hash");

        let diff = lockfile1.diff(&lockfile2);
        assert!(!diff.is_empty());
        assert!(diff.is_compatible()); // Added file is non-breaking
    }

    #[test]
    fn g11_7_lockfile_diff_incompatible() {
        let mut lockfile1 = DeterminismLockfile::new("0.2.9");
        lockfile1.add_file("src/main.ts", "abc123");
        lockfile1.config_hash = "config1".to_string();

        let mut lockfile2 = lockfile1.clone();
        lockfile2.file_hashes[0].hash = "changed_hash".to_string();

        let diff = lockfile1.diff(&lockfile2);
        assert!(!diff.is_compatible()); // Changed file is breaking
    }

    #[test]
    fn g11_8_creusot_config_default() {
        let config = CreusotVerificationConfig::default();
        assert!(!config.enabled);
        assert!(config.verify_determinism);
        assert!(config.verify_memory_safety);
        assert!(!config.verify_termination);
    }

    #[test]
    fn g11_8_verification_result_success() {
        let result = VerificationResult::success("parse_source");
        assert!(result.verified);
        assert!(result.determinism_proven);
        assert!(result.memory_safety_proven);
    }

    #[test]
    fn g11_8_verification_result_failure() {
        let result = VerificationResult::failure(
            "fetch_data",
            vec!["Cannot prove determinism: network access".to_string()],
        );
        assert!(!result.verified);
        assert!(!result.determinism_proven);
        assert_eq!(result.failed_obligations.len(), 1);
    }

    #[test]
    fn g11_8_creusot_verify_command() {
        let config = CreusotVerificationConfig {
            enabled: true,
            creusot_path: Some("/usr/local/bin/cargo-creusot".to_string()),
            verify_determinism: true,
            verify_termination: false,
            verify_memory_safety: true,
            extra_flags: vec!["--verbose".to_string()],
        };
        let cmd = creusot_verify_command(&config, "src/main.rs", "parse_source");
        assert!(cmd[0].contains("cargo-creusot"));
        assert!(cmd.contains(&"--determinism".to_string()));
        assert!(cmd.contains(&"--memory-safety".to_string()));
        assert!(!cmd.contains(&"--termination".to_string()));
        assert!(cmd.contains(&"--verbose".to_string()));
    }
}
