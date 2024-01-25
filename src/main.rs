use std::collections::HashMap;

use actix_web::{error, web, App, Error, HttpResponse, HttpServer};
use anyhow::Context;
use clap::Parser;
use dashmap::DashMap;
use env_logger::Env;
use redis::Commands;
use reqwest::Url;
use serde_json::{json, Value};

use crate::cli::Cli;
use crate::json_rpc::{DefinedError, JsonRpcResponse, RequestId};
use crate::rpc_cache_handler::RpcCacheHandler;

mod cli;
mod json_rpc;
mod rpc_cache_handler;
mod utils;

fn read_cache(
    redis_con: &mut r2d2::PooledConnection<redis::Client>,
    chain_id: u64,
    handler: &dyn RpcCacheHandler,
    method: &str,
    params: &Value,
) -> anyhow::Result<CacheStatus> {
    let cache_key = handler
        .extract_cache_key(params)
        .context("fail to extract cache key")?;

    let cache_key = match cache_key {
        Some(cache_key) => format!("{chain_id}:{method}:{cache_key}"),
        None => return Ok(CacheStatus::NotAvailable),
    };

    let value: Option<String> = redis_con.get(&cache_key).unwrap();

    Ok(if let Some(value) = value {
        let cache_value =
            serde_json::from_str::<Value>(&value).context("fail to deserialize cache value")?;
        CacheStatus::Cached(cache_key, cache_value)
    } else {
        CacheStatus::Missed(cache_key)
    })
}

fn extract_single_request_info(
    mut raw_request: Value,
) -> Result<(RequestId, String, Value), (Option<RequestId>, DefinedError)> {
    let id = RequestId::try_from(raw_request["id"].take())
        .map_err(|_| (None, DefinedError::InvalidRequest))?;
    let method = match raw_request["method"].take() {
        Value::String(s) => s,
        _ => return Err((Some(id), DefinedError::MethodNotFound)),
    };
    let params = raw_request["params"].take();

    Ok((id, method, params))
}

#[actix_web::post("/{chain}")]
async fn rpc_call(
    path: web::Path<(String,)>,
    data: web::Data<AppState>,
    body: web::Json<Value>,
) -> Result<HttpResponse, Error> {
    let (chain,) = path.into_inner();
    let chain_state = data
        .chains
        .get(&chain.to_uppercase())
        .ok_or_else(|| error::ErrorNotFound("endpoint not supported"))?;

    let (requests, is_single_request) = match body {
        web::Json(Value::Array(requests)) => (requests, false),
        web::Json(Value::Object(obj)) => (vec![Value::Object(obj)], true),
        _ => return JsonRpcResponse::from_error(None, DefinedError::InvalidRequest).into(),
    };

    let mut ordered_requests_result: Vec<Option<JsonRpcResponse>> = vec![None; requests.len()];
    let mut uncached_requests = vec![];
    let mut request_id_index_map: HashMap<RequestId, usize> = HashMap::new();

    // Scope the redis connection
    {
        let mut redis_con = data.redis.get().map_err(|err| {
            tracing::error!("fail to get redis connection because: {}", err);
            error::ErrorInternalServerError("fail to get redis connection")
        })?;

        for (index, request) in requests.into_iter().enumerate() {
            let (id, method, params) = match extract_single_request_info(request) {
                Ok(v) => v,
                Err((request_id, err)) => {
                    ordered_requests_result
                        .push(Some(JsonRpcResponse::from_error(request_id, err)));
                    continue;
                }
            };

            let rpc_request = match chain_state.cache_entries.get(&method) {
                Some(cache_entry) => {
                    let result = read_cache(
                        &mut redis_con,
                        chain_state.id,
                        cache_entry.handler.as_ref(),
                        &method,
                        &params,
                    );

                    match result {
                        Ok(CacheStatus::NotAvailable) => {
                            tracing::info!("cache not available for method {}", method);
                            RpcRequest::new_uncachable(index, id, method, params)
                        }
                        Ok(CacheStatus::Cached(cache_key, value)) => {
                            tracing::info!(
                                "cache hit for method {} with key {}",
                                method,
                                cache_key
                            );

                            let response = JsonRpcResponse::from_result(id, value);
                            ordered_requests_result[index] = Some(response);
                            continue;
                        }
                        Ok(CacheStatus::Missed(cache_key)) => {
                            tracing::info!(
                                "cache missed for method {} with key {}",
                                method,
                                cache_key
                            );
                            RpcRequest::new(index, id, method, params, cache_key)
                        }
                        Err(err) => {
                            tracing::error!("fail to read cache because: {}", err);
                            RpcRequest::new_uncachable(index, id, method, params)
                        }
                    }
                }
                None => RpcRequest::new_uncachable(index, id, method, params),
            };

            request_id_index_map.insert(rpc_request.id.clone(), uncached_requests.len());
            uncached_requests.push(rpc_request);
        }
    }

    macro_rules! return_response {
        () => {
            return Ok(match is_single_request {
                true => ordered_requests_result[0].clone().unwrap().into(),
                false => HttpResponse::Ok().json(ordered_requests_result),
            })
        };
    }

    if uncached_requests.is_empty() {
        return_response!();
    }

    let request_body = Value::Array(
        uncached_requests
            .iter()
            .map(|rpc_request| {
                json!({
                    "jsonrpc": "2.0",
                    "id": rpc_request.id.clone(),
                    "method": rpc_request.method,
                    "params": rpc_request.params.clone(),
                })
            })
            .collect::<Vec<Value>>(),
    );

    let rpc_result = utils::do_rpc_request(
        &data.http_client,
        chain_state.rpc_url.clone(),
        &request_body,
    );

    let rpc_result = match rpc_result.await {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("fail to make rpc request because: {}", err);

            for rpc_request in uncached_requests {
                ordered_requests_result[rpc_request.index] = Some(JsonRpcResponse::from_error(
                    Some(rpc_request.id),
                    DefinedError::InternalError(Some(json!({
                        "error": "fail to make rpc request to backend",
                        "reason": err.to_string(),
                    }))),
                ));
            }

            return_response!();
        }
    };

    let result_values = match rpc_result {
        Value::Array(v) => v,
        _ => {
            tracing::error!(
                "array is expected but we got invalid rpc response: {},",
                rpc_result.to_string()
            );

            for rpc_request in uncached_requests {
                ordered_requests_result[rpc_request.index] = Some(JsonRpcResponse::from_error(
                    Some(rpc_request.id),
                    DefinedError::InternalError(Some(json!({
                        "error": "invalid rpc response from backend",
                        "reason": "array is expected",
                        "response": rpc_result.to_string(),
                    }))),
                ));
            }

            return_response!();
        }
    };

    if result_values.len() != uncached_requests.len() {
        tracing::warn!(
            "rpc response length mismatch, expected: {}, got: {}",
            uncached_requests.len(),
            result_values.len()
        );
    }

    let mut redis_con = match data.redis.get() {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("fail to get redis connection because: {}", err);

            for rpc_request in uncached_requests {
                ordered_requests_result[rpc_request.index] = Some(JsonRpcResponse::from_error(
                    Some(rpc_request.id),
                    DefinedError::InternalError(Some(json!({
                        "error": "fail to get redis connection",
                        "reason": err.to_string(),
                    }))),
                ));
            }

            return_response!();
        }
    };

    for (index, mut response) in result_values.into_iter().enumerate() {
        let rpc_request = match RequestId::try_from(response["id"].clone()) {
            Ok(id) if request_id_index_map.get(&id).is_some() => {
                &uncached_requests[*request_id_index_map.get(&id).unwrap()]
            }
            _ => {
                if index >= uncached_requests.len() {
                    tracing::warn!("rpc response has invalid id and fail to map to original request. response is ignored, response: {response}");
                    continue;
                }

                tracing::warn!(
                    "rpc response has invalid id. find a potential match from original request"
                );
                &uncached_requests[index]
            }
        };

        match response["error"].take() {
            Value::Null => {}
            error => {
                let response =
                    JsonRpcResponse::from_custom_error(Some(rpc_request.id.clone()), error);
                ordered_requests_result[rpc_request.index] = Some(response);
                continue;
            }
        }

        let result = response["result"].take();
        let response = JsonRpcResponse::from_result(rpc_request.id.clone(), result.clone());
        ordered_requests_result[rpc_request.index] = Some(response);

        let cache_key = match rpc_request.cache_key.clone() {
            Some(cache_key) => cache_key.clone(),
            None => continue,
        };

        let cache_entry = chain_state.cache_entries.get(&rpc_request.method).unwrap();

        let (can_cache, extracted_value) = match cache_entry.handler.extract_cache_value(&result) {
            Ok(v) => v,
            Err(err) => {
                tracing::error!("fail to extract cache value because: {}", err);

                ordered_requests_result[rpc_request.index] = Some(JsonRpcResponse::from_error(
                    Some(rpc_request.id.clone()),
                    DefinedError::InternalError(Some(json!({
                        "error": "fail to extract cache value",
                        "reason": err.to_string(),
                    }))),
                ));

                continue;
            }
        };

        if can_cache {
            let value = extracted_value.as_str();
            let _ = redis_con.set::<_, _, String>(cache_key.clone(), value);
        }
    }

    return_response!()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let arg = Cli::parse();

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let redis_client = redis::Client::open(arg.redis_url).expect("Failed to create Redis client");
    let redis_con_pool = r2d2::Pool::builder()
        .max_size(300)
        .build(redis_client)
        .expect("Failed to create Redis connection pool");

    let mut app_state = AppState {
        chains: Default::default(),
        http_client: reqwest::Client::new(),
        redis: redis_con_pool,
    };

    let handler_factories = rpc_cache_handler::all_factories();

    tracing::info!("Provisioning cache tables");

    for (name, rpc_url) in arg.endpoints.iter() {
        tracing::info!("Adding endpoint {} linked to {}", name, rpc_url);

        let chain_id = utils::get_chain_id(&reqwest::Client::new(), rpc_url.as_str())
            .await
            .expect("fail to get chain id");

        let mut chain_state = ChainState::new(rpc_url.clone(), chain_id);

        for factory in &handler_factories {
            let handler = factory();
            chain_state
                .cache_entries
                .insert(handler.method_name().to_string(), CacheEntry::new(handler));
        }

        app_state.chains.insert(name.to_string(), chain_state);
    }

    let app_state = web::Data::new(app_state);

    tracing::info!("Server listening on {}:{}", arg.bind, arg.port);

    {
        let app_state = app_state.clone();

        HttpServer::new(move || App::new().service(rpc_call).app_data(app_state.clone()))
            .bind((arg.bind, arg.port))?
            .run()
            .await?;
    }

    tracing::info!("Server stopped");

    Ok(())
}

struct ChainState {
    rpc_url: Url,
    id: u64,
    cache_entries: HashMap<String, CacheEntry>,
}

impl ChainState {
    fn new(rpc_url: Url, chain_id: u64) -> Self {
        Self {
            rpc_url,
            id: chain_id,
            cache_entries: Default::default(),
        }
    }
}

pub type ChainStorePersistedCache = HashMap<String, DashMap<String, String>>;

struct CacheEntry {
    handler: Box<dyn RpcCacheHandler>,
}

impl CacheEntry {
    fn new(handler: Box<dyn RpcCacheHandler>) -> Self {
        Self { handler }
    }
}

struct AppState {
    chains: HashMap<String, ChainState>,
    http_client: reqwest::Client,
    redis: r2d2::Pool<redis::Client>,
}

enum CacheStatus {
    NotAvailable,
    Cached(String, Value),
    Missed(String),
}

#[derive(Debug)]
struct RpcRequest {
    index: usize,
    id: RequestId,
    method: String,
    params: Value,
    cache_key: Option<String>,
}

impl RpcRequest {
    fn new(index: usize, id: RequestId, method: String, params: Value, cache_key: String) -> Self {
        Self {
            index,
            id,
            method,
            params,
            cache_key: Some(cache_key),
        }
    }

    fn new_uncachable(index: usize, id: RequestId, method: String, params: Value) -> Self {
        Self {
            index,
            id,
            method,
            params,
            cache_key: None,
        }
    }
}
