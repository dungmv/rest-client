use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::Arc;
use crate::models::HttpResponseRecord;

/// Trait to report the estimated memory footprint in bytes of an item stored in cache.
pub trait ByteSized {
    fn byte_size(&self) -> usize {
        0
    }
}

impl<T: ByteSized> ByteSized for Arc<T> {
    fn byte_size(&self) -> usize {
        (**self).byte_size()
    }
}

impl ByteSized for HttpResponseRecord {
    fn byte_size(&self) -> usize {
        self.body.len()
    }
}

impl ByteSized for String {
    fn byte_size(&self) -> usize {
        self.len()
    }
}

impl ByteSized for Vec<u8> {
    fn byte_size(&self) -> usize {
        self.len()
    }
}

/// Generic bounded in-memory cache with Least Recently Used (LRU) eviction policy
/// and dual constraints: maximum entry count cap and maximum total memory (bytes) cap.
#[derive(Debug, Clone)]
pub struct LruMemoryCache<K, V> {
    entries: HashMap<K, V>,
    order: VecDeque<K>,
    max_entries: usize,
    max_total_bytes: usize,
    current_total_bytes: usize,
}

pub const DEFAULT_MAX_CACHE_ENTRIES: usize = 100;
pub const DEFAULT_MAX_CACHE_BYTES: usize = 50 * 1024 * 1024; // 50 MB

impl<K, V> Default for LruMemoryCache<K, V>
where
    K: Hash + Eq + Clone,
    V: ByteSized,
{
    fn default() -> Self {
        Self::new(DEFAULT_MAX_CACHE_ENTRIES, DEFAULT_MAX_CACHE_BYTES)
    }
}

impl<K, V> LruMemoryCache<K, V>
where
    K: Hash + Eq + Clone,
    V: ByteSized,
{
    pub fn new(max_entries: usize, max_total_bytes: usize) -> Self {
        let max_entries = max_entries.max(1);
        Self {
            entries: HashMap::with_capacity(max_entries.min(1024)),
            order: VecDeque::with_capacity(max_entries.min(1024)),
            max_entries,
            max_total_bytes,
            current_total_bytes: 0,
        }
    }

    /// Access an entry by key, updating its LRU position to Most Recently Used (MRU).
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.entries.contains_key(key) {
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                let k = self.order.remove(pos).unwrap();
                self.order.push_back(k);
            }
            self.entries.get(key)
        } else {
            None
        }
    }

    /// Read-only inspection of an entry without modifying the LRU order.
    pub fn peek(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    /// Insert a key-value pair, marking it as MRU and evicting older LRU entries if
    /// either entry count or total byte limits are exceeded.
    /// Returns the previous value if the key was already present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let new_bytes = value.byte_size();
        let old_value = if self.entries.contains_key(&key) {
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
            let old = self.entries.insert(key.clone(), value);
            if let Some(old_ref) = &old {
                self.current_total_bytes = self.current_total_bytes.saturating_sub(old_ref.byte_size());
            }
            self.current_total_bytes = self.current_total_bytes.saturating_add(new_bytes);
            self.order.push_back(key);
            old
        } else {
            self.entries.insert(key.clone(), value);
            self.current_total_bytes = self.current_total_bytes.saturating_add(new_bytes);
            self.order.push_back(key);
            None
        };

        // Evict LRU items while bounds are exceeded, but keep at least 1 entry
        while self.entries.len() > self.max_entries
            || (self.current_total_bytes > self.max_total_bytes && self.entries.len() > 1)
        {
            if let Some(evict_key) = self.order.pop_front() {
                if let Some(evicted_val) = self.entries.remove(&evict_key) {
                    self.current_total_bytes =
                        self.current_total_bytes.saturating_sub(evicted_val.byte_size());
                }
            } else {
                break;
            }
        }

        old_value
    }

    /// Remove an entry by key, updating the order and total byte size.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(val) = self.entries.remove(key) {
            self.current_total_bytes = self.current_total_bytes.saturating_sub(val.byte_size());
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
            Some(val)
        } else {
            None
        }
    }

    /// Rename a key in-place, preserving its LRU position and byte count.
    pub fn rename(&mut self, old_key: &K, new_key: K) -> bool {
        if old_key == &new_key {
            return true;
        }
        if let Some(val) = self.entries.remove(old_key) {
            // If new_key already existed, subtract its byte size and remove from order
            if let Some(overwritten) = self.entries.insert(new_key.clone(), val) {
                self.current_total_bytes =
                    self.current_total_bytes.saturating_sub(overwritten.byte_size());
                if let Some(pos) = self.order.iter().position(|k| k == &new_key) {
                    self.order.remove(pos);
                }
            }
            if let Some(pos) = self.order.iter().position(|k| k == old_key) {
                self.order[pos] = new_key;
            } else {
                self.order.push_back(new_key);
            }
            true
        } else {
            false
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn total_bytes(&self) -> usize {
        self.current_total_bytes
    }

    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    pub fn max_total_bytes(&self) -> usize {
        self.max_total_bytes
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.current_total_bytes = 0;
    }
}

/// Specialized type alias for caching HTTP response records.
pub type ResponseCache = LruMemoryCache<String, Arc<HttpResponseRecord>>;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_record(id: &str, body_len: usize) -> Arc<HttpResponseRecord> {
        Arc::new(HttpResponseRecord {
            id: id.to_string(),
            request_id: format!("req-{}", id),
            status_code: 200,
            status_text: "OK".to_string(),
            duration_ms: 10,
            size_bytes: body_len,
            headers: vec![],
            cookies: vec![],
            body: vec![b'x'; body_len],
            timing: Default::default(),
            request_sent: None,
            test_results: vec![],
            console_logs: vec![],
            created_at: chrono::Utc::now(),
        })
    }

    #[test]
    fn test_lru_eviction_by_entry_count() {
        let mut cache: LruMemoryCache<String, String> = LruMemoryCache::new(3, 10_000);
        cache.insert("a".to_string(), "val-a".to_string());
        cache.insert("b".to_string(), "val-b".to_string());
        cache.insert("c".to_string(), "val-c".to_string());
        assert_eq!(cache.len(), 3);

        // Inserting 4th item evicts oldest ('a')
        cache.insert("d".to_string(), "val-d".to_string());
        assert_eq!(cache.len(), 3);
        assert!(!cache.contains_key(&"a".to_string()));
        assert!(cache.contains_key(&"b".to_string()));
        assert!(cache.contains_key(&"c".to_string()));
        assert!(cache.contains_key(&"d".to_string()));
    }

    #[test]
    fn test_mru_promotion_on_get() {
        let mut cache: LruMemoryCache<String, String> = LruMemoryCache::new(3, 10_000);
        cache.insert("a".to_string(), "val-a".to_string());
        cache.insert("b".to_string(), "val-b".to_string());
        cache.insert("c".to_string(), "val-c".to_string());

        // Access 'a', making it MRU
        assert_eq!(cache.get(&"a".to_string()), Some(&"val-a".to_string()));

        // Inserting 'd' should now evict 'b' (oldest), not 'a'
        cache.insert("d".to_string(), "val-d".to_string());
        assert!(cache.contains_key(&"a".to_string()));
        assert!(!cache.contains_key(&"b".to_string()));
        assert!(cache.contains_key(&"c".to_string()));
        assert!(cache.contains_key(&"d".to_string()));
    }

    #[test]
    fn test_lru_eviction_by_memory_size() {
        // Max 10 items, but only 250 bytes limit
        let mut cache = ResponseCache::new(10, 250);

        let r1 = create_test_record("1", 100);
        let r2 = create_test_record("2", 100);
        let r3 = create_test_record("3", 100);

        cache.insert("1".to_string(), r1);
        assert_eq!(cache.total_bytes(), 100);

        cache.insert("2".to_string(), r2);
        assert_eq!(cache.total_bytes(), 200);

        // Adding r3 brings total to 300, which exceeds 250 -> evicts "1"
        cache.insert("3".to_string(), r3);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.total_bytes(), 200);
        assert!(!cache.contains_key(&"1".to_string()));
        assert!(cache.contains_key(&"2".to_string()));
        assert!(cache.contains_key(&"3".to_string()));
    }

    #[test]
    fn test_single_large_item_retention() {
        // Max 50 bytes limit, but we insert a 200-byte response
        let mut cache = ResponseCache::new(5, 50);
        let big = create_test_record("big", 200);

        cache.insert("big".to_string(), big);
        // Cache must keep at least 1 entry so current request is never immediately evicted
        assert_eq!(cache.len(), 1);
        assert!(cache.contains_key(&"big".to_string()));
        assert_eq!(cache.total_bytes(), 200);
    }

    #[test]
    fn test_rename_preserves_order_and_bytes() {
        let mut cache = ResponseCache::new(3, 10_000);
        let r1 = create_test_record("1", 50);
        let r2 = create_test_record("2", 70);

        cache.insert("req-1".to_string(), r1);
        cache.insert("req-2".to_string(), r2);
        assert_eq!(cache.total_bytes(), 120);

        let ok = cache.rename(&"req-1".to_string(), "req-renamed".to_string());
        assert!(ok);
        assert!(!cache.contains_key(&"req-1".to_string()));
        assert!(cache.contains_key(&"req-renamed".to_string()));
        assert_eq!(cache.total_bytes(), 120);

        // Add 2 more items: "req-renamed" should be evicted first because it is still the oldest!
        let r3 = create_test_record("3", 10);
        let r4 = create_test_record("4", 10);
        cache.insert("req-3".to_string(), r3);
        cache.insert("req-4".to_string(), r4);

        assert!(!cache.contains_key(&"req-renamed".to_string()));
        assert!(cache.contains_key(&"req-2".to_string()));
        assert!(cache.contains_key(&"req-3".to_string()));
        assert!(cache.contains_key(&"req-4".to_string()));
    }

    #[test]
    fn test_remove_and_clear() {
        let mut cache = ResponseCache::new(5, 10_000);
        cache.insert("1".to_string(), create_test_record("1", 50));
        cache.insert("2".to_string(), create_test_record("2", 60));
        assert_eq!(cache.total_bytes(), 110);

        let removed = cache.remove(&"1".to_string());
        assert!(removed.is_some());
        assert_eq!(cache.total_bytes(), 60);
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.total_bytes(), 0);
        assert!(cache.is_empty());
    }
}
