use anyhow::Context;
use redis::Commands;
use serde_json::from_str;

use super::{CacheBackend, CacheBackendFactory, CacheStatus, CacheValue};

pub struct RedisBackendFactory {
    chain_id: u64, // TODO, remove this
    client: r2d2::Pool<redis::Client>,
    reorg_ttl: u32,
}

impl RedisBackendFactory {
    pub fn new(chain_id: u64, client: r2d2::Pool<redis::Client>, reorg_ttl: u32) -> Self {
        Self {
            chain_id,
            client,
            reorg_ttl,
        }
    }
}

impl CacheBackendFactory for RedisBackendFactory {
    fn get_instance(&self) -> anyhow::Result<Box<dyn CacheBackend>> {
        Ok(Box::new(RedisBackend {
            chain_id: self.chain_id,
            conn: self.client.get()?,
            reorg_ttl: self.reorg_ttl,
        }))
    }
}

pub struct RedisBackend {
    chain_id: u64,
    conn: r2d2::PooledConnection<redis::Client>,
    reorg_ttl: u32,
}

impl CacheBackend for RedisBackend {
    fn get_reorg_ttl(&self) -> u32 {
        self.reorg_ttl
    }

    fn read(&mut self, method: &str, params_key: &str) -> anyhow::Result<CacheStatus> {
        let cache_key = format!("{}:{method}:{params_key}", self.chain_id);
        let value: Option<String> = self.conn.get(&cache_key)?;

        let v = match value {
            Some(value) => {
                let value =
                    from_str::<CacheValue>(&value).context("fail to deserialize cache value")?;
                CacheStatus::Cached {
                    key: cache_key,
                    value,
                }
            }
            None => CacheStatus::Missed { key: cache_key },
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
        let redis_ttl = cache_value.effective_ttl() * 2;
        let _ = self
            .conn
            .set_ex::<_, _, String>(key, cache_value.to_string()?, redis_ttl.into());
        Ok(())
    }
}
