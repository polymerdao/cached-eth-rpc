use alloy_primitives::B256;
use anyhow::Context;
use serde_json::Value;
use std::str::FromStr;

use crate::rpc_cache_handler::common::require_array_params;
use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_getLogs"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        let params = &require_array_params(params, common::ParamsSpec::Exact(1))?[0];

        let filter = params[0]
            .as_object()
            .context("params[0] not a filter object")?;

        let mut block_tag = None;

        if let Some(block_hash) = filter["blockHash"].as_str() {
            if let Ok(block_hash) = B256::from_str(block_hash) {
                block_tag = Some(format!("{:#x}", block_hash));
            }
        }

        if block_tag.is_none() {
            let from_block = if !filter["fromBlock"].is_null() {
                common::extract_and_format_block_number(&filter["fromBlock"])
                    .context("`fromBlock` is not a valid block number")?
            } else {
                None
            };

            let to_block = if !filter["toBlock"].is_null() {
                common::extract_and_format_block_number(&filter["toBlock"])
                    .context("`toBlock` is not a valid block number")?
            } else {
                None
            };

            if let (Some(from_block), Some(to_block)) = (from_block, to_block) {
                block_tag = Some(format!("{}-{}", from_block, to_block));
            }
        }

        let cache_key = block_tag.map(|block_tag| {
            format!(
                "{}-{}",
                block_tag,
                common::hash_string(&serde_json::to_string(filter).unwrap())
            )
        });

        Ok(cache_key)
    }
}
