use anyhow::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct DebugTraceBlockByNumber;

impl RpcCacheHandler for DebugTraceBlockByNumber {
    fn method_name(&self) -> &'static str {
        "debug_traceBlockByNumber"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        let params = params
            .as_array()
            .context("params not found or not an array")?;

        let block_tag = common::extract_and_format_block_tag(&params[0])?;
        let tracer_config =
            serde_json::to_string(params[1].as_object().context("params[2] not an object")?)?;

        if block_tag.is_none() {
            return Ok(None);
        }

        let block_tag = block_tag.unwrap();
        let tracer_config_hash = common::hash_string(tracer_config.as_str());

        Ok(Some(format!("{block_tag}-{tracer_config_hash}")))
    }
}
