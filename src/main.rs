use actix_web::{error, web, App, Error, HttpResponse, HttpServer};
use anyhow::Context;
use cache::{lru_backend, memory_backend, CacheBackendFactory};
use clap::Parser;
use env_logger::Env;
use reqwest::header::CACHE_STATUS;
use reqwest::{Response, StatusCode, Url};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::app_state::AppState;
use crate::args::Args;
use crate::cache::redis_backend::RedisBackendFactory;
use crate::cache::{CacheStatus, CacheValue};
use crate::config::AppConfig;
use crate::json_rpc::{DefinedError, JsonRpcRequest, JsonRpcResponse, RequestId};
use crate::rpc_cache_handler::RpcCacheHandler;

use tracing::debug;

mod app_state;
mod args;
mod cache;
mod config;
mod json_rpc;
mod metrics;
mod rpc_cache_handler;
mod rpc_provider_backend_group;
mod utils;

// Health check handler
#[actix_web::get("/health")]
async fn health_check() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().body("OK"))
}

// RPC handler
#[actix_web::post("/{chain}")]
async fn rpc_call(
    path: web::Path<(String,)>,
    data: web::Data<AppState>,
    body: web::Json<Value>,
) -> Result<HttpResponse, Error> {
    let metrics = &data.metrics;
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
    let mut uncached_requests: Vec<(RpcRequest, Option<CacheValue>)> = vec![];
    let mut request_id_index_map: HashMap<RequestId, usize> = HashMap::new();

    let mut rcp_response: HttpResponse = HttpResponse::new(reqwest::StatusCode::OK);

    // Scope the redis connection
    {
        // retrieve the caching backend (memory, redis, etc)
        let mut cache_backend = match chain_state.cache_factory.get_instance() {
            Ok(v) => v,
            Err(err) => {
                tracing::error!("fail to get cache backend because: {err:#}");
                return JsonRpcResponse::from_error(
                    None,
                    DefinedError::InternalError(Some(json!({
                        "error": "fail to get cache backend",
                        "reason": err.to_string(),
                    }))),
                )
                .into();
            }
        };

        // iterate through each request looking for the result in cache and aggregating uncached requests
        for (index, request) in requests.into_iter().enumerate() {
            let (id, method, params) = match extract_single_request_info(request) {
                Ok(v) => v,
                Err((request_id, err)) => {
                    ordered_requests_result
                        .push(Some(JsonRpcResponse::from_error(request_id, err)));
                    continue;
                }
            };

            // Check if the method starts with an allowed prefix
            if !chain_state
                .allowed_prefixes
                .iter()
                .any(|prefix| method.starts_with(prefix))
            {
                tracing::warn!("Method '{}' is not allowed", method);
                ordered_requests_result[index] = Some(JsonRpcResponse::from_error(
                    Some(id.clone()),
                    DefinedError::MethodNotFound,
                ));
                continue;
            }

            macro_rules! push_uncached_request_and_continue {
                () => {{
                    let rpc_request = RpcRequest::new_uncachable(index, id, method, params);
                    request_id_index_map.insert(rpc_request.id.clone(), uncached_requests.len());
                    uncached_requests.push((rpc_request, None));
                    continue;
                }};

                ($key: expr) => {{
                    let rpc_request = RpcRequest::new(index, id, method, params, $key);
                    request_id_index_map.insert(rpc_request.id.clone(), uncached_requests.len());
                    uncached_requests.push((rpc_request, None));
                    continue;
                }};

                ($key: expr, $val: expr) => {{
                    let rpc_request = RpcRequest::new(index, id, method, params, $key);
                    request_id_index_map.insert(rpc_request.id.clone(), uncached_requests.len());
                    uncached_requests.push((rpc_request, Some($val)));
                    continue;
                }};
            }

            // retrieve the handler for the requested method
            let handler = match chain_state.handlers.get(&method) {
                Some(handler) => handler,
                None => {
                    metrics.cache_uncacheable_counter.inc();
                    tracing::warn!(method, "cache is not supported");
                    push_uncached_request_and_continue!()
                }
            };

            // get the cache key from the handler based on the request params
            debug!("params: {:?}", params);
            let params_key = match handler.extract_cache_key(&params) {
                Ok(Some(params_key)) => params_key,
                Ok(None) => push_uncached_request_and_continue!(),
                Err(err) => {
                    tracing::error!(
                        method,
                        params = format_args!("{}", params),
                        "fail to extract cache key: {err:#}",
                    );
                    metrics.error_counter.inc();
                    push_uncached_request_and_continue!();
                }
            };

            // read results from cache
            match cache_backend.read(&method, &params_key) {
                Ok(CacheStatus::Cached { key, value }) => {
                    if !value.is_expired() {
                        metrics.cache_hit_counter.inc();
                        tracing::info!("cache hit for method {} with key {}", method, key);
                        ordered_requests_result[index] =
                            Some(JsonRpcResponse::from_result(id, value.data));
                    } else {
                        metrics.cache_miss_counter.inc();
                        metrics.cache_expired_miss_counter.inc();
                        tracing::info!("cache expired for method {} with key {}", method, key);
                        push_uncached_request_and_continue!(key, value);
                    }
                }
                Ok(CacheStatus::Missed { key }) => {
                    metrics.cache_miss_counter.inc();
                    tracing::info!("cache missed for method {} with key {}", method, key);
                    push_uncached_request_and_continue!(key);
                }
                Err(err) => {
                    metrics.error_counter.inc();
                    tracing::error!("fail to read cache because: {err:#}");
                    push_uncached_request_and_continue!();
                }
            }
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

    // if nothing to cache then return empty response
    if uncached_requests.is_empty() {
        return_response!();
    }

    let rpc_requests: Vec<RpcRequest> = uncached_requests
        .iter()
        .map(|(req, _)| req.clone())
        .collect();

    // prepare rpc and return the result future
    let rpc_result = utils::do_rpc_request(
        &data.http_client,
        chain_state.rpc_providers[0].clone(),
        &rpc_requests,
    );

    // await the rpc response, for each cache miss record the response
    let rpc_result = match rpc_result.await {
        Ok(v) => v,
        Err(err) => {
            metrics.error_counter.inc();
            tracing::error!("fail to make rpc request because: {}", err);

            for (rpc_request, _) in uncached_requests {
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

    // unwrap rpc_result into a vector of responses
    let result_values = match rpc_result {
        Value::Array(v) => v,
        _ => {
            metrics.error_counter.inc();
            tracing::error!(
                "array is expected but we got invalid rpc response: {},",
                rpc_result.to_string()
            );

            for (rpc_request, _) in uncached_requests {
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

    // ensure we got the expected number of responses
    if result_values.len() != uncached_requests.len() {
        metrics.error_counter.inc();
        tracing::warn!(
            "rpc response length mismatch, expected: {}, got: {}",
            uncached_requests.len(),
            result_values.len()
        );
    }

    // get the cache backend
    let mut cache_backend = match chain_state.cache_factory.get_instance() {
        Ok(v) => v,
        Err(err) => {
            metrics.error_counter.inc();
            tracing::error!("fail to get cache backend because: {}", err);

            for (rpc_request, _) in uncached_requests {
                ordered_requests_result[rpc_request.index] = Some(JsonRpcResponse::from_error(
                    Some(rpc_request.id),
                    DefinedError::InternalError(Some(json!({
                        "error": "fail to get cache backend",
                        "reason": err.to_string(),
                    }))),
                ));
            }

            return_response!();
        }
    };

    // for each response, get the corresponding request
    // if the response was an error, record an error result and continue
    // else assign the response and extract the cache key for insertion
    // into the cache backend.
    for (index, mut response) in result_values.into_iter().enumerate() {
        let (rpc_request, cache_value) = match RequestId::try_from(response["id"].clone()) {
            Ok(id) if request_id_index_map.get(&id).is_some() => {
                &uncached_requests[*request_id_index_map.get(&id).unwrap()]
            }
            _ => {
                if index >= uncached_requests.len() {
                    metrics.error_counter.inc();
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
                metrics.error_counter.inc();
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

        // It's safe to unwrap here because if the cache system doesn't support this method, we have already
        // made the early return.
        let handler = chain_state.handlers.get(&rpc_request.method).unwrap();

        let (is_cacheable, extracted_value) =
            match handler.extract_cache_value(result, cache_backend.get_reorg_ttl()) {
                Ok(v) => v,
                Err(err) => {
                    metrics.error_counter.inc();
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

        if is_cacheable {
            let _ = cache_backend.write(cache_key.as_str(), extracted_value, cache_value);
        }
    }

    return_response!()
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let args = Args::parse();

    let cfg = AppConfig::new(args.config_file);

    let mut app_state = AppState::new("cached_eth_rpc")

    let handler_factories = rpc_cache_handler::factories();

    for (name, rpc_url) in args.endpoints.iter() {
        tracing::info!("Linked `{name}` to endpoint {rpc_url}");

        let chain_id = utils::get_chain_id(&reqwest::Client::new(), rpc_url.as_str())
            .await
            .expect("fail to get chain id");

        let cache_factory = new_cache_backend_factory(&args, chain_id)
            .expect("fail to create cache backend factory");

        let mut chain_state = ChainState {
            rpc_providers: vec![rpc_url.clone()], // FIXME
            handlers: Default::default(),
            cache_factory,
            allowed_prefixes: vec![
                "eth_".to_string(),
                "alchemy_".to_string(),
                "net_".to_string(),
            ],
        };

        for factory in &handler_factories {
            let handler = factory();
            chain_state.handlers.insert(
                handler.method_name().to_string(),
                HandlerEntry { inner: handler },
            );
        }

        app_state.chains.insert(name.to_string(), chain_state);
    }

    let app_state = web::Data::new(app_state);

    tracing::info!("Server listening on {}:{}", args.bind, args.port);

    {
        let app_state = app_state.clone();

        HttpServer::new(move || {
            App::new()
                .app_data(app_state.clone())
                .service(rpc_call)
                .service(metrics::metrics)
                .service(health_check)
        })
        .bind((args.bind, args.port))?
        .run()
        .await?;
    }

    tracing::info!("Server stopped");

    Ok(())
}


struct HandlerEntry {
    inner: Box<dyn RpcCacheHandler>,
}

impl HandlerEntry {
    fn extract_cache_key(&self, params: &Value) -> anyhow::Result<Option<String>> {
        self.inner.extract_cache_key(params)
    }

    fn extract_cache_value(
        &self,
        result: Value,
        reorg_ttl: u32,
    ) -> anyhow::Result<(bool, CacheValue)> {
        self.inner.extract_cache_value(result, reorg_ttl)
    }
}

#[derive(Debug, Clone)]
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

impl Serialize for RpcRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        JsonRpcRequest::new(
            Some(self.id.clone()),
            self.method.clone(),
            self.params.clone(),
        )
        .serialize(serializer)
    }
}
