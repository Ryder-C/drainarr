use anyhow::Result;
use bytesize::ByteSize;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::{config::Target, disk::DiskMonitor, domain::Candidate, requests::RequestService};

pub struct EvictionEngine {
    pub disk: DiskMonitor,
    pub target: Target,
    pub requester: Option<Arc<dyn RequestService>>,
    pub dry_run: bool,
    pub settle: Duration,
}

impl EvictionEngine {
    pub async fn evict(&self, sorted: Vec<Candidate>) -> Result<Report> {
        let total = self.disk.usage()?.total;
        let ceiling = self.target.used_ceiling(total);
        let mut used = self.disk.usage()?.used;

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
                used = used.saturating_sub(size); // simulate freeing space
            } else {
                info!(title=%c.item.raw.title, arr=c.item.arr.label(), "deleting");
                self.delete_one(&c).await?;
                sleep(self.settle).await;
                used = self.disk.usage()?.used;
            }

            report.deleted += 1;
            report.freed += size;
        }

        report.reached_target = used <= ceiling;
        Ok(report)
    }

    async fn delete_one(&self, c: &Candidate) -> Result<()> {
        let r = &c.item.raw;
        c.item.arr.delete(r.arr_id, r.season).await?;
        if let Some(req) = &self.requester
            && let Err(e) = req.clear_request(&r.ids, c.item.arr.kind()).await
        {
            warn!(title=%r.title, error=%e, "couldn't clear request from requester service");
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct Report {
    pub deleted: u64,
    pub freed: u64,
    pub reached_target: bool,
}
