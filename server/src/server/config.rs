use std::env;
use std::path::Path;
use config::{Config, ConfigError};
use serde::Deserialize;

// https://serde.rs/derive.html
// https://github.com/mehcode/config-rs/blob/master/examples/hierarchical-env/config/default.toml
// https://github.com/mehcode/config-rs/blob/master/examples/hierarchical-env/src/settings.rs

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub connection: DbConnection,
    pub http: HttpListener,
    pub jwt: JwtConfig,
    pub others: Option<OthersConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DbConnection {
    pub url: String,
    pub credentials: DbCredentials,
}

#[derive(Debug, Deserialize)]
pub struct DbCredentials {
    pub user: String,
    pub pw: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HttpListener {
    pub listen: SocketAddress,
    pub tls_key: String,
    pub tls_cert: String
}

#[derive(Debug, Deserialize)]
pub struct SocketAddress {
    pub domain: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct JwtConfig {
    pub public_key: String,
    pub issuer:     String,
}

#[derive(Debug, Deserialize)]
pub struct OthersConfig {
    pub excludes: Vec<String>,
}

pub fn load_config() -> Result<ServerConfig, ConfigError> {
    // Add in the current environment file
    // Default to 'development' env
    // Note that this file is _optional_
    let env = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

    let root = Path::new("config");
    let default_path = root.join("default.toml");
    let current_path = root.join(format!("{}.toml", env));

    let mut config = Config::default();
    config
        .merge(config::File::from(default_path)).unwrap()
        .merge(config::File::from(current_path)).unwrap()
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .merge(config::Environment::with_prefix("APP").separator("_")).unwrap();

        config.try_into()
}
