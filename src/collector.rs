use anyhow::{Context, Result};
use chrono::Utc;
use futures::{Stream, StreamExt, TryStreamExt, stream};
use std::{future, sync::Arc, time::Duration};

use crate::{arr::ArrInstance, domain::MediaItem};

const IGNORE_TAG: &str = "drainarr_ignore";

pub struct CandidateCollector {
    pub instances: Vec<Arc<dyn ArrInstance>>,
    pub min_added_age: Duration,
}

impl CandidateCollector {
    pub fn collect(&self) -> impl Stream<Item = Result<MediaItem>> {
        self.fetch_all()
            .try_filter(move |m| future::ready(self.is_eligible(m)))
    }

    /// Fetches all items from all instances in parallel.
    fn fetch_all(&self) -> impl Stream<Item = Result<MediaItem>> {
        stream::iter(self.instances.iter().cloned())
            .map(|inst| async move {
                let raws = inst
                    .list()
                    .await
                    .context(format!("listing {}", inst.label()))?;
                Ok::<_, anyhow::Error>(stream::iter(raws.into_iter().map(move |raw| {
                    Ok(MediaItem {
                        arr: inst.clone(),
                        raw,
                    })
                })))
            })
            .buffer_unordered(self.instances.len())
            .try_flatten()
    }

    fn within_min_added(&self, m: &MediaItem) -> bool {
        let delta = (Utc::now() - m.raw.added)
            .to_std()
            .unwrap_or(Duration::ZERO);

        delta <= self.min_added_age
    }

    fn is_eligible(&self, m: &MediaItem) -> bool {
        !m.raw.tags.iter().any(|t| *t == IGNORE_TAG) && !self.within_min_added(m)
    }
}
