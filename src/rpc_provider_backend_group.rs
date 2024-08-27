use std::collections::HashMap;
use std::time::{Duration, Instant};
use url::Url;

struct IndexedMap<K, V> {
    map: HashMap<K, V>,
    keys: Vec<K>,
    current_index: usize,
}

impl<K, V> IndexedMap<K, V>
where
    K: std::hash::Hash + Eq + Clone,
{
    // Create a new IndexedMap
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            keys: Vec::new(),
            current_index: 0,
        }
    }

    // Insert a key-value pair
    fn insert(&mut self, key: K, value: V) {
        // If the key is new, add it to the keys vector
        if !self.map.contains_key(&key) {
            self.keys.push(key.clone());
        }
        self.map.insert(key, value);
    }

    // Get a value by key
    fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    // Get the next value by index, looping around at the end
    fn next(&mut self) -> Option<(&K, &V)> {
        if self.keys.is_empty() {
            return None;
        }

        let key = &self.keys[self.current_index];
        self.current_index = (self.current_index + 1) % self.keys.len();

        self.map.get_key_value(key)
    }

    fn len(&self) -> usize {
        self.keys.len()
    }
    // Reset the index
    fn reset(&mut self) {
        self.current_index = 0;
    }
}

struct RpcProviderBackend {
    retry_ttl: Option<Instant>,
}

impl RpcProviderBackend {
    pub fn is_inactive(&self, retry_timeout: Duration) -> bool {
        match self.retry_ttl {
            Some(retry_ttl) => retry_ttl.elapsed().gt(&retry_timeout),
            None => false,
        }
    }

    pub fn is_active(&self, retry_timeout: Duration) -> bool {
        !self.is_inactive(retry_timeout)
    }

    pub fn set_inactive(&mut self) {
        self.retry_ttl = Some(Instant::now());
    }

    pub fn set_active(&mut self) {
        self.retry_ttl = None;
    }
}

// Implement FromIterator to construct an IndexedMap from an iterator of (Url, RpcProviderBackend)
impl FromIterator<(Url, RpcProviderBackend)> for IndexedMap<Url, RpcProviderBackend> {
    fn from_iter<I: IntoIterator<Item = (Url, RpcProviderBackend)>>(iter: I) -> Self {
        let mut indexed_map = IndexedMap::new();

        for (url, backend) in iter {
            indexed_map.insert(url, backend);
        }

        indexed_map
    }
}

struct RpcProviderBackendGroup {
    backends: IndexedMap<Url, RpcProviderBackend>,
    retry_timeout: Duration,
}

impl RpcProviderBackendGroup {
    // Create a new ProviderGroup from a list of URLs
    fn new(urls: Vec<Url>, retry_timeout: Duration) -> Self {
        let backends = urls
            .iter()
            .map(|url| (url.clone(), RpcProviderBackend { retry_ttl: None }))
            .collect();

        Self {
            backends,
            retry_timeout,
        }
    }

    // Get the next provider URL
    fn next_provider(&mut self) -> Option<Url> {
        for _ in 0..self.backends.len() {
            match self.backends.next() {
                Some((url, backend)) => {
                    if backend.is_active(self.retry_timeout) {
                        return Some(url.clone());
                    } else {
                        continue;
                    }
                }
                None => {}
            }
        }
        None
    }

    fn set_inactive(&mut self, url: &Url) {
        self.backends.insert(
            url.clone(),
            RpcProviderBackend {
                retry_ttl: Some(Instant::now()),
            },
        );
    }

    fn set_active(&mut self, url: &Url) {
        self.backends
            .insert(url.clone(), RpcProviderBackend { retry_ttl: None });
    }
}
