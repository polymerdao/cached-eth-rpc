use alloy_primitives::{Address, U256};
use anyhow::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct EthGetStorageAt;

impl RpcCacheHandler for EthGetStorageAt {
    fn method_name(&self) -> &'static str {
        "eth_getStorageAt"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        let params = params
            .as_array()
            .context("params not found or not an array")?;

        let block_tag = common::extract_and_format_block_tag(&params[2])
            .context("params[2] is not a valid block tag")?;
        let block_tag = match block_tag {
            Some(block_tag) => block_tag,
            None => return Ok(None),
        };

        let account: Address =
            serde_json::from_value(params[0].clone()).context("params[0] not a valid address")?;
        let slot: U256 =
            serde_json::from_value(params[1].clone()).context("params[1] not a valid hex value")?;

        Ok(Some(format!("{block_tag}-{account:#x}-{slot:#x}")))
    }
}
