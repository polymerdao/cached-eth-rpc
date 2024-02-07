use anyhow::Result;
use serde_json::Value;

mod common;
mod debug_trace_block_by_hash;
mod debug_trace_block_by_number;
mod debug_trace_call;
mod eth_call;
mod eth_chainid;
mod eth_get_balance;
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

pub fn all_factories() -> Vec<RpcCacheHandlerFactory> {
    vec![
        || Box::new(debug_trace_block_by_hash::DebugTraceBlockByHash) as Box<dyn RpcCacheHandler>,
        || {
            Box::new(debug_trace_block_by_number::DebugTraceBlockByNumber)
                as Box<dyn RpcCacheHandler>
        },
        || Box::new(debug_trace_call::DebugTraceCall) as Box<dyn RpcCacheHandler>,
        || Box::new(eth_call::EthCall) as Box<dyn RpcCacheHandler>,
        || Box::new(eth_chainid::EthChainId) as Box<dyn RpcCacheHandler>,
        || Box::new(eth_get_balance::EthGetBalance) as Box<dyn RpcCacheHandler>,
        || Box::new(eth_get_block_by_number::EthGetBlockByNumber) as Box<dyn RpcCacheHandler>,
        || Box::new(eth_get_block_receipts::EthGetBlockReceipts) as Box<dyn RpcCacheHandler>,
        || Box::new(eth_get_code::EthGetCode) as Box<dyn RpcCacheHandler>,
        || Box::new(eth_get_storage_at::EthGetStorageAt) as Box<dyn RpcCacheHandler>,
        || {
            Box::new(
                eth_get_transaction_by_block_hash_and_index::EthGetTransactionByBlockHashAndIndex,
            ) as Box<dyn RpcCacheHandler>
        },
        || {
            Box::new(eth_get_transaction_by_block_number_and_index::EthGetTransactionByBlockNumberAndIndex) as Box<dyn RpcCacheHandler>
        },
        || {
            Box::new(eth_get_transaction_by_hash::EthGetTransactionByHash)
                as Box<dyn RpcCacheHandler>
        },
        || Box::new(eth_get_transaction_count::EthGetTransactionCount) as Box<dyn RpcCacheHandler>,
        || {
            Box::new(eth_get_transaction_receipt::EthGetTransactionReceipt)
                as Box<dyn RpcCacheHandler>
        },
    ]
}
