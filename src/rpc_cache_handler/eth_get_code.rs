use anyhow::Context;
use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};

#[derive(Default, Clone)]
pub struct EthGetCode;

impl RpcCacheHandler for EthGetCode {
    fn method_name(&self) -> &'static str {
        "eth_getCode"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        let params = params
            .as_array()
            .context("params not found or not an array")?;

        let account = params[0].as_str().context("params[0] not a string")?;
        let block_tag = common::extract_and_format_block_tag(&params[1])?;

        let block_tag = block_tag.unwrap_or_else(|| "dummy".to_string());

        Ok(Some(format!("{}-{}", block_tag, account)))
    }

    fn extract_cache_value(&self, result: &Value) -> anyhow::Result<(bool, String)> {
        match result.as_str() {
            Some(str) => Ok((str.starts_with("0x"), serde_json::to_string(result)?)),
            _ => Err(anyhow::anyhow!("result not a string")),
        }
    }
}
