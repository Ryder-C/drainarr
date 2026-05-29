use std::sync::Arc;

use drainarr::{
    ArrHttp, ArrInstance, CandidateCollector, Config, DiskMonitor, Drainarr, EvictionEngine,
    JanitorrStats, RadarrClient, RecencyResolver, RequestService, SeerrClient, SonarrClient,
    StatsKind, StatsProvider,
};
use tokio::time::interval;
use tracing::error;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let cfg = Config::load("config.toml")?;
    let http = reqwest::Client::new();

    let instances = build_instances(&cfg, &http);

    // Optional backends
    let stats: Option<Arc<dyn StatsProvider>> = if let Some(stats) = cfg.stats {
        match stats.kind {
            StatsKind::Janitorr => Some(Arc::new(JanitorrStats {
                base_url: stats.url,
                http: http.clone(),
            })),
        }
    } else {
        None
    };
    let requester: Option<Arc<dyn RequestService>> = if let Some(seerr) = cfg.seerr {
        Some(Arc::new(SeerrClient {
            base_url: seerr.url,
            api_key: seerr.api_key,
            http,
        }))
    } else {
        None
    };

    let app = Drainarr {
        collector: CandidateCollector { instances },
        resolver: RecencyResolver { stats },
        engine: EvictionEngine {
            disk: DiskMonitor {
                path: cfg.disk_path.clone(),
            },
            target: cfg.target_usage,
            requester,
            dry_run: cfg.dry_run,
            settle: cfg.settle_time,
        },
    };

    // Run loop
    let mut tick = interval(cfg.check_interval);
    loop {
        tick.tick().await;
        if let Err(e) = app.run_once().await {
            error!(error = %e, "drain run failed, retrying next tick");
        }
    }
    Ok(())
}

fn build_instances(cfg: &Config, http: &reqwest::Client) -> Vec<Arc<dyn ArrInstance>> {
    let mut v: Vec<Arc<dyn ArrInstance>> = Vec::new();
    for c in &cfg.radarr {
        let api = ArrHttp {
            label: c.label.clone(),
            base_url: c.url.clone(),
            api_key: c.api_key.clone(),
            http: http.clone(),
        };

        v.push(Arc::new(RadarrClient { api }))
    }
    for c in &cfg.sonarr {
        let api = ArrHttp {
            label: c.label.clone(),
            base_url: c.url.clone(),
            api_key: c.api_key.clone(),
            http: http.clone(),
        };

        v.push(Arc::new(SonarrClient { api }))
    }
    v
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().without_time())
        .init();
}
