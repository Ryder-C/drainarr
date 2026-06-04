use anyhow::Result;
use bytesize::ByteSize;
use tracing::{info, warn};

use crate::{
    config::Target,
    disk::{DiskMonitor, Usage},
    domain::Candidate,
};

pub struct EvictionEngine {
    pub disk: DiskMonitor,
    pub target: Target,
    pub dry_run: bool,
}

impl EvictionEngine {
    pub async fn evict(&self, sorted: impl IntoIterator<Item = Candidate>) -> Result<Report> {
        let Usage { total, mut used } = self.disk.usage()?;
        let ceiling = self.target.used_ceiling(total);

        let mut report = Report::default();
        let mut it = sorted.into_iter();

        while used > ceiling {
            let Some(c) = it.next() else {
                warn!("ran out of candidates before reaching target usage");
                break;
            };
            let size = c.item.raw.size_bytes;

            if self.dry_run {
                info!(title=%c.item.raw.title, arr=c.item.arr.label(), size=%ByteSize(size), recency=%c.recency, "DRY RUN: would delete");
            } else {
                info!(title=%c.item.raw.title, arr=c.item.arr.label(), size=%ByteSize(size), recency=%c.recency, "deleting");
                self.delete_one(&c).await?;
            }

            used = used.saturating_sub(size);
            report.deleted += 1;
            report.freed += size;
        }

        report.reached_target = used <= ceiling;
        Ok(report)
    }

    async fn delete_one(&self, c: &Candidate) -> Result<()> {
        let r = &c.item.raw;
        c.item.arr.delete(r.arr_id, r.season).await
    }
}

#[derive(Default)]
pub struct Report {
    pub deleted: u64,
    pub freed: u64,
    pub reached_target: bool,
}
