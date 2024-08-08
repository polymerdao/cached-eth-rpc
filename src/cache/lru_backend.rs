use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use serde_json::{from_str, Value};

use super::{CacheBackend, CacheBackendFactory, CacheStatus};

pub struct LruBackendFactory {
    data: Arc<Mutex<LruCache<String, String>>>,
}

impl LruBackendFactory {
    pub fn new(cap: usize) -> Self {
        Self {
            data: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(cap).unwrap()))),
        }
    }
}

impl CacheBackendFactory for LruBackendFactory {
    fn get_instance(&self) -> anyhow::Result<Box<dyn CacheBackend>> {
        Ok(Box::new(LruBackend {
            data: self.data.clone(),
        }))
    }
}

pub struct LruBackend {
    data: Arc<Mutex<LruCache<String, String>>>,
}

impl CacheBackend for LruBackend {
    fn read(&mut self, method: &str, params_key: &str) -> anyhow::Result<CacheStatus> {
        let key = format!("{method}:{params_key}");

        let mut lru_cache = self.data.lock().unwrap();
        let v = match lru_cache.get(&key) {
            Some(value) => {
                let value = from_str::<Value>(&value).context("fail to deserialize cache value")?;

                CacheStatus::Cached { key, value }
            }

            None => CacheStatus::Missed { key },
        };

        Ok(v)
    }

    fn write(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        let mut lru_cache = self.data.lock().unwrap();
        let _ = lru_cache.put(key.to_string(), value.to_string());
        Ok(())
    }
}
