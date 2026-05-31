use anyhow::{Context, Result};
use futures::{Stream, StreamExt, TryStreamExt, stream};
use std::{future, sync::Arc};

use crate::{arr::ArrInstance, domain::MediaItem};

const IGNORE_TAG: &str = "drainarr_ignore";

pub struct CandidateCollector {
    pub instances: Vec<Arc<dyn ArrInstance>>,
}

impl CandidateCollector {
    pub fn collect(&self) -> impl Stream<Item = Result<MediaItem>> {
        self.fetch_all()
            .try_filter(|m| future::ready(Self::is_eligible(m)))
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

    fn is_eligible(m: &MediaItem) -> bool {
        !m.raw.tags.iter().any(|t| *t == IGNORE_TAG)
    }
}
