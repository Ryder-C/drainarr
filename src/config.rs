use anyhow::{Context, Result};
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Config {
    database_url: Option<Url>,

    #[serde(default = "default_sonarr_base_url")]
    sonarr_base_url: Url,
    sonarr_api_key: Option<String>,

    #[serde(default = "default_radarr_base_url")]
    radarr_base_url: Url,
    radarr_api_key: Option<String>,

    #[serde(default = "default_seerr_base_url")]
    seerr_base_url: Url,
    seerr_api_key: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        envy::from_env::<Config>()
            .context("Failed to load configuration from environment variables")
    }
}

pub fn default_sonarr_base_url() -> Url {
    Url::parse("http://localhost:8989").unwrap()
}

pub fn default_radarr_base_url() -> Url {
    Url::parse("http://localhost:7878").unwrap()
}

pub fn default_seerr_base_url() -> Url {
    Url::parse("http://localhost:5055").unwrap()
}
