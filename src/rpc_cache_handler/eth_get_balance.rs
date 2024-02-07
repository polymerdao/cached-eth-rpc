use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct EthGetBalance;

impl RpcCacheHandler for EthGetBalance {
    fn method_name(&self) -> &'static str {
        "eth_getBalance"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        common::extract_address_cache_key(params)
    }
}
