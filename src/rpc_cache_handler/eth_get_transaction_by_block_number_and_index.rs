use alloy_primitives::U64;
use anyhow::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct EthGetTransactionByBlockNumberAndIndex;

impl RpcCacheHandler for EthGetTransactionByBlockNumberAndIndex {
    fn method_name(&self) -> &'static str {
        "eth_getTransactionByBlockNumberAndIndex"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        let params = params
            .as_array()
            .context("params not found or not an array")?;

        let block_tag = common::extract_and_format_block_tag(&params[0])
            .context("params[0] is not a valid block tag")?;
        let block_tag = match block_tag {
            Some(block_tag) => block_tag,
            None => return Ok(None),
        };

        let tx_index: U64 =
            serde_json::from_value(params[1].clone()).context("params[1] is not a valid index")?;

        Ok(Some(format!("{block_tag}-{tx_index}")))
    }

    fn extract_cache_value(&self, result: &Value) -> anyhow::Result<(bool, String)> {
        common::extract_transaction_cache_value(result)
    }
}
