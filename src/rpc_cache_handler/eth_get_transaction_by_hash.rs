use serde_json::Value;

use crate::cache::CacheValue;
use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler {
    inner: super::eth_get_transaction_receipt::Handler,
}

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_getTransactionByHash"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        self.inner.extract_cache_key(params)
    }

    fn extract_cache_value(&self, result: Value) -> anyhow::Result<(bool, CacheValue)> {
        common::extract_transaction_cache_value(result, self.get_ttl())
    }
}
