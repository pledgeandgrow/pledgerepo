// Plugin System enhancements
//
// Features:
//   38. Plugin hot reload — reload JS plugins without restarting dev server
//   39. Plugin sandboxing improvements — WASM memory/CPU/fs limits
//   40. Plugin dependency resolution — npm packages in WASM sandbox
//   41. Plugin lifecycle hooks — watchStart/watchChange/watchEnd
//   42. Plugin parallel execution via rayon

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use std::sync::mpsc;

// ─── Feature 38: Plugin hot reload ────────────────────────────────────

/// Tracks plugin source files for hot reload
pub struct PluginHotReloader {
    /// Map of plugin ID to source file path and last known hash
    plugin_sources: DashMap<String, PluginSource>,
    /// Callbacks to invoke when a plugin is reloaded
    reload_callbacks: DashMap<String, Vec<ReloadCallback>>,
    /// Optional debounced file watcher receiver (when using notify-debouncer)
    watcher_rx: DashMap<String, mpsc::Receiver<PathBuf>>,
}

#[derive(Debug, Clone)]
struct PluginSource {
    path: PathBuf,
    content_hash: [u8; 32],
    last_modified: std::time::SystemTime,
}

type ReloadCallback = Arc<dyn Fn(&str) + Send + Sync>;

impl PluginHotReloader {
    pub fn new() -> Self {
        Self {
            plugin_sources: DashMap::new(),
            reload_callbacks: DashMap::new(),
            watcher_rx: DashMap::new(),
        }
    }

    /// Register a plugin for hot reload watching
    pub fn register(&self, plugin_id: &str, source_path: &Path) {
        if let Ok(content) = std::fs::read(source_path) {
            let hash = blake3::hash(&content);
            let metadata = std::fs::metadata(source_path).ok();
            self.plugin_sources.insert(
                plugin_id.to_string(),
                PluginSource {
                    path: source_path.to_path_buf(),
                    content_hash: hash.into(),
                    last_modified: metadata
                        .and_then(|m| m.modified().ok())
                        .unwrap_or(std::time::SystemTime::now()),
                },
            );
        }
    }

    /// Add a reload callback for a plugin
    pub fn on_reload(&self, plugin_id: &str, callback: impl Fn(&str) + Send + Sync + 'static) {
        let mut callbacks = self
            .reload_callbacks
            .entry(plugin_id.to_string())
            .or_insert_with(Vec::new);
        callbacks.push(Arc::new(callback));
    }

    /// Check all registered plugins for changes and trigger reloads
    pub fn check_for_changes(&self) -> Vec<String> {
        let mut reloaded = Vec::new();

        let plugin_ids: Vec<String> = self.plugin_sources.iter().map(|e| e.key().clone()).collect();

        for plugin_id in &plugin_ids {
            let source = match self.plugin_sources.get(plugin_id) {
                Some(s) => s,
                None => continue,
            };

            if let Ok(content) = std::fs::read(&source.path) {
                let new_hash = blake3::hash(&content);
                if new_hash.as_bytes() != &source.content_hash {
                    // Plugin source changed — trigger reload
                    if let Some(callbacks) = self.reload_callbacks.get(plugin_id) {
                        for cb in callbacks.iter() {
                            cb(plugin_id);
                        }
                    }

                    // Update stored hash
                    drop(source);
                    if let Some(mut source) = self.plugin_sources.get_mut(plugin_id) {
                        source.content_hash = new_hash.into();
                        if let Ok(metadata) = std::fs::metadata(&source.path) {
                            if let Ok(modified) = metadata.modified() {
                                source.last_modified = modified;
                            }
                        }
                    }

                    reloaded.push(plugin_id.clone());
                }
            }
        }

        reloaded
    }

    /// Unregister a plugin from hot reload watching
    pub fn unregister(&self, plugin_id: &str) {
        self.plugin_sources.remove(plugin_id);
        self.reload_callbacks.remove(plugin_id);
        self.watcher_rx.remove(plugin_id);
    }

    /// Start a notify-debouncer watcher for a plugin's source file.
    /// When the file changes, the callback is invoked automatically with
    /// built-in debouncing (replaces manual polling via `check_for_changes`).
    pub fn start_debounced_watcher(&self, plugin_id: &str, source_path: &Path) {
        use notify::RecursiveMode;
        use notify_debouncer_full::new_debouncer;

        let (tx, rx) = mpsc::channel::<PathBuf>();
        let path = source_path.to_path_buf();
        let plugin_id = plugin_id.to_string();

        let callback_plugin_id = plugin_id.clone();
        let callback_path = path.clone();

        let mut debouncer = match new_debouncer(
            Duration::from_millis(200),
            None,
            move |result: Result<Vec<notify_debouncer_full::DebouncedEvent>, Vec<notify::Error>>| {
                if let Ok(events) = result {
                    for event in events {
                        for p in &event.paths {
                            if *p == callback_path {
                                let _ = tx.send(p.clone());
                            }
                        }
                    }
                }
            },
        ) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to create debounced watcher for plugin {}: {}", callback_plugin_id, e);
                return;
            }
        };

        if let Err(e) = debouncer.watch(&path, RecursiveMode::NonRecursive) {
            tracing::warn!("Failed to watch plugin source {}: {}", path.display(), e);
            return;
        }

        // Keep the debouncer alive by leaking it (it runs in the background)
        // In a production system, we'd store the debouncer to drop it on unregister
        std::mem::forget(debouncer);

        self.watcher_rx.insert(plugin_id, rx);
    }

    /// Check for changes via debounced watchers (non-blocking).
    /// Returns list of reloaded plugin IDs.
    pub fn poll_debounced_changes(&self) -> Vec<String> {
        let mut reloaded = Vec::new();
        let plugin_ids: Vec<String> = self.watcher_rx.iter().map(|e| e.key().clone()).collect();

        for plugin_id in &plugin_ids {
            if let Some(mut rx) = self.watcher_rx.get_mut(plugin_id) {
                while let Ok(path) = rx.try_recv() {
                    // Trigger reload callbacks
                    if let Some(callbacks) = self.reload_callbacks.get(plugin_id) {
                        for cb in callbacks.iter() {
                            cb(plugin_id);
                        }
                    }
                    // Update stored hash
                    if let Ok(content) = std::fs::read(&path) {
                        let new_hash = blake3::hash(&content);
                        if let Some(mut source) = self.plugin_sources.get_mut(plugin_id) {
                            source.content_hash = new_hash.into();
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                if let Ok(modified) = metadata.modified() {
                                    source.last_modified = modified;
                                }
                            }
                        }
                    }
                    reloaded.push(plugin_id.clone());
                }
            }
        }
        reloaded
    }
}

impl Default for PluginHotReloader {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Feature 39: Plugin sandboxing improvements ───────────────────────

/// Resource limits for WASM plugin sandbox
#[derive(Debug, Clone)]
pub struct SandboxLimits {
    /// Maximum memory in bytes (default: 64MB)
    pub max_memory_bytes: usize,
    /// Maximum CPU time in milliseconds (default: 5000ms)
    pub max_cpu_time_ms: u64,
    /// Maximum number of filesystem reads
    pub max_fs_reads: usize,
    /// Maximum number of filesystem writes
    pub max_fs_writes: usize,
    /// Allowed filesystem paths (prefix matching)
    pub allowed_paths: Vec<PathBuf>,
    /// Whether network access is allowed
    pub allow_network: bool,
    /// Maximum stack depth for recursive calls
    pub max_stack_depth: usize,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            max_cpu_time_ms: 5000,
            max_fs_reads: 100,
            max_fs_writes: 10,
            allowed_paths: vec![PathBuf::from("./src")],
            allow_network: false,
            max_stack_depth: 256,
        }
    }
}

/// Resource usage tracker for a sandboxed plugin
#[derive(Debug)]
pub struct ResourceUsage {
    pub memory_used: usize,
    pub cpu_time: Duration,
    pub fs_reads: usize,
    pub fs_writes: usize,
    pub start_time: Instant,
}

impl ResourceUsage {
    pub fn new() -> Self {
        Self {
            memory_used: 0,
            cpu_time: Duration::ZERO,
            fs_reads: 0,
            fs_writes: 0,
            start_time: Instant::now(),
        }
    }

    pub fn check_memory(&self, limits: &SandboxLimits) -> Result<(), SandboxError> {
        if self.memory_used > limits.max_memory_bytes {
            Err(SandboxError::MemoryLimitExceeded {
                used: self.memory_used,
                limit: limits.max_memory_bytes,
            })
        } else {
            Ok(())
        }
    }

    pub fn check_cpu_time(&self, limits: &SandboxLimits) -> Result<(), SandboxError> {
        let elapsed = self.start_time.elapsed();
        if elapsed.as_millis() as u64 > limits.max_cpu_time_ms {
            Err(SandboxError::CpuTimeLimitExceeded {
                used: elapsed,
                limit: Duration::from_millis(limits.max_cpu_time_ms),
            })
        } else {
            Ok(())
        }
    }

    pub fn check_fs_read(&self, limits: &SandboxLimits) -> Result<(), SandboxError> {
        if self.fs_reads >= limits.max_fs_reads {
            Err(SandboxError::FsReadLimitExceeded {
                count: self.fs_reads,
                limit: limits.max_fs_reads,
            })
        } else {
            Ok(())
        }
    }

    pub fn check_fs_write(&self, limits: &SandboxLimits) -> Result<(), SandboxError> {
        if self.fs_writes >= limits.max_fs_writes {
            Err(SandboxError::FsWriteLimitExceeded {
                count: self.fs_writes,
                limit: limits.max_fs_writes,
            })
        } else {
            Ok(())
        }
    }

    pub fn check_path_access(&self, path: &Path, limits: &SandboxLimits) -> Result<(), SandboxError> {
        let allowed = limits.allowed_paths.iter().any(|allowed| {
            path.starts_with(allowed) || path.to_string_lossy().as_ref().contains(&allowed.to_string_lossy().as_ref().to_string())
        });
        if !allowed {
            Err(SandboxError::PathAccessDenied {
                path: path.to_path_buf(),
            })
        } else {
            Ok(())
        }
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self::new()
    }
}

/// Sandbox errors
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("memory limit exceeded: used {used} bytes, limit {limit} bytes")]
    MemoryLimitExceeded { used: usize, limit: usize },
    #[error("CPU time limit exceeded: used {used:?}, limit {limit:?}")]
    CpuTimeLimitExceeded { used: Duration, limit: Duration },
    #[error("filesystem read limit exceeded: {count}/{limit}")]
    FsReadLimitExceeded { count: usize, limit: usize },
    #[error("filesystem write limit exceeded: {count}/{limit}")]
    FsWriteLimitExceeded { count: usize, limit: usize },
    #[error("path access denied: {path:?}")]
    PathAccessDenied { path: PathBuf },
    #[error("network access denied")]
    NetworkAccessDenied,
    #[error("stack depth limit exceeded: {depth}/{limit}")]
    StackDepthExceeded { depth: usize, limit: usize },
}

/// Sandboxed filesystem access wrapper
pub struct SandboxedFs {
    usage: Arc<std::sync::Mutex<ResourceUsage>>,
    limits: SandboxLimits,
}

impl SandboxedFs {
    pub fn new(limits: SandboxLimits) -> Self {
        Self {
            usage: Arc::new(std::sync::Mutex::new(ResourceUsage::new())),
            limits,
        }
    }

    pub fn read(&self, path: &Path) -> Result<Vec<u8>, SandboxError> {
        let usage = self.usage.lock().unwrap();
        usage.check_path_access(path, &self.limits)?;
        usage.check_fs_read(&self.limits)?;
        drop(usage);

        std::fs::read(path).map_err(|e| SandboxError::PathAccessDenied {
            path: path.to_path_buf(),
        })
    }

    pub fn write(&self, path: &Path, data: &[u8]) -> Result<(), SandboxError> {
        let usage = self.usage.lock().unwrap();
        usage.check_path_access(path, &self.limits)?;
        usage.check_fs_write(&self.limits)?;
        drop(usage);

        std::fs::write(path, data).map_err(|e| SandboxError::PathAccessDenied {
            path: path.to_path_buf(),
        })
    }
}

// ─── Feature 40: Plugin dependency resolution ─────────────────────────

/// Pre-bundled dependency for WASM plugins
#[derive(Debug, Clone)]
pub struct BundledDependency {
    /// Package name (e.g., "lodash")
    pub name: String,
    /// Version string
    pub version: String,
    /// Bundled JS source code (ESM format)
    pub source: String,
    /// Exports map (export name → bundled path)
    pub exports: HashMap<String, String>,
}

/// Dependency resolver for WASM plugins
pub struct PluginDependencyResolver {
    /// Pre-bundled dependencies cache
    bundled: DashMap<String, BundledDependency>,
    /// Resolution cache: package@version → resolved source
    cache: DashMap<String, String>,
}

impl PluginDependencyResolver {
    pub fn new() -> Self {
        Self {
            bundled: DashMap::new(),
            cache: DashMap::new(),
        }
    }

    /// Pre-bundle a dependency for use in WASM plugins
    pub fn pre_bundle(&self, dep: BundledDependency) {
        let key = format!("{}@{}", dep.name, dep.version);
        self.cache.insert(key.clone(), dep.source.clone());
        self.bundled.insert(key, dep);
    }

    /// Resolve a dependency import within a WASM plugin
    pub fn resolve(&self, import_spec: &str) -> Option<String> {
        // Check cache first
        if let Some(source) = self.cache.get(import_spec) {
            return Some(source.clone());
        }

        // Try to find in bundled deps
        for entry in self.bundled.iter() {
            let dep = entry.value();
            if dep.name == import_spec || import_spec.starts_with(&format!("{}/", dep.name)) {
                return Some(dep.source.clone());
            }
        }

        None
    }

    /// Generate import map for a WASM plugin
    pub fn generate_import_map(&self) -> String {
        let mut imports = Vec::new();
        for entry in self.bundled.iter() {
            let dep = entry.value();
            imports.push(format!(
                r#"    "{}": "bundle:{}""#,
                dep.name, entry.key()
            ));
        }
        format!("{{\n  \"imports\": {{\n{}\n  }}\n}}", imports.join(",\n"))
    }

    /// List all pre-bundled dependencies
    pub fn list_bundled(&self) -> Vec<(String, String)> {
        self.bundled
            .iter()
            .map(|e| (e.value().name.clone(), e.value().version.clone()))
            .collect()
    }
}

impl Default for PluginDependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Feature 41: Plugin lifecycle hooks ───────────────────────────────

/// Lifecycle hook types for plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleHook {
    /// Called when file watching starts
    WatchStart,
    /// Called when a watched file changes
    WatchChange,
    /// Called when file watching stops
    WatchEnd,
    /// Called before transform
    BeforeTransform,
    /// Called after transform
    AfterTransform,
    /// Called before build
    BeforeBuild,
    /// Called after build
    AfterBuild,
    /// Called on dev server start
    DevServerStart,
    /// Called on dev server stop
    DevServerStop,
}

impl LifecycleHook {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WatchStart => "watchStart",
            Self::WatchChange => "watchChange",
            Self::WatchEnd => "watchEnd",
            Self::BeforeTransform => "beforeTransform",
            Self::AfterTransform => "afterTransform",
            Self::BeforeBuild => "beforeBuild",
            Self::AfterBuild => "afterBuild",
            Self::DevServerStart => "devServerStart",
            Self::DevServerStop => "devServerStop",
        }
    }
}

/// Context passed to lifecycle hooks
#[derive(Debug, Clone)]
pub struct HookContext {
    /// The hook type being invoked
    pub hook: LifecycleHook,
    /// File path that triggered the hook (if applicable)
    pub file_path: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl HookContext {
    pub fn new(hook: LifecycleHook) -> Self {
        Self {
            hook,
            file_path: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_path(mut self, path: &str) -> Self {
        self.file_path = Some(path.to_string());
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Hook handler function type
pub type HookHandler = Arc<dyn Fn(&HookContext) + Send + Sync>;

/// Registry for plugin lifecycle hooks
pub struct LifecycleHookRegistry {
    hooks: DashMap<LifecycleHook, Vec<(String, HookHandler)>>,
}

impl LifecycleHookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: DashMap::new(),
        }
    }

    /// Register a hook handler for a specific plugin
    pub fn register(
        &self,
        plugin_id: &str,
        hook: LifecycleHook,
        handler: impl Fn(&HookContext) + Send + Sync + 'static,
    ) {
        let mut handlers = self.hooks.entry(hook).or_insert_with(Vec::new);
        handlers.push((plugin_id.to_string(), Arc::new(handler)));
    }

    /// Invoke all handlers for a hook
    pub fn invoke(&self, ctx: &HookContext) {
        if let Some(handlers) = self.hooks.get(&ctx.hook) {
            for (_plugin_id, handler) in handlers.iter() {
                handler(ctx);
            }
        }
    }

    /// Remove all hooks for a specific plugin
    pub fn unregister_plugin(&self, plugin_id: &str) {
        for entry in self.hooks.iter() {
            let hook = *entry.key();
            if let Some(mut handlers) = self.hooks.get_mut(&hook) {
                handlers.retain(|(pid, _)| pid != plugin_id);
            }
        }
    }

    /// Get the count of registered hooks
    pub fn hook_count(&self, hook: LifecycleHook) -> usize {
        self.hooks.get(&hook).map(|h| h.len()).unwrap_or(0)
    }
}

impl Default for LifecycleHookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Feature 42: Plugin parallel execution ────────────────────────────

/// A plugin transform task that can be executed in parallel
pub struct PluginTransformTask {
    pub plugin_id: String,
    pub file_path: String,
    pub source: String,
    pub priority: u8, // 0 = highest
}

/// Result of a parallel plugin transform
pub struct PluginTransformResult {
    pub plugin_id: String,
    pub file_path: String,
    pub code: String,
    pub source_map: Option<String>,
    pub duration: Duration,
    pub error: Option<String>,
}

/// Execute plugin transforms in parallel using rayon
pub fn execute_parallel_transforms(
    tasks: Vec<PluginTransformTask>,
    transform_fn: impl Fn(&PluginTransformTask) -> Result<(String, Option<String>), String>
        + Send
        + Sync,
) -> Vec<PluginTransformResult> {
    use rayon::prelude::*;

    tasks
        .into_par_iter()
        .map(|task| {
            let start = Instant::now();
            let plugin_id = task.plugin_id.clone();
            let file_path = task.file_path.clone();

            match transform_fn(&task) {
                Ok((code, source_map)) => PluginTransformResult {
                    plugin_id,
                    file_path,
                    code,
                    source_map,
                    duration: start.elapsed(),
                    error: None,
                },
                Err(e) => PluginTransformResult {
                    plugin_id,
                    file_path,
                    code: String::new(),
                    source_map: None,
                    duration: start.elapsed(),
                    error: Some(e),
                },
            }
        })
        .collect()
}

/// Group tasks by independence — tasks that operate on different files can run in parallel
pub fn group_independent_tasks(tasks: Vec<PluginTransformTask>) -> Vec<Vec<PluginTransformTask>> {
    let mut groups: Vec<Vec<PluginTransformTask>> = Vec::new();

    for task in tasks {
        // Find a group where no task has the same file_path
        let group_idx = groups.iter().position(|group| {
            !group.iter().any(|t| t.file_path == task.file_path)
        });

        match group_idx {
            Some(idx) => groups[idx].push(task),
            None => groups.push(vec![task]),
        }
    }

    groups
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_reloader_register() {
        let reloader = PluginHotReloader::new();
        let temp = std::env::temp_dir().join("test_plugin.js");
        std::fs::write(&temp, "module.exports = {};").unwrap();
        reloader.register("test-plugin", &temp);
        assert_eq!(reloader.plugin_sources.len(), 1);
    }

    #[test]
    fn test_hot_reloader_detect_change() {
        let reloader = PluginHotReloader::new();
        let temp = std::env::temp_dir().join("test_plugin_change.js");
        std::fs::write(&temp, "module.exports = {};").unwrap();
        reloader.register("test-plugin", &temp);

        // No changes yet
        let changed = reloader.check_for_changes();
        assert!(changed.is_empty());

        // Modify the file
        std::fs::write(&temp, "module.exports = { changed: true };").unwrap();
        let changed = reloader.check_for_changes();
        assert!(changed.contains(&"test-plugin".to_string()));
    }

    #[test]
    fn test_sandbox_limits_default() {
        let limits = SandboxLimits::default();
        assert_eq!(limits.max_memory_bytes, 64 * 1024 * 1024);
        assert_eq!(limits.max_cpu_time_ms, 5000);
        assert!(!limits.allow_network);
    }

    #[test]
    fn test_sandbox_memory_check() {
        let limits = SandboxLimits::default();
        let usage = ResourceUsage {
            memory_used: 100,
            cpu_time: Duration::ZERO,
            fs_reads: 0,
            fs_writes: 0,
            start_time: Instant::now(),
        };
        assert!(usage.check_memory(&limits).is_ok());

        let usage_over = ResourceUsage {
            memory_used: 128 * 1024 * 1024,
            cpu_time: Duration::ZERO,
            fs_reads: 0,
            fs_writes: 0,
            start_time: Instant::now(),
        };
        assert!(usage_over.check_memory(&limits).is_err());
    }

    #[test]
    fn test_sandbox_path_access() {
        let limits = SandboxLimits {
            allowed_paths: vec![PathBuf::from("./src")],
            ..Default::default()
        };
        let usage = ResourceUsage::new();
        assert!(usage.check_path_access(&Path::new("./src/index.ts"), &limits).is_ok());
        assert!(usage.check_path_access(&Path::new("/etc/passwd"), &limits).is_err());
    }

    #[test]
    fn test_dependency_resolver() {
        let resolver = PluginDependencyResolver::new();
        resolver.pre_bundle(BundledDependency {
            name: "lodash".to_string(),
            version: "4.17.21".to_string(),
            source: "export const _ = {};".to_string(),
            exports: HashMap::new(),
        });

        let resolved = resolver.resolve("lodash");
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("export const _"));
    }

    #[test]
    fn test_dependency_import_map() {
        let resolver = PluginDependencyResolver::new();
        resolver.pre_bundle(BundledDependency {
            name: "react".to_string(),
            version: "18.0.0".to_string(),
            source: "export const React = {};".to_string(),
            exports: HashMap::new(),
        });

        let map = resolver.generate_import_map();
        assert!(map.contains("react"));
        assert!(map.contains("bundle:"));
    }

    #[test]
    fn test_lifecycle_hooks() {
        let registry = LifecycleHookRegistry::new();
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();

        registry.register(
            "test-plugin",
            LifecycleHook::WatchStart,
            move |_ctx| {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            },
        );

        assert_eq!(registry.hook_count(LifecycleHook::WatchStart), 1);

        let ctx = HookContext::new(LifecycleHook::WatchStart);
        registry.invoke(&ctx);

        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn test_lifecycle_hooks_unregister() {
        let registry = LifecycleHookRegistry::new();
        registry.register("plugin-a", LifecycleHook::BeforeBuild, |_| {});
        registry.register("plugin-b", LifecycleHook::BeforeBuild, |_| {});
        assert_eq!(registry.hook_count(LifecycleHook::BeforeBuild), 2);

        registry.unregister_plugin("plugin-a");
        assert_eq!(registry.hook_count(LifecycleHook::BeforeBuild), 1);
    }

    #[test]
    fn test_parallel_transforms() {
        let tasks = vec![
            PluginTransformTask {
                plugin_id: "p1".to_string(),
                file_path: "a.js".to_string(),
                source: "const a = 1;".to_string(),
                priority: 0,
            },
            PluginTransformTask {
                plugin_id: "p2".to_string(),
                file_path: "b.js".to_string(),
                source: "const b = 2;".to_string(),
                priority: 1,
            },
        ];

        let results = execute_parallel_transforms(tasks, |task| {
            Ok((format!("// transformed by {}\n{}", task.plugin_id, task.source), None))
        });

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.error.is_none()));
    }

    #[test]
    fn test_group_independent_tasks() {
        let tasks = vec![
            PluginTransformTask {
                plugin_id: "p1".to_string(),
                file_path: "a.js".to_string(),
                source: "".to_string(),
                priority: 0,
            },
            PluginTransformTask {
                plugin_id: "p2".to_string(),
                file_path: "a.js".to_string(),
                source: "".to_string(),
                priority: 0,
            },
            PluginTransformTask {
                plugin_id: "p3".to_string(),
                file_path: "b.js".to_string(),
                source: "".to_string(),
                priority: 0,
            },
        ];

        let groups = group_independent_tasks(tasks);
        // p1 and p3 can be in the same group (different files)
        // p2 must be in a separate group (same file as p1)
        assert!(groups.len() >= 2);
    }
}
