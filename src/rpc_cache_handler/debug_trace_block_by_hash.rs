use alloy_primitives::B256;
use anyhow::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct DebugTraceBlockByHash;

impl RpcCacheHandler for DebugTraceBlockByHash {
    fn method_name(&self) -> &'static str {
        "debug_traceBlockByHash"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        let params = params
            .as_array()
            .context("params not found or not an array")?;

        let block_hash: B256 =
            serde_json::from_value(params[0].clone()).context("params[0] not a valid block tag")?;
        let tracer_config =
            serde_json::to_string(params[1].as_object().context("params[2] not an object")?)?;
        let tracer_config_hash = common::hash_string(tracer_config.as_str());

        Ok(Some(format!("{block_hash:#x}-{tracer_config_hash}")))
    }
}
