use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde_json::Value;

pub struct ApiCache {
    entries: HashMap<String, CacheEntry>,
    max_size: usize,
    default_ttl: Duration,
}

struct CacheEntry {
    data: Value,
    expires_at: Instant,
}

impl ApiCache {
    pub fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
            default_ttl,
        }
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.entries.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.data.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&mut self, key: String, data: Value) {
        if self.entries.len() >= self.max_size {
            self.evict_expired();
        }
        self.entries.insert(key, CacheEntry {
            data,
            expires_at: Instant::now() + self.default_ttl,
        });
    }

    pub fn invalidate(&mut self, pattern: &str) {
        self.entries.retain(|key, _| !key.contains(pattern));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn evict_expired(&mut self) {
        let now = Instant::now();
        self.entries.retain(|_, entry| entry.expires_at > now);
        if self.entries.is_empty() {
            self.entries.shrink_to_fit();
        }
    }
}
