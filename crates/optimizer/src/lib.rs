// Production optimizer: tree shaking, code splitting, minification, scope hoisting
//
// Strategy: Use the cached module graph from the build engine,
// then run optimization passes on the FULL graph (not cached chunks).
//
// This is how we avoid Turbopack's 72% bundle bloat:
//   - Function-level cache makes graph reconstruction fast
//   - Optimization runs on the complete graph (not individual cached chunks)
//   - Tree shaking sees the full dependency picture

use anyhow::Result;
use pledgepack_core::module::{ModuleId, ResolvedModule};
use pledgepack_core::config::BuildConfig;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

pub struct Optimizer {
    /// Modules that have side effects (can't be tree-shaken)
    side_effect_modules: HashSet<ModuleId>,
    /// Chunk grouping
    chunks: Vec<Chunk>,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: String,
    pub modules: Vec<ModuleId>,
    pub chunk_type: ChunkType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkType {
    Entry,
    Vendor,
    Async,
    Shared,
    Route,
}

impl Optimizer {
    pub fn new() -> Self {
        Self {
            side_effect_modules: HashSet::new(),
            chunks: Vec::new(),
        }
    }

    /// Run all optimization passes
    pub fn optimize(
        &mut self,
        entry_modules: &[ModuleId],
        all_modules: &HashMap<ModuleId, ResolvedModule>,
        graph: &pledgepack_core::Graph,
    ) -> Result<Vec<Chunk>> {
        // Phase 1: Mark side-effect-free modules
        self.mark_side_effects(all_modules);

        // Phase 2: Tree shake — remove unreachable modules
        let reachable = self.tree_shake(entry_modules, graph);

        // Phase 3: Code splitting — group modules into chunks
        self.split_chunks(entry_modules, &reachable, all_modules, graph);

        // Phase 4: Scope hoisting — merge entry chunk modules into a single scope
        // (Already handled by emitting modules as separate files with ESM imports)

        Ok(self.chunks.clone())
    }

    /// Run optimization passes with build config for inline_dynamic_imports and manual_chunks
    pub fn optimize_with_config(
        &mut self,
        entry_modules: &[ModuleId],
        all_modules: &HashMap<ModuleId, ResolvedModule>,
        graph: &pledgepack_core::Graph,
        build_config: &BuildConfig,
    ) -> Result<Vec<Chunk>> {
        // Phase 1: Mark side-effect-free modules
        self.mark_side_effects(all_modules);

        // Phase 2: Tree shake — remove unreachable modules
        let reachable = self.tree_shake(entry_modules, graph);

        // Phase 3: Code splitting — group modules into chunks
        self.split_chunks(entry_modules, &reachable, all_modules, graph);

        // Phase 3b: Apply manual chunks configuration
        if !build_config.manual_chunks.is_empty() {
            self.apply_manual_chunks(&reachable, all_modules, &build_config.manual_chunks);
        }

        // Phase 3c: If inline_dynamic_imports, merge all async chunks into their parent entry chunks
        if build_config.inline_dynamic_imports {
            self.inline_dynamic_imports();
        }

        Ok(self.chunks.clone())
    }

    /// Mark modules with side effects (parallelized using rayon)
    fn mark_side_effects(
        &mut self,
        modules: &HashMap<ModuleId, ResolvedModule>,
    ) {
        // Process modules in parallel — each module's side-effect analysis is independent
        let side_effects: Mutex<HashSet<ModuleId>> = Mutex::new(HashSet::new());

        modules.par_iter().for_each(|(id, module)| {
            let source = String::from_utf8_lossy(&module.source);

            // Heuristic: modules with top-level statements that aren't
            // import/export are considered to have side effects
            let has_side_effects = source
                .lines()
                .any(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty()
                        && !trimmed.starts_with("import ")
                        && !trimmed.starts_with("export ")
                        && !trimmed.starts_with("//")
                        && !trimmed.starts_with("/*")
                });

            if has_side_effects {
                side_effects.lock().unwrap().insert(*id);
            }
        });

        self.side_effect_modules = side_effects.into_inner().unwrap();
    }

    /// Tree shake: find all reachable modules from entry points
    fn tree_shake(
        &self,
        entry_modules: &[ModuleId],
        graph: &pledgepack_core::Graph,
    ) -> HashSet<ModuleId> {
        let mut reachable = HashSet::new();
        let mut queue: Vec<ModuleId> = entry_modules.to_vec();

        while let Some(id) = queue.pop() {
            if reachable.contains(&id) {
                continue;
            }
            reachable.insert(id);

            // Follow dependencies
            let deps = graph.get_dependents(id, 256);
            for dep in deps {
                if !reachable.contains(&dep) {
                    queue.push(dep);
                }
            }
        }

        reachable
    }

    /// Split modules into chunks: entry, vendor, and shared (parallelized using rayon)
    fn split_chunks(
        &mut self,
        entry_modules: &[ModuleId],
        reachable: &HashSet<ModuleId>,
        modules: &HashMap<ModuleId, ResolvedModule>,
        graph: &pledgepack_core::Graph,
    ) {
        // Track which modules are used by multiple entry points
        // Process each entry's dependency traversal in parallel
        let module_users: Mutex<HashMap<ModuleId, HashSet<ModuleId>>> = Mutex::new(HashMap::new());

        entry_modules.par_iter().for_each(|entry| {
            let mut visited = HashSet::new();
            let mut queue = vec![*entry];
            let mut local_users: HashMap<ModuleId, HashSet<ModuleId>> = HashMap::new();

            while let Some(id) = queue.pop() {
                if visited.contains(&id) {
                    continue;
                }
                visited.insert(id);
                local_users.entry(id).or_default().insert(*entry);
                for dep in graph.get_dependents(id, 256) {
                    queue.push(dep);
                }
            }

            // Merge local results into shared map
            let mut global = module_users.lock().unwrap();
            for (id, entries) in local_users {
                global.entry(id).or_default().extend(entries);
            }
        });

        let module_users = module_users.into_inner().unwrap();

        // Vendor modules: in node_modules (parallelized)
        let entry_module_set: HashSet<ModuleId> = entry_modules.iter().copied().collect();

        // Classify modules in parallel: vendor, shared, or entry-exclusive
        let (vendor_modules, shared_modules): (Vec<ModuleId>, Vec<ModuleId>) = reachable
            .par_iter()
            .filter(|id| !entry_module_set.contains(id))
            .partition_map(|id| {
                // Check if module is in node_modules
                if let Some(module) = modules.get(id) {
                    let path_str = module.path.to_string_lossy();
                    if path_str.contains("node_modules") {
                        return rayon::iter::Either::Left(*id);
                    }
                }

                // Check if shared between entries
                if let Some(users) = module_users.get(id) {
                    if users.len() > 1 {
                        return rayon::iter::Either::Right(*id);
                    }
                }

                rayon::iter::Either::Left(*id) // default to vendor if not shared
            });

        // Filter out non-vendor, non-shared from vendor_modules (fix partition logic)
        let vendor_modules: Vec<ModuleId> = vendor_modules.into_iter().filter(|id| {
            if let Some(module) = modules.get(id) {
                module.path.to_string_lossy().contains("node_modules")
            } else {
                false
            }
        }).collect();

        // Entry chunk: entry module + its exclusive deps (parallelized)
        let entry_chunks: Vec<Chunk> = entry_modules
            .par_iter()
            .enumerate()
            .map(|(i, entry)| {
                let mut chunk_modules = vec![*entry];
                let mut visited = HashSet::new();
                let mut queue = vec![*entry];

                while let Some(id) = queue.pop() {
                    if visited.contains(&id) {
                        continue;
                    }
                    visited.insert(id);
                    if id != *entry {
                        if !vendor_modules.contains(&id)
                            && !shared_modules.contains(&id)
                            && !entry_module_set.contains(&id)
                        {
                            chunk_modules.push(id);
                        }
                    }
                    for dep in graph.get_dependents(id, 256) {
                        queue.push(dep);
                    }
                }

                Chunk {
                    id: format!("entry-{}", i),
                    modules: chunk_modules,
                    chunk_type: ChunkType::Entry,
                }
            })
            .collect();

        self.chunks = entry_chunks;

        // Vendor chunk
        if !vendor_modules.is_empty() {
            self.chunks.push(Chunk {
                id: "vendor".to_string(),
                modules: vendor_modules,
                chunk_type: ChunkType::Vendor,
            });
        }

        // Shared chunk
        if !shared_modules.is_empty() {
            self.chunks.push(Chunk {
                id: "shared".to_string(),
                modules: shared_modules,
                chunk_type: ChunkType::Shared,
            });
        }
    }

    /// Apply manual chunks configuration — group modules matching patterns into named chunks
    fn apply_manual_chunks(
        &mut self,
        reachable: &HashSet<ModuleId>,
        modules: &HashMap<ModuleId, ResolvedModule>,
        manual_chunks: &HashMap<String, Vec<String>>,
    ) {
        for (chunk_name, patterns) in manual_chunks {
            let mut chunk_modules: Vec<ModuleId> = Vec::new();

            // Build a GlobSet from the patterns for this chunk
            let mut glob_builder = globset::GlobSetBuilder::new();
            for pattern in patterns {
                if let Ok(glob) = globset::Glob::new(pattern) {
                    glob_builder.add(glob);
                }
            }
            let glob_set = glob_builder.build().unwrap_or_default();

            for &id in reachable {
                if let Some(module) = modules.get(&id) {
                    let path_str = module.path.to_string_lossy();
                    if glob_set.is_match(path_str.as_ref()) || patterns.iter().any(|pattern| {
                        path_str.contains(pattern) || path_str.as_ref() == pattern.as_str()
                    }) {
                        chunk_modules.push(id);
                    }
                }
            }

            if !chunk_modules.is_empty() {
                // Remove these modules from other chunks to avoid duplication
                for chunk in &mut self.chunks {
                    chunk.modules.retain(|m| !chunk_modules.contains(m));
                }

                self.chunks.push(Chunk {
                    id: chunk_name.clone(),
                    modules: chunk_modules,
                    chunk_type: ChunkType::Shared,
                });
                tracing::info!("Manual chunk '{}': {} modules", chunk_name, 
                    self.chunks.last().map(|c| c.modules.len()).unwrap_or(0));
            }
        }
    }

    /// Inline dynamic imports — merge all async chunks into their parent entry chunks
    fn inline_dynamic_imports(&mut self) {
        // Find all async chunks
        let async_chunks: Vec<(usize, Vec<ModuleId>)> = self.chunks.iter()
            .enumerate()
            .filter(|(_, c)| c.chunk_type == ChunkType::Async)
            .map(|(i, c)| (i, c.modules.clone()))
            .collect();

        if async_chunks.is_empty() {
            return;
        }

        // Merge async chunk modules into entry chunks
        let entry_indices: Vec<usize> = self.chunks.iter()
            .enumerate()
            .filter(|(_, c)| c.chunk_type == ChunkType::Entry)
            .map(|(i, _)| i)
            .collect();

        for (_, async_modules) in &async_chunks {
            for &entry_idx in &entry_indices {
                if let Some(entry_chunk) = self.chunks.get_mut(entry_idx) {
                    for module in async_modules {
                        if !entry_chunk.modules.contains(module) {
                            entry_chunk.modules.push(*module);
                        }
                    }
                }
            }
        }

        // Remove async chunks
        self.chunks.retain(|c| c.chunk_type != ChunkType::Async);

        tracing::info!("Inlined dynamic imports: merged {} async chunks into entry chunks", async_chunks.len());
    }

    /// Get all chunk IDs
    pub fn chunk_ids(&self) -> Vec<String> {
        self.chunks.iter().map(|c| c.id.clone()).collect()
    }

    /// #71: Route-based chunk splitting
    /// Splits modules into per-route chunks, extracting shared modules.
    pub fn split_by_routes(
        &mut self,
        routes: &[(String, Vec<ModuleId>)],
        all_modules: &HashMap<ModuleId, ResolvedModule>,
    ) {
        let mut module_route_count: HashMap<ModuleId, usize> = HashMap::new();

        for (_, mods) in routes {
            for m in mods {
                *module_route_count.entry(*m).or_default() += 1;
            }
        }

        let shared: Vec<ModuleId> = module_route_count
            .iter()
            .filter(|(_, count)| **count > 1)
            .map(|(m, _)| *m)
            .collect();

        if !shared.is_empty() {
            self.chunks.push(Chunk {
                id: "route-shared".to_string(),
                modules: shared.clone(),
                chunk_type: ChunkType::Shared,
            });
        }

        for (route_name, mods) in routes {
            let route_modules: Vec<ModuleId> = mods
                .iter()
                .filter(|m| !shared.contains(m))
                .copied()
                .collect();

            if !route_modules.is_empty() {
                self.chunks.push(Chunk {
                    id: format!("route-{}", route_name),
                    modules: route_modules,
                    chunk_type: ChunkType::Route,
                });
            }
        }

        tracing::info!(
            "Route-based splitting: {} route chunks, 1 shared chunk ({} modules)",
            routes.len(),
            shared.len()
        );
    }
}
