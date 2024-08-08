use crate::rpc_cache_handler::{common, RpcCacheHandler};
use serde_json::Value;

#[derive(Default, Clone)]
pub struct Handler {}

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_maxPriorityFeePerGas"
    }

    fn extract_cache_key(&self, _: &Value) -> anyhow::Result<Option<String>> {
        let bucket = common::compute_cache_bucket(2);
        Ok(Some(format!("eth_maxPriorityFeePerGas-{bucket}")))
    }
}
