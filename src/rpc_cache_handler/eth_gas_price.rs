use crate::rpc_cache_handler::RpcCacheHandler;
use serde_json::Value;

#[derive(Default, Clone)]
pub struct Handler {}

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_gasPrice"
    }

    fn extract_cache_key(&self, _: &Value) -> anyhow::Result<Option<String>> {
        Ok(Some(format!("eth_gasPrice")))
    }

    fn get_ttl(&self) -> u32 {
        10
    }
}
