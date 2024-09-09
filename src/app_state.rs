use crate::args::Args;
use crate::cache::{lru_backend, memory_backend, CacheBackendFactory};
use crate::config::{AppConfig, RpcProxy};
use crate::rpc_provider_backend_group::RpcProviderBackendGroup;
use crate::cache::redis_backend::RedisBackendFactory;
use crate::metrics;
use anyhow::Context;
use reqwest::Url;
use std::collections::HashMap;

struct ChainState {
    rpc_providers: Vec<Url>,
    cache_backend_factory: Box<dyn CacheBackendFactory>,
    handlers: HashMap<String, HandlerEntry>,
    allowed_prefixes: Vec<String>,
}

pub struct AppState {
    chains: HashMap<String, ChainState>,
    rpc_provider_backend_groups: Vec<RpcProviderBackendGroup>
    http_client: reqwest::Client,
    pub metrics: metrics::Metrics,

}

impl AppState {
    pub fn new(args: &Args, cfg: &AppConfig, metrics_prefix: &str) -> anyhow::Result<Self> {

        let rpc_provider_backend_groups = init_rpc_provider_backend_groups(args, cfg);
        let cache_backend_factories = init_cache_backend_factories(args, cfg, &rpc_provider_backend_groups)?;


        // setup cache backend
        Self {
            chains: Default::default(),
            rpc_provider_backend_groups,
            http_client: reqwest::Client::new(),
            metrics: metrics::Metrics::new(metrics_prefix),
        }
    }
}

fn new_cache_backend_factory(
    args: &Args,
    cfg: &AppConfig,
    chain_id: u64,
) -> anyhow::Result<Box<dyn CacheBackendFactory>> {
    let factory: Box<dyn CacheBackendFactory> = match args.cache_type.as_str() {
        "redis" => match &args.redis_url {
            Some(redis_url) => {
                tracing::info!("Using redis cache backend");

                let client = redis::Client::open(redis_url.as_ref())
                    .context("fail to create redis client")?;

                let conn_pool = r2d2::Pool::builder()
                    .max_size(300)
                    .test_on_check_out(false)
                    .build(client)
                    .context("fail to create redis connection pool")?;
                let factory = RedisBackendFactory::new(chain_id, conn_pool, args.reorg_ttl);

                Box::new(factory)
            }
            None => {
                return Err(anyhow::anyhow!(
                    "Must specify redis url when using redis cache backend!"
                ));
            }
        },
        "memory" => {
            tracing::info!("Using in memory cache backend");
            Box::new(memory_backend::MemoryBackendFactory::new(args.reorg_ttl))
        }
        "lru" => {
            tracing::info!("Using in LRU cache backend");
            Box::new(lru_backend::LruBackendFactory::new(
                args.lru_max_items,
                args.reorg_ttl,
            ))
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unknown cache backend specified: {}!",
                args.cache_type
            ));
        }
    };

    Ok(factory)
}

fn init_cache_backend_factories(args: &Args, cfg: &AppConfig, rpc_proxy_backend_groups: &Vec<RpcProviderBackendGroup>) -> anyhow::Result<Vec<Box<dyn CacheBackendFactory>>> {
    rpc_proxy_backend_groups.iter().map(|group| {
        new_cache_backend_factory(args, cfg, group.get_chain_id())
    }).collect()
}

fn init_rpc_provider_backend_groups(args: &Args, cfg: &AppConfig) -> Vec<RpcProviderBackendGroup>{
    cfg.rpc_backends.iter().map(|rpc_proxy: &RpcProxy|{
        RpcProviderBackendGroup::new(&rpc_proxy.provider_backend_group, rpc_proxy.proxy_retry_timeout)
    }).collect()
}
