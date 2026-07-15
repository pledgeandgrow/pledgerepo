// Serializable module graph for persistent storage and incremental rebuilds.
//
// The persistent module graph is serialized to disk between builds,
// enabling incremental rebuilds by comparing content hashes of the
// current build against the previous one. Only changed modules and
// their dependents are re-transformed.

use crate::module::{ModuleId, ModuleKind};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{debug, info};

/// Serializable representation of a single module in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleNode {
    pub id: ModuleId,
    pub path: PathBuf,
    pub kind: String,
    pub content_hash: u64,
    /// Direct dependency module IDs
    pub dependencies: Vec<ModuleId>,
    /// Dynamic import module IDs
    pub dynamic_dependencies: Vec<ModuleId>,
}

/// Serializable representation of the entire module graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableModuleGraph {
    /// All modules keyed by ModuleId
    pub modules: HashMap<ModuleId, ModuleNode>,
    /// Entry point module IDs
    pub entry_modules: Vec<ModuleId>,
    /// Reverse dependency map: module ID → modules that depend on it
    pub reverse_deps: HashMap<ModuleId, Vec<ModuleId>>,
    /// Build timestamp
    pub built_at: u64,
    /// Git tree hash at build time (for fast cache invalidation)
    #[serde(default)]
    pub git_tree_hash: Option<String>,
}

impl SerializableModuleGraph {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            entry_modules: Vec::new(),
            reverse_deps: HashMap::new(),
            built_at: 0,
            git_tree_hash: None,
        }
    }

    /// Add a module to the graph
    pub fn add_module(
        &mut self,
        id: ModuleId,
        path: PathBuf,
        kind: ModuleKind,
        content_hash: u64,
    ) {
        let kind_str = match kind {
            ModuleKind::Tsx => "tsx",
            ModuleKind::TypeScript => "ts",
            ModuleKind::Jsx => "jsx",
            ModuleKind::JavaScript => "js",
            ModuleKind::Css => "css",
            ModuleKind::Json => "json",
            ModuleKind::Wasm => "wasm",
            ModuleKind::Vue => "vue",
            ModuleKind::Svelte => "svelte",
            ModuleKind::Astro => "astro",
            ModuleKind::Worker => "worker",
            ModuleKind::Asset => "asset",
            ModuleKind::Mdx => "mdx",
            ModuleKind::Graphql => "graphql",
            ModuleKind::Yaml => "yaml",
            ModuleKind::Csv => "csv",
            ModuleKind::Tsv => "tsv",
            ModuleKind::Unknown => "unknown",
        };

        self.modules.insert(
            id,
            ModuleNode {
                id,
                path,
                kind: kind_str.to_string(),
                content_hash,
                dependencies: Vec::new(),
                dynamic_dependencies: Vec::new(),
            },
        );
    }

    /// Add a static dependency edge
    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId) {
        if let Some(module) = self.modules.get_mut(&from) {
            if !module.dependencies.contains(&to) {
                module.dependencies.push(to);
            }
        }
        self.reverse_deps
            .entry(to)
            .or_default()
            .push(from);
    }

    /// Add a dynamic import edge
    pub fn add_dynamic_dependency(&mut self, from: ModuleId, to: ModuleId) {
        if let Some(module) = self.modules.get_mut(&from) {
            if !module.dynamic_dependencies.contains(&to) {
                module.dynamic_dependencies.push(to);
            }
        }
        self.reverse_deps
            .entry(to)
            .or_default()
            .push(from);
    }

    /// Set entry modules
    pub fn set_entries(&mut self, entries: Vec<ModuleId>) {
        self.entry_modules = entries;
    }

    /// Find all modules that changed by comparing content hashes
    /// with a previous graph snapshot
    pub fn find_changed_modules(&self, previous: &SerializableModuleGraph) -> HashSet<ModuleId> {
        let mut changed: HashSet<ModuleId> = HashSet::new();

        for (id, module) in &self.modules {
            match previous.modules.get(id) {
                Some(prev_module) => {
                    if module.content_hash != prev_module.content_hash {
                        debug!(
                            "Changed module: {:?} (hash {} → {})",
                            module.path, prev_module.content_hash, module.content_hash
                        );
                        changed.insert(*id);
                    }
                }
                None => {
                    debug!("New module: {:?}", module.path);
                    changed.insert(*id);
                }
            }
        }

        // Also mark modules that were removed (their dependents need rebuild)
        for (id, _) in &previous.modules {
            if !self.modules.contains_key(id) {
                if let Some(rev_deps) = previous.reverse_deps.get(id) {
                    for dep_id in rev_deps {
                        if self.modules.contains_key(dep_id) {
                            changed.insert(*dep_id);
                        }
                    }
                }
            }
        }

        changed
    }

    /// Compute the transitive closure of dependents for a set of changed modules.
    /// Returns all modules that need to be re-transformed.
    pub fn compute_affected(&self, changed: &HashSet<ModuleId>) -> HashSet<ModuleId> {
        let mut affected = changed.clone();
        let mut queue: Vec<ModuleId> = changed.iter().copied().collect();

        while let Some(id) = queue.pop() {
            if let Some(rev_deps) = self.reverse_deps.get(&id) {
                for dep_id in rev_deps {
                    if affected.insert(*dep_id) {
                        queue.push(*dep_id);
                    }
                }
            }
        }

        affected
    }

    /// Get all modules that are NOT in the affected set (can be skipped)
    pub fn compute_unchanged(&self, affected: &HashSet<ModuleId>) -> Vec<ModuleId> {
        self.modules
            .keys()
            .filter(|id| !affected.contains(id))
            .copied()
            .collect()
    }

    /// Serialize to disk using bincode
    pub fn save_to_disk(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = bincode::serialize(self)?;
        std::fs::write(path, data)?;
        info!("Module graph saved: {} modules", self.modules.len());
        Ok(())
    }

    /// Load from disk using bincode
    pub fn load_from_disk(path: &PathBuf) -> Result<Self> {
        let data = std::fs::read(path)?;
        let graph: SerializableModuleGraph = bincode::deserialize(&data)?;
        info!("Module graph loaded: {} modules", graph.modules.len());
        Ok(graph)
    }

    /// Check if a graph snapshot exists on disk
    pub fn exists_on_disk(path: &PathBuf) -> bool {
        path.exists()
    }
}

impl Default for SerializableModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_changed_modules() {
        let mut prev = SerializableModuleGraph::new();
        prev.add_module(0, PathBuf::from("/src/a.ts"), ModuleKind::TypeScript, 100);
        prev.add_module(1, PathBuf::from("/src/b.ts"), ModuleKind::TypeScript, 200);
        prev.add_dependency(0, 1);

        let mut curr = SerializableModuleGraph::new();
        curr.add_module(0, PathBuf::from("/src/a.ts"), ModuleKind::TypeScript, 100);
        curr.add_module(1, PathBuf::from("/src/b.ts"), ModuleKind::TypeScript, 999);

        let changed = curr.find_changed_modules(&prev);
        assert!(changed.contains(&1));
        assert!(!changed.contains(&0));
    }

    #[test]
    fn test_affected_dependents() {
        let mut graph = SerializableModuleGraph::new();
        graph.add_module(0, PathBuf::from("/src/a.ts"), ModuleKind::TypeScript, 100);
        graph.add_module(1, PathBuf::from("/src/b.ts"), ModuleKind::TypeScript, 200);
        graph.add_module(2, PathBuf::from("/src/c.ts"), ModuleKind::TypeScript, 300);
        graph.add_dependency(0, 1);
        graph.add_dependency(1, 2);

        let mut changed = HashSet::new();
        changed.insert(2);

        let affected = graph.compute_affected(&changed);
        assert!(affected.contains(&2));
        assert!(affected.contains(&1));
        assert!(affected.contains(&0));
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = std::env::temp_dir().join("pledgepack_graph_test");
        let path = dir.join("module_graph.bin");

        let mut graph = SerializableModuleGraph::new();
        graph.add_module(0, PathBuf::from("/src/a.ts"), ModuleKind::TypeScript, 100);
        graph.add_module(1, PathBuf::from("/src/b.ts"), ModuleKind::TypeScript, 200);
        graph.add_dependency(0, 1);
        graph.set_entries(vec![0]);

        graph.save_to_disk(&path).unwrap();
        let loaded = SerializableModuleGraph::load_from_disk(&path).unwrap();

        assert_eq!(loaded.modules.len(), 2);
        assert_eq!(loaded.entry_modules, vec![0]);
        assert!(loaded.reverse_deps.get(&1).unwrap().contains(&0));
    }
}
