use std::sync::Arc;

use drainarr::{
    ArrHttp, ArrInstance, CandidateCollector, DiskMonitor, Drainarr, EvictionEngine, JanitorrStats,
    RadarrClient, RecencyResolver, SonarrClient, StatsProvider,
    config::{ArrInstanceConfig, Config, StatsKind},
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
    let stats = cfg.stats.map(|s| -> Arc<dyn StatsProvider> {
        match s.kind {
            StatsKind::Janitorr => Arc::new(JanitorrStats {
                base_url: s.url,
                http: http.clone(),
            }),
        }
    });

    let app = Drainarr {
        collector: CandidateCollector {
            instances,
            min_added_age: cfg.min_added_age,
        },
        resolver: RecencyResolver { stats },
        engine: EvictionEngine {
            disk: DiskMonitor {
                path: cfg.disk_path.clone(),
            },
            target: cfg.target_usage,
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
}

fn build_instances(cfg: &Config, http: &reqwest::Client) -> Vec<Arc<dyn ArrInstance>> {
    fn http_for(c: &ArrInstanceConfig, http: &reqwest::Client) -> ArrHttp {
        ArrHttp {
            label: c.label.clone(),
            base_url: c.url.clone(),
            api_key: c.api_key.clone(),
            http: http.clone(),
        }
    }

    let mut v: Vec<Arc<dyn ArrInstance>> = Vec::new();
    for c in &cfg.radarr {
        let api = http_for(c, http);
        v.push(Arc::new(RadarrClient { api }))
    }
    for c in &cfg.sonarr {
        let api = http_for(c, http);
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
