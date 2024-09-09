use crate::utils;
use futures::executor::block_on;
use std::collections::HashMap;
use std::slice::Iter;
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

    pub fn keys_iter(&self) -> Iter<K> {
        self.keys.iter()
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

pub struct RpcProviderBackend {
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

pub struct RpcProviderBackendGroup {
    backends: IndexedMap<Url, RpcProviderBackend>,
    proxy_retry_timeout: Duration,
    chain_id: u64,
}

impl RpcProviderBackendGroup {
    // Create a new ProviderGroup from a list of URLs
    pub fn new(rpc_urls: &Vec<Url>, proxy_retry_timeout: Duration) -> Self {
        // queyr chain_id and validate all urls refer to the same chain
        let mut chain_id: u64 = 0;
        rpc_urls.iter().for_each(|rpc_url| {
            let next_chain_id = block_on(utils::get_chain_id(
                &reqwest::Client::new(),
                rpc_url.as_str(),
            ))
            .expect(format!("failed to get chain id: {}", rpc_url).as_str());
            if chain_id == 0 {
                chain_id = next_chain_id;
            } else if chain_id != next_chain_id {
                panic!(
                    "RPC {} has chain_id {}, but previous chain_id is {}!",
                    rpc_url, next_chain_id, chain_id
                );
            }
        });

        let backends = rpc_urls
            .iter()
            .map(|rpc_url| (rpc_url.clone(), RpcProviderBackend { retry_ttl: None }))
            .collect();

        Self {
            backends,
            proxy_retry_timeout,
            chain_id,
        }
    }

    // Get the next provider URL
    pub fn next_provider(&mut self) -> Option<Url> {
        for _ in 0..self.backends.len() {
            match self.backends.next() {
                Some((url, backend)) => {
                    if backend.is_active(self.proxy_retry_timeout) {
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

    pub fn set_inactive(&mut self, url: &Url) {
        self.backends.insert(
            url.clone(),
            RpcProviderBackend {
                retry_ttl: Some(Instant::now()),
            },
        );
    }

    pub fn set_active(&mut self, url: &Url) {
        self.backends
            .insert(url.clone(), RpcProviderBackend { retry_ttl: None });
    }

    pub fn get_chain_id(&self) -> u64 {
        self.chain_id
    }
}
