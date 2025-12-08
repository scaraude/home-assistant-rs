use hyper::body::Bytes;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Cache key based on request path and query string
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    path: String,
    query: String,
}

impl CacheKey {
    fn new(path: impl Into<String>, query: Option<&str>) -> Self {
        Self {
            path: path.into(),
            query: query.unwrap_or("").to_string(),
        }
    }
}

/// Cached HTTP response with metadata
#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub body: Bytes,
    pub etag: String,
    timestamp: SystemTime,
}

/// Generic HTTP response cache with TTL-based invalidation
pub struct ResponseCache {
    cache: Arc<Mutex<HashMap<CacheKey, CachedResponse>>>,
    ttl: Duration,
}

impl ResponseCache {
    /// Create a new response cache with specified TTL in seconds
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get cached response if valid
    pub fn get(&self, path: &str, query: Option<&str>) -> Option<CachedResponse> {
        let key = CacheKey::new(path, query);
        let cache = self.cache.lock().unwrap();

        if let Some(entry) = cache.get(&key) {
            // Check if entry is still valid (not expired)
            if entry.timestamp.elapsed().unwrap_or(self.ttl) < self.ttl {
                return Some(entry.clone());
            }
        }

        None
    }

    /// Store response in cache with auto-generated ETag
    pub fn put(&self, path: &str, query: Option<&str>, body: Bytes) -> String {
        let key = CacheKey::new(path, query);
        let etag = Self::generate_etag(&body);

        let entry = CachedResponse {
            body,
            etag: etag.clone(),
            timestamp: SystemTime::now(),
        };

        let mut cache = self.cache.lock().unwrap();
        cache.insert(key, entry);

        etag
    }

    /// Generate ETag from response body using hash
    fn generate_etag(body: &Bytes) -> String {
        let mut hasher = DefaultHasher::new();
        body.hash(&mut hasher);
        format!("\"{}\"", hasher.finish())
    }

    /// Clear all cached entries (useful for testing)
    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// Get cache statistics (useful for monitoring)
    #[allow(dead_code)]
    pub fn stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        let total_entries = cache.len();
        let valid_entries = cache
            .values()
            .filter(|entry| entry.timestamp.elapsed().unwrap_or(self.ttl) < self.ttl)
            .count();

        (total_entries, valid_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_put_and_get() {
        let cache = ResponseCache::new(60);
        let body = Bytes::from("test response");

        let etag = cache.put("/api/test", None, body.clone());
        assert!(!etag.is_empty());

        let cached = cache.get("/api/test", None);
        assert!(cached.is_some());

        let cached = cached.unwrap();
        assert_eq!(cached.body, body);
        assert_eq!(cached.etag, etag);
    }

    #[test]
    fn test_cache_key_with_query() {
        let cache = ResponseCache::new(60);

        cache.put("/api/test", Some("foo=bar"), Bytes::from("response1"));
        cache.put("/api/test", Some("foo=baz"), Bytes::from("response2"));

        let cached1 = cache.get("/api/test", Some("foo=bar")).unwrap();
        let cached2 = cache.get("/api/test", Some("foo=baz")).unwrap();

        assert_eq!(cached1.body, Bytes::from("response1"));
        assert_eq!(cached2.body, Bytes::from("response2"));
    }

    #[test]
    fn test_cache_miss() {
        let cache = ResponseCache::new(60);
        let cached = cache.get("/api/nonexistent", None);
        assert!(cached.is_none());
    }

    #[test]
    fn test_etag_generation() {
        let body1 = Bytes::from("same content");
        let body2 = Bytes::from("same content");
        let body3 = Bytes::from("different");

        let etag1 = ResponseCache::generate_etag(&body1);
        let etag2 = ResponseCache::generate_etag(&body2);
        let etag3 = ResponseCache::generate_etag(&body3);

        assert_eq!(etag1, etag2);
        assert_ne!(etag1, etag3);
    }
}
