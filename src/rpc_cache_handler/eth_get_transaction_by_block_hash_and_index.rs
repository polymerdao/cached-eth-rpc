use alloy_primitives::{B256, U64};
use anyhow::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct EthGetTransactionByBlockHashAndIndex;

impl RpcCacheHandler for EthGetTransactionByBlockHashAndIndex {
    fn method_name(&self) -> &'static str {
        "eth_getTransactionByBlockHashAndIndex"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        let params = params
            .as_array()
            .context("params not found or not an array")?;

        let block_tag: B256 =
            serde_json::from_value(params[0].clone()).context("params[0] is not a valid hash")?;
        let tx_index: U64 =
            serde_json::from_value(params[1].clone()).context("params[1] is not a valid index")?;

        Ok(Some(format!("{block_tag:#x}-{tx_index}")))
    }

    fn extract_cache_value(&self, result: &Value) -> anyhow::Result<(bool, String)> {
        common::extract_transaction_cache_value(result)
    }
}
