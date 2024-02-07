use anyhow::Result;
use serde_json::Value;

mod common;
mod debug_trace_block_by_hash;
mod debug_trace_block_by_number;
mod debug_trace_call;
mod eth_call;
mod eth_chainid;
mod eth_get_balance;
mod eth_get_block_by_hash;
mod eth_get_block_by_number;
mod eth_get_block_receipts;
mod eth_get_code;
mod eth_get_storage_at;
mod eth_get_transaction_by_block_hash_and_index;
mod eth_get_transaction_by_block_number_and_index;
mod eth_get_transaction_by_hash;
mod eth_get_transaction_count;
mod eth_get_transaction_receipt;

pub trait RpcCacheHandler: Send + Sync {
    fn method_name(&self) -> &'static str;

    fn extract_cache_key(&self, params: &Value) -> Result<Option<String>>;

    fn extract_cache_value(&self, result: &Value) -> Result<(bool, String)> {
        Ok((!result.is_null(), serde_json::to_string(result)?))
    }
}

pub type RpcCacheHandlerFactory = fn() -> Box<dyn RpcCacheHandler>;

macro_rules! define_factory {
    ($HandlerType: expr) => {
        || Box::new($HandlerType) as Box<dyn RpcCacheHandler>
    };
}

pub fn all_factories() -> Vec<RpcCacheHandlerFactory> {
    vec![
        define_factory!(debug_trace_block_by_hash::Handler),
        define_factory!(debug_trace_block_by_number::Handler),
        define_factory!(debug_trace_call::Handler),
        define_factory!(eth_call::Handler),
        define_factory!(eth_chainid::Handler),
        define_factory!(eth_get_balance::Handler),
        define_factory!(eth_get_block_by_hash::Handler),
        define_factory!(eth_get_block_by_number::Handler),
        define_factory!(eth_get_block_receipts::Handler),
        define_factory!(eth_get_code::Handler),
        define_factory!(eth_get_storage_at::Handler),
        define_factory!(eth_get_transaction_by_block_hash_and_index::Handler),
        define_factory!(eth_get_transaction_by_block_number_and_index::Handler),
        define_factory!(eth_get_transaction_by_hash::Handler),
        define_factory!(eth_get_transaction_count::Handler),
        define_factory!(eth_get_transaction_receipt::Handler),
    ]
}
