// Function-level incremental cache
//
// This is the "Turbo engine" equivalent — caches the result of
// every function in the build pipeline. When a file changes,
// only the affected functions are re-run.
//
// Two storage tiers:
//   1. In-memory (dashmap) — fast, per-session
//   2. Filesystem (bincode) — persistent across restarts

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::debug;

/// Unique key for a cached function result
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    /// Content hash of the input file
    pub content_hash: u64,
    /// Function identifier (e.g., "swc_transform", "resolve_imports")
    pub function_id: String,
    /// Additional parameters that affect the output
    pub params_hash: u64,
}

/// Cached result of a function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub code: String,
    pub source_map: Option<String>,
    pub deps: Vec<String>,
    pub created_at: u64,
}

/// The function-level cache
pub struct FunctionCache {
    /// In-memory cache (concurrent)
    memory: Arc<DashMap<CacheKey, CacheEntry>>,
    /// Cache directory on filesystem
    cache_dir: PathBuf,
    /// Whether filesystem persistence is enabled
    persist: bool,
}

impl FunctionCache {
    pub fn new(cache_dir: PathBuf, persist: bool) -> Self {
        if persist {
            std::fs::create_dir_all(&cache_dir).ok();
        }

        Self {
            memory: Arc::new(DashMap::new()),
            cache_dir,
            persist,
        }
    }

    /// Get a cached entry
    pub fn get(&self, key: &CacheKey) -> Option<CacheEntry> {
        // Check memory first
        if let Some(entry) = self.memory.get(key) {
            debug!("Cache hit (memory): {}", key.function_id);
            return Some(entry.clone());
        }

        // Check filesystem
        if self.persist {
            if let Ok(entry) = self.read_from_disk(key) {
                debug!("Cache hit (disk): {}", key.function_id);
                // Populate memory cache
                self.memory.insert(key.clone(), entry.clone());
                return Some(entry);
            }
        }

        None
    }

    /// Insert a cached entry
    pub fn set(&self, key: CacheKey, entry: CacheEntry) {
        self.memory.insert(key.clone(), entry.clone());

        if self.persist {
            if let Err(e) = self.write_to_disk(&key, &entry) {
                tracing::warn!("Failed to persist cache entry: {}", e);
            }
        }
    }

    /// Invalidate entries for a given content hash
    pub fn invalidate_by_content(&self, content_hash: u64) {
        let keys_to_remove: Vec<CacheKey> = self
            .memory
            .iter()
            .filter(|entry| entry.key().content_hash == content_hash)
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys_to_remove {
            self.memory.remove(&key);
            if self.persist {
                let path = self.cache_path(&key);
                std::fs::remove_file(path).ok();
            }
        }
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        self.memory.clear();
        if self.persist {
            std::fs::remove_dir_all(&self.cache_dir).ok();
            std::fs::create_dir_all(&self.cache_dir).ok();
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.memory.len() as u64,
        }
    }

    fn cache_path(&self, key: &CacheKey) -> PathBuf {
        let hash = blake3::hash(bincode::serialize(key).unwrap_or_default().as_slice());
        self.cache_dir.join(hash.to_hex().as_str().to_string())
    }

    fn read_from_disk(&self, key: &CacheKey) -> Result<CacheEntry> {
        let path = self.cache_path(key);
        let data = std::fs::read(&path)?;
        let entry: CacheEntry = bincode::deserialize(&data)?;
        Ok(entry)
    }

    fn write_to_disk(&self, key: &CacheKey, entry: &CacheEntry) -> Result<()> {
        let path = self.cache_path(key);
        let data = bincode::serialize(entry)?;
        std::fs::write(path, data)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub entries: u64,
}

/// Helper to compute a cache key
pub fn make_key(content_hash: u64, function_id: &str, params: &impl serde::Serialize) -> CacheKey {
    let params_bytes = bincode::serialize(params).unwrap_or_default();
    let params_hash = u64::from_be_bytes(blake3::hash(&params_bytes).as_bytes()[0..8].try_into().unwrap());

    CacheKey {
        content_hash,
        function_id: function_id.to_string(),
        params_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_set_get() {
        let dir = std::env::temp_dir().join("pledgepack_cache_test");
        let cache = FunctionCache::new(dir.clone(), false);

        let key = make_key(123, "test_fn", &"params");
        let entry = CacheEntry {
            code: "console.log('hello')".to_string(),
            source_map: None,
            deps: vec!["./foo".to_string()],
            created_at: 0,
        };

        cache.set(key.clone(), entry.clone());
        let result = cache.get(&key).unwrap();
        assert_eq!(result.code, entry.code);
    }
}
