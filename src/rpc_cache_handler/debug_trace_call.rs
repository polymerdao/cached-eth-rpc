use anyhow::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct DebugTraceCall;

impl RpcCacheHandler for DebugTraceCall {
    fn method_name(&self) -> &'static str {
        "debug_traceCall"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        let params = params
            .as_array()
            .context("params not found or not an array")?;

        let tx = serde_json::to_string(params[0].as_object().expect("params[0] not an object"))?;
        let block_tag = common::extract_and_format_block_tag(&params[1])?;
        let tracer_config =
            serde_json::to_string(params[2].as_object().expect("params[2] not an object"))?;

        if block_tag.is_none() {
            return Ok(None);
        }

        let block_tag = block_tag.unwrap();
        let tx_hash = common::hash_string(tx.as_str());
        let tracer_config_hash = common::hash_string(tracer_config.as_str());

        Ok(Some(format!("{block_tag}-{tx_hash}-{tracer_config_hash}")))
    }
}
