use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use serde_json::from_str;

use super::{CacheBackend, CacheBackendFactory, CacheStatus, CacheValue};

pub struct LruBackendFactory {
    data: Arc<Mutex<LruCache<String, String>>>,
    reorg_ttl: u32,
}

impl LruBackendFactory {
    pub fn new(cap: usize, reorg_ttl: u32) -> Self {
        Self {
            data: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(cap).unwrap()))),
            reorg_ttl,
        }
    }
}

impl CacheBackendFactory for LruBackendFactory {
    fn get_instance(&self) -> anyhow::Result<Box<dyn CacheBackend>> {
        Ok(Box::new(LruBackend {
            data: self.data.clone(),
            reorg_ttl: self.reorg_ttl,
        }))
    }
}

pub struct LruBackend {
    data: Arc<Mutex<LruCache<String, String>>>,
    reorg_ttl: u32,
}

impl CacheBackend for LruBackend {
    fn get_reorg_ttl(&self) -> u32 {
        self.reorg_ttl
    }

    fn read(&mut self, method: &str, params_key: &str) -> anyhow::Result<CacheStatus> {
        let key = format!("{method}:{params_key}");

        let mut lru_cache = self.data.lock().unwrap();
        let v = match lru_cache.get(&key) {
            Some(value) => {
                let value =
                    from_str::<CacheValue>(value).context("fail to deserialize cache value")?;
                CacheStatus::Cached { key, value }
            }

            None => CacheStatus::Missed { key },
        };

        Ok(v)
    }

    fn write(
        &mut self,
        key: &str,
        cache_value: CacheValue,
        expired_value: &Option<CacheValue>,
    ) -> anyhow::Result<()> {
        let mut lru_cache = self.data.lock().unwrap();
        let cache_value = cache_value.update(expired_value, self.reorg_ttl);
        let _ = lru_cache.put(key.to_string(), cache_value.to_string()?);
        Ok(())
    }
}
