use anyhow::{Context, Result, anyhow, bail, ensure};
use bytesize::ByteSize;
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Volume to stat for usage
    pub disk_path: PathBuf,
    /// e.x. "80%" or "2TB"
    pub target_usage: Target,

    #[serde(default)]
    pub dry_run: bool,

    /// Optional watch-history source. Base off only age if empty.
    pub stats: Option<StatsConfig>,
    /// Optional seerr removal.
    pub seerr: Option<SeerrConfig>,

    #[serde(default)]
    pub radarr: Vec<ArrInstanceConfig>,
    #[serde(default)]
    pub sonarr: Vec<ArrInstanceConfig>,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw =
            fs::read_to_string(path).context(format!("reading config at {}", path.display()))?;
        let cfg: Config =
            toml::from_str(&raw).context(format!("parsing config at {}", path.display()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            !self.radarr.is_empty() || !self.sonarr.is_empty(),
            "configure at least on [[radarr]] or [[sonarr]] instance"
        );
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ArrInstanceConfig {
    pub label: String,
    pub url: Url,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
struct StatsConfig {
    pub kind: StatsKind,
    pub url: Url,
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum StatsKind {
    Janitorr,
    Streamystats,
}

#[derive(Debug, Deserialize)]
struct SeerrConfig {
    pub url: Url,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
enum Target {
    UsedPercent(f64),
    UsedBytes(u64),
}

impl FromStr for Target {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();
        if let Some(num_str) = s.strip_suffix('%') {
            let pct: f64 = num_str
                .trim()
                .parse()
                .context(format!("invalid percent in target `{s}`"))?;
            if !(0.0..=100.0).contains(&pct) {
                bail!("target percent `{pct}` must be between 0 and 100");
            }
            Ok(Self::UsedPercent(pct))
        } else {
            let size: ByteSize = s
                .parse()
                .map_err(|e| anyhow!("invalid size in target `{s}`: {e}"))?;
            Ok(Target::UsedBytes(size.as_u64()))
        }
    }
}

impl TryFrom<String> for Target {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self> {
        s.parse()
    }
}
