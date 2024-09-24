use actix_web::{web, Error, HttpResponse};
use prometheus::{Counter, Encoder, Registry, TextEncoder, IntCounterVec};

pub struct Metrics {
    pub registry: prometheus::Registry,
    pub cache_hit_counter: Counter,
    pub cache_miss_counter: Counter,
    pub cache_expired_miss_counter: Counter,
    pub cache_uncacheable_counter: Counter,
    pub error_counter: Counter,
    pub method_call_counter: IntCounterVec,
}

// Function to add a prefix to the metric names
fn add_prefix(prefix: &str, name: &str) -> String {
    format!("{}_{}", prefix, name)
}

// Create a function to register metrics with a prefix
fn register_counter_with_prefix(
    registry: &Registry,
    prefix: &str,
    name: &str,
    description: &str,
) -> Counter {
    let name = add_prefix(prefix, name);
    let opts = prometheus::Opts::new(name, description);
    let counter = prometheus::Counter::with_opts(opts).unwrap();
    registry.register(Box::new(counter.clone())).unwrap();
    counter
}

// Create a function to register IntCounterVec with a prefix
fn register_int_counter_vec_with_prefix(
    registry: &Registry,
    prefix: &str,
    name: &str,
    description: &str,
    labels: &[&str],
) -> IntCounterVec {
    let name = add_prefix(prefix, name);
    let opts = prometheus::Opts::new(name, description);
    let counter_vec = IntCounterVec::new(opts, labels).unwrap();
    registry.register(Box::new(counter_vec.clone())).unwrap();
    counter_vec
}

impl Metrics {
    pub fn new(prefix: &str) -> Self {
        let registry = Registry::new();

        let cache_hit_counter = register_counter_with_prefix(
            &registry,
            prefix,
            "cache_hit_total",
            "Total number of cache hits.",
        );
        let cache_miss_counter = register_counter_with_prefix(
            &registry,
            prefix,
            "cache_miss_total",
            "Total number of cache misses across all miss types.",
        );
        let cache_expired_miss_counter = register_counter_with_prefix(
            &registry,
            prefix,
            "cache_expired_miss_total",
            "Total number of expired cache misses.",
        );
        let cache_uncacheable_counter = register_counter_with_prefix(
            &registry,
            prefix,
            "cache_uncacheable_total",
            "Total number of uncacheable requests.",
        );
        let error_counter = register_counter_with_prefix(
            &registry,
            prefix,
            "error_total",
            "Total number of errors.",
        );
        let method_call_counter = register_int_counter_vec_with_prefix(
            &registry,
            prefix,
            "method_call_total",
            "Total number of method calls per chain",
            &["chain", "method", "cache"],
        );

        Self {
            registry,
            cache_hit_counter,
            cache_miss_counter,
            cache_expired_miss_counter,
            cache_uncacheable_counter,
            error_counter,
            method_call_counter,
        }
    }
}

// Metrics handler
#[actix_web::get("/metrics")]
async fn metrics(data: web::Data<crate::AppState>) -> Result<HttpResponse, Error> {
    let encoder = TextEncoder::new();
    let metric_families = data.metrics.registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Ok(HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(buffer))
}
