use anyhow::Result;
use bytesize::ByteSize;
use futures::TryStreamExt;
use tracing::info;

use crate::{
    collector::CandidateCollector, domain::Candidate, engine::EvictionEngine,
    resolver::RecencyResolver,
};

pub struct Drainarr {
    pub collector: CandidateCollector,
    pub resolver: RecencyResolver,
    pub engine: EvictionEngine,
}

impl Drainarr {
    pub async fn run_once(&self) -> Result<()> {
        // Check disk usage first
        if self.engine.disk.under_target(&self.engine.target)? {
            return Ok(());
        }

        // Collect candidates
        let items = self.collector.collect();
        let mut candidates: Vec<Candidate> = self.resolver.resolve(items).try_collect().await?;
        candidates.sort_unstable_by_key(|c| c.recency);
        info!(
            eligible = candidates.len(),
            "ranked candidates, starting drain"
        );

        // Evict until target reached
        let report = self.engine.evict(candidates).await?;

        info!(
            deleted = report.deleted,
            freed_bytes = %ByteSize(report.freed),
            reached_target = report.reached_target,
            "drain complete"
        );

        Ok(())
    }
}
