use anyhow::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct EthGetBlockByNumber;

impl RpcCacheHandler for EthGetBlockByNumber {
    fn method_name(&self) -> &'static str {
        "eth_getBlockByNumber"
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

        let transaction_detail = params[1].as_bool().context("params[1] not a bool")?;

        Ok(Some(format!("{block_tag}-{transaction_detail}")))
    }
}
