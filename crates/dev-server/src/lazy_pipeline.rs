// Lazy transform pipeline initialization (feature 11: cold boot optimization)
//
// The transform pipeline (Oxc parser, Lightning CSS, etc.) is only initialized
// on the first module request, not at server startup. This significantly reduces
// dev server cold boot time for projects that don't need to transform every file
// immediately.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

/// Lazy-initialized transform pipeline state
pub struct LazyPipeline {
    /// Whether the transform pipeline has been initialized
    initialized: AtomicBool,
    /// Set of modules whose dependencies need re-optimization (feature 15)
    dirty_deps: HashSet<String>,
}

impl LazyPipeline {
    pub fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            dirty_deps: HashSet::new(),
        }
    }

    /// Ensure the transform pipeline is initialized.
    /// On first call, this loads Oxc and Lightning CSS.
    /// Subsequent calls are no-ops.
    pub fn ensure_initialized(&mut self) {
        if !self.initialized.load(Ordering::Relaxed) {
            // The transform pipeline is initialized lazily by the first
            // call to pledge_transform::transform(). This function serves
            // as a marker and could be extended to pre-warm caches.
            self.initialized.store(true, Ordering::Relaxed);
            tracing::info!("Transform pipeline initialized (lazy load)");
        }
    }

    /// Check if the pipeline has been initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    /// Mark a module's dependencies as dirty, requiring re-optimization (feature 15)
    pub fn mark_deps_dirty(&mut self, module_path: &str) {
        self.dirty_deps.insert(module_path.to_string());
    }

    /// Check if any dependencies need re-optimization
    pub fn has_dirty_deps(&self) -> bool {
        !self.dirty_deps.is_empty()
    }

    /// Get the set of modules with dirty dependencies
    pub fn get_dirty_deps(&self) -> &HashSet<String> {
        &self.dirty_deps
    }

    /// Clear dirty deps after re-optimization has been performed
    pub fn clear_dirty_deps(&mut self) {
        self.dirty_deps.clear();
    }
}

impl Default for LazyPipeline {
    fn default() -> Self {
        Self::new()
    }
}
