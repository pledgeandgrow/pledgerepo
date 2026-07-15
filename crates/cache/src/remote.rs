// Remote cache backend for sharing transform cache across machines.
//
// Supports content-addressable storage via:
//   - HTTP REST API (generic, self-hosted)
//   - S3-compatible storage (AWS S3, MinIO, Cloudflare R2, etc.)
//   - GCS (Google Cloud Storage)
//
// The remote cache is used as a fallback when the local disk cache
// misses. This enables CI builds to share cache across runs and
// team members to share cache across machines.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Configuration for the remote cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCacheConfig {
    /// Backend type: "http", "s3", "gcs"
    pub backend: String,
    /// URL/endpoint (e.g., "https://cache.example.com" or "https://s3.amazonaws.com")
    pub endpoint: String,
    /// Bucket name (for S3/GCS)
    pub bucket: Option<String>,
    /// Region (for S3)
    pub region: Option<String>,
    /// Access key (for S3/GCS)
    pub access_key: Option<String>,
    /// Secret key (for S3/GCS)
    pub secret_key: Option<String>,
    /// Namespace prefix for cache keys (e.g., "myproject/")
    pub namespace: Option<String>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Whether remote cache is enabled
    pub enabled: bool,
}

impl Default for RemoteCacheConfig {
    fn default() -> Self {
        Self {
            backend: "http".to_string(),
            endpoint: String::new(),
            bucket: None,
            region: None,
            access_key: None,
            secret_key: None,
            namespace: None,
            timeout_secs: 30,
            enabled: false,
        }
    }
}

/// A cached entry stored remotely (same structure as local CacheEntry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCacheEntry {
    pub code: String,
    pub source_map: Option<String>,
    pub deps: Vec<String>,
    pub created_at: u64,
}

/// Remote cache client — abstracts over HTTP/S3/GCS backends
pub struct RemoteCache {
    config: RemoteCacheConfig,
    enabled: bool,
}

impl RemoteCache {
    pub fn new(config: RemoteCacheConfig) -> Self {
        let enabled = config.enabled && !config.endpoint.is_empty();
        Self { config, enabled }
    }

    /// Check if remote cache is active
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get a cache entry from the remote backend
    pub fn get(&self, key: &str) -> Result<Option<RemoteCacheEntry>> {
        if !self.enabled {
            return Ok(None);
        }

        match self.config.backend.as_str() {
            "http" => self.http_get(key),
            "s3" => self.s3_get(key),
            "gcs" => self.gcs_get(key),
            _ => bail!("Unknown remote cache backend: {}", self.config.backend),
        }
    }

    /// Store a cache entry in the remote backend
    pub fn set(&self, key: &str, entry: &RemoteCacheEntry) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        match self.config.backend.as_str() {
            "http" => self.http_set(key, entry),
            "s3" => self.s3_set(key, entry),
            "gcs" => self.gcs_set(key, entry),
            _ => bail!("Unknown remote cache backend: {}", self.config.backend),
        }
    }

    fn build_url(&self, key: &str) -> String {
        let endpoint = self.config.endpoint.trim_end_matches('/');
        let ns = self.config.namespace.as_deref().map(|n| n.trim_start_matches('/')).unwrap_or("");
        if ns.is_empty() {
            format!("{}/cache/{}", endpoint, key)
        } else {
            format!("{}/cache/{}/{}", endpoint, ns, key)
        }
    }

    fn http_get(&self, key: &str) -> Result<Option<RemoteCacheEntry>> {
        let url = self.build_url(key);
        debug!("Remote cache GET: {}", url);

        let output = std::process::Command::new("curl")
            .args(["-s", "-f", "--max-time", &self.config.timeout_secs.to_string(), &url])
            .output();

        match output {
            Ok(result) if result.status.success() && !result.stdout.is_empty() => {
                match bincode::deserialize::<RemoteCacheEntry>(&result.stdout) {
                    Ok(entry) => {
                        info!("Remote cache hit: {}", key);
                        Ok(Some(entry))
                    }
                    Err(e) => {
                        warn!("Remote cache deserialization failed: {}", e);
                        Ok(None)
                    }
                }
            }
            _ => {
                debug!("Remote cache miss: {}", key);
                Ok(None)
            }
        }
    }

    fn http_set(&self, key: &str, entry: &RemoteCacheEntry) -> Result<()> {
        let url = self.build_url(key);
        let data = bincode::serialize(entry)?;

        let temp_file = std::env::temp_dir().join(format!(
            "pledgepack_remote_{}",
            blake3::hash(&data).to_hex()
        ));
        std::fs::write(&temp_file, &data)?;

        let output = std::process::Command::new("curl")
            .args([
                "-s", "-f", "--max-time", &self.config.timeout_secs.to_string(),
                "-X", "PUT",
                "-H", "Content-Type: application/octet-stream",
                "--data-binary", &format!("@{}", temp_file.to_string_lossy()),
                &url,
            ])
            .output();

        let _ = std::fs::remove_file(&temp_file);

        match output {
            Ok(result) if result.status.success() => {
                debug!("Remote cache stored: {}", key);
                Ok(())
            }
            _ => {
                warn!("Remote cache store failed: {}", key);
                Ok(())
            }
        }
    }

    fn s3_get(&self, key: &str) -> Result<Option<RemoteCacheEntry>> {
        let bucket = self.config.bucket.as_deref().unwrap_or("pledgepack-cache");
        let region = self.config.region.as_deref().unwrap_or("us-east-1");
        let ns = self.config.namespace.as_deref().unwrap_or("");
        let object_key = if ns.is_empty() { key.to_string() } else { format!("{}/{}", ns, key) };

        let output = std::process::Command::new("aws")
            .args(["s3", "cp", &format!("s3://{}/{}", bucket, object_key), "-", "--region", region])
            .output();

        match output {
            Ok(result) if result.status.success() && !result.stdout.is_empty() => {
                match bincode::deserialize::<RemoteCacheEntry>(&result.stdout) {
                    Ok(entry) => {
                        info!("S3 cache hit: {}/{}", bucket, object_key);
                        Ok(Some(entry))
                    }
                    Err(e) => {
                        warn!("S3 cache deserialization failed: {}", e);
                        Ok(None)
                    }
                }
            }
            _ => {
                debug!("S3 cache miss: {}/{}", bucket, object_key);
                Ok(None)
            }
        }
    }

    fn s3_set(&self, key: &str, entry: &RemoteCacheEntry) -> Result<()> {
        let bucket = self.config.bucket.as_deref().unwrap_or("pledgepack-cache");
        let region = self.config.region.as_deref().unwrap_or("us-east-1");
        let ns = self.config.namespace.as_deref().unwrap_or("");
        let object_key = if ns.is_empty() { key.to_string() } else { format!("{}/{}", ns, key) };

        let data = bincode::serialize(entry)?;
        let temp_file = std::env::temp_dir().join(format!("pledgepack_s3_{}", blake3::hash(&data).to_hex()));
        std::fs::write(&temp_file, &data)?;

        let output = std::process::Command::new("aws")
            .args(["s3", "cp", &temp_file.to_string_lossy(), &format!("s3://{}/{}", bucket, object_key), "--region", region])
            .output();

        let _ = std::fs::remove_file(&temp_file);

        match output {
            Ok(result) if result.status.success() => {
                debug!("S3 cache stored: {}/{}", bucket, object_key);
                Ok(())
            }
            _ => {
                warn!("S3 cache store failed: {}/{}", bucket, object_key);
                Ok(())
            }
        }
    }

    fn gcs_get(&self, key: &str) -> Result<Option<RemoteCacheEntry>> {
        let bucket = self.config.bucket.as_deref().unwrap_or("pledgepack-cache");
        let ns = self.config.namespace.as_deref().unwrap_or("");
        let object_key = if ns.is_empty() { key.to_string() } else { format!("{}/{}", ns, key) };

        let output = std::process::Command::new("gsutil")
            .args(["cp", &format!("gs://{}/{}", bucket, object_key), "-"])
            .output();

        match output {
            Ok(result) if result.status.success() && !result.stdout.is_empty() => {
                match bincode::deserialize::<RemoteCacheEntry>(&result.stdout) {
                    Ok(entry) => {
                        info!("GCS cache hit: {}/{}", bucket, object_key);
                        Ok(Some(entry))
                    }
                    Err(e) => {
                        warn!("GCS cache deserialization failed: {}", e);
                        Ok(None)
                    }
                }
            }
            _ => {
                debug!("GCS cache miss: {}/{}", bucket, object_key);
                Ok(None)
            }
        }
    }

    fn gcs_set(&self, key: &str, entry: &RemoteCacheEntry) -> Result<()> {
        let bucket = self.config.bucket.as_deref().unwrap_or("pledgepack-cache");
        let ns = self.config.namespace.as_deref().unwrap_or("");
        let object_key = if ns.is_empty() { key.to_string() } else { format!("{}/{}", ns, key) };

        let data = bincode::serialize(entry)?;
        let temp_file = std::env::temp_dir().join(format!("pledgepack_gcs_{}", blake3::hash(&data).to_hex()));
        std::fs::write(&temp_file, &data)?;

        let output = std::process::Command::new("gsutil")
            .args(["cp", &temp_file.to_string_lossy(), &format!("gs://{}/{}", bucket, object_key)])
            .output();

        let _ = std::fs::remove_file(&temp_file);

        match output {
            Ok(result) if result.status.success() => {
                debug!("GCS cache stored: {}/{}", bucket, object_key);
                Ok(())
            }
            _ => {
                warn!("GCS cache store failed: {}/{}", bucket, object_key);
                Ok(())
            }
        }
    }
}

/// Build a remote cache key from a content hash and function ID
pub fn remote_cache_key(content_hash: u64, function_id: &str, path: &str) -> String {
    let combined = format!("{}:{}:{}", content_hash, function_id, path);
    let hash = blake3::hash(combined.as_bytes());
    hash.to_hex().as_str().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_cache_key_deterministic() {
        let key1 = remote_cache_key(123, "transform", "/src/a.ts");
        let key2 = remote_cache_key(123, "transform", "/src/a.ts");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_remote_cache_key_different_inputs() {
        let key1 = remote_cache_key(123, "transform", "/src/a.ts");
        let key2 = remote_cache_key(456, "transform", "/src/a.ts");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_disabled_remote_cache() {
        let config = RemoteCacheConfig::default();
        let cache = RemoteCache::new(config);
        assert!(!cache.is_enabled());
    }
}
