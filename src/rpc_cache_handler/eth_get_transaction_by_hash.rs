use alloy_primitives::B256;
use anyhow::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct EthGetTransactionByHash;

impl RpcCacheHandler for EthGetTransactionByHash {
    fn method_name(&self) -> &'static str {
        "eth_getTransactionByHash"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        let params = params
            .as_array()
            .context("params not found or not an array")?;

        let tx_hash: B256 = serde_json::from_value(params[0].clone())?;

        Ok(Some(format!("{tx_hash:#x}")))
    }

    fn extract_cache_value(&self, result: &Value) -> anyhow::Result<(bool, String)> {
        common::extract_transaction_cache_value(result)
    }
}
