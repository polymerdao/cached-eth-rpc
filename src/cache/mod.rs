pub mod lru_backend;
pub mod memory_backend;
pub mod redis_backend;

use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::min;
use tracing::info;

pub enum CacheStatus {
    Cached { key: String, value: CacheValue },
    Missed { key: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CacheValue {
    pub data: Value,
    reorg_ttl: u32,
    ttl: u32,
    last_modified: i64,
}

impl CacheValue {
    pub fn new(data: Value, reorg_ttl: u32, ttl: u32) -> Self {
        let last_modified = Local::now().timestamp();
        Self {
            data,
            reorg_ttl,
            ttl,
            last_modified,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = Local::now().timestamp();

        let last_modified = self.last_modified;
        let reorg_ttl = self.reorg_ttl;
        let ttl = self.ttl;

        if last_modified > now {
            return true;
        }

        let age: u64 = (now - last_modified) as u64;
        let ttl = if reorg_ttl == 0 && ttl == 0 {
            0
        } else if reorg_ttl == 0 && ttl > 0 {
            ttl
        } else if reorg_ttl > 0 && ttl == 0 {
            reorg_ttl
        } else {
            min(reorg_ttl, ttl)
        };

        ttl != 0 && age > ttl.into()
    }

    pub fn update(mut self, expired_value: &Option<Self>, reorg_ttl: u32) -> Self {
        // if a previous entry existed then check if the response has changed
        // else this is a new entry and nothing to do
        if let Some(expired_value) = expired_value {
            let is_new = expired_value.data != self.data;
            self.last_modified = Local::now().timestamp();

            // if the value has changed then reset the reorg ttl
            // else we can exponentially backoff the reorg_ttl
            // but only exponential backoff if we hit the reorg_ttl
            // and not the rpc ttl
            self.reorg_ttl = if is_new {
                reorg_ttl
            } else {
                let now = Local::now().timestamp();
                let age: u64 = (now - expired_value.last_modified) as u64;
                if age > expired_value.reorg_ttl as u64 {
                    expired_value.reorg_ttl * 2
                } else {
                    reorg_ttl
                }
            };
            info!("self.reorg_ttl: {}", self.reorg_ttl)
        } else {
            self.reorg_ttl = reorg_ttl
        }

        self
    }

    pub fn to_string(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&self)?)
    }

    pub fn from_str(value: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(value)?)
    }
}

pub trait CacheBackendFactory: Send + Sync {
    fn get_instance(&self) -> anyhow::Result<Box<dyn CacheBackend>>;
}

pub trait CacheBackend {
    fn read(&mut self, method: &str, params_key: &str) -> anyhow::Result<CacheStatus>;
    fn write(
        &mut self,
        key: &str,
        cache_value: CacheValue,
        expired_value: &Option<CacheValue>,
    ) -> anyhow::Result<()>;
}
