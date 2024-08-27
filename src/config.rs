use config::{Config, ConfigError};
use serde::{self, de, Deserialize, Deserializer};
use std::env;
use std::path::Path;
use std::str::FromStr;
use url::Url;

#[derive(Debug, Deserialize)]
struct AppConfig {
    server: ServerConfig,
    cache_backend: CacheBackend,
    rpc_backends: Vec<RpcProxy>,
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct CacheBackend {
    cache_type: String, // todo: make enum with custom deserializer
    #[serde(deserialize_with = "deserialize_and_validate_url")]
    redis_url: Option<Url>,
}

#[derive(Debug, Deserialize)]
struct ProviderGroup {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct RpcProxy {
    chain_name: String,
    path_prefix: String,
    #[serde(deserialize_with = "deserialize_and_validate_urls")]
    provider_backend_group: Vec<Url>,
    reorg_ttl: u32,
    allowed_method_prefixes: Vec<String>,
}

impl AppConfig {
    pub fn new(config_file: String) -> Result<Self, ConfigError> {
        // Get the binary name and convert it to uppercase
        let env_prefix = env::args()
            .next()
            .and_then(|path| {
                Path::new(&path)
                    .file_name()
                    .map(|os_str| os_str.to_str().unwrap_or_default().to_string())
            })
            .unwrap_or_else(|| "DEFAULT".to_string())
            .to_ascii_uppercase();

        let cfg = Config::builder()
            .add_source(config::File::with_name(&config_file))
            .add_source(config::Environment::with_prefix(&env_prefix))
            .build()?;

        cfg.try_deserialize()
    }
}

// Custom deserializer for urls
fn deserialize_and_validate_urls<'de, D>(deserializer: D) -> Result<Vec<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    let urls: Vec<String> = Vec::deserialize(deserializer)?;
    let mut valid_urls = Vec::with_capacity(urls.len());

    for url_str in urls {
        match Url::from_str(&url_str) {
            Ok(url) => valid_urls.push(url),
            Err(_) => return Err(de::Error::custom(format!("Invalid URL: {}", url_str))),
        }
    }

    Ok(valid_urls)
}

// Custom deserializer for urls
fn deserialize_and_validate_url<'de, D>(deserializer: D) -> Result<Option<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    let url_str = String::deserialize(deserializer)?;

    match Url::from_str(&url_str) {
        Ok(url) => Ok(Some(url)),
        Err(_) => return Err(de::Error::custom(format!("Invalid URL: {}", url_str))),
    }
}
