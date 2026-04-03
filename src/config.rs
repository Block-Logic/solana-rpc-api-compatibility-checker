use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_endpoint: String,
    pub minimum_request_interval_ms: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let rpc_endpoint = std::env::var("RPC_ENDPOINT")
            .context("RPC_ENDPOINT must be set in the environment or .env file")?;

        Ok(Self {
            rpc_endpoint,
            minimum_request_interval_ms: 2_000,
        })
    }
}
