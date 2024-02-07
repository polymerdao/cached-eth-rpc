use serde_json::Value;

use crate::rpc_cache_handler::{common, RpcCacheHandler};
use crate::rpc_cache_handler::common::ParamsSpec;

#[derive(Default, Clone)]
pub struct Handler;

impl RpcCacheHandler for Handler {
    fn method_name(&self) -> &'static str {
        "eth_chainId"
    }

    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        common::require_array_params(params, ParamsSpec::Exact(0))?;

        Ok(Some("static".to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    static HANDLER: Handler = Handler;

    #[test]
    fn test_invalid_params_len() {
        let params = json!([1]);
        assert_eq!(HANDLER.extract_cache_key(&params).unwrap_err().to_string(), "expected 0 params, got 1");
    }

    #[test]
    fn test() {
        let params = json!([]);
        let cache_key = HANDLER.extract_cache_key(&params).unwrap().unwrap();
        assert_eq!(cache_key, "static");
    }


}
