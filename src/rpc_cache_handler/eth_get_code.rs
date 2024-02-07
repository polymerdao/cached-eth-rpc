use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct EthGetCode;

impl RpcCacheHandler for EthGetCode {
    fn method_name(&self) -> &'static str {
        "eth_getCode"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        common::extract_address_cache_key(params)
    }

    fn extract_cache_value(&self, result: &Value) -> anyhow::Result<(bool, String)> {
        match result.as_str() {
            Some(str) => Ok((str.starts_with("0x"), serde_json::to_string(result)?)),
            _ => Err(anyhow::anyhow!("result not a string")),
        }
    }
}
