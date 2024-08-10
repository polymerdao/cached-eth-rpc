use std::sync::Arc;

use anyhow::Context;
use dashmap::DashMap;
use serde_json::from_str;

use super::{CacheBackend, CacheBackendFactory, CacheStatus, CacheValue};

pub struct MemoryBackendFactory {
    data: Arc<DashMap<String, String>>,
    reorg_ttl: u32,
}

impl MemoryBackendFactory {
    pub fn new(reorg_ttl: u32) -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            reorg_ttl,
        }
    }
}

impl CacheBackendFactory for MemoryBackendFactory {
    fn get_instance(&self) -> anyhow::Result<Box<dyn CacheBackend>> {
        Ok(Box::new(MemoryBackend {
            data: self.data.clone(),
            reorg_ttl: self.reorg_ttl,
        }))
    }
}

pub struct MemoryBackend {
    data: Arc<DashMap<String, String>>,
    reorg_ttl: u32,
}

impl CacheBackend for MemoryBackend {
    fn get_reorg_ttl(&self) -> u32 {
        self.reorg_ttl
    }

    fn read(&mut self, method: &str, params_key: &str) -> anyhow::Result<CacheStatus> {
        let key = format!("{method}:{params_key}");

        let v = match self.data.get(&key) {
            Some(value) => {
                let value =
                    from_str::<CacheValue>(&value).context("fail to deserialize cache value")?;

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
        let cache_value = cache_value.update(expired_value, self.reorg_ttl);
        let _ = self.data.insert(key.to_string(), cache_value.to_string()?);
        Ok(())
    }
}
