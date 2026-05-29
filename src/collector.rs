use anyhow::{Context, Result};
use futures::{StreamExt, TryStreamExt, stream};
use std::sync::Arc;

use crate::{arr::ArrInstance, domain::MediaItem};

const IGNORE_TAG: &str = "drainarr_ignore";

pub struct CandidateCollector {
    pub instances: Vec<Arc<dyn ArrInstance>>,
}

impl CandidateCollector {
    pub async fn collect(&self) -> Result<Vec<MediaItem>> {
        Ok(self
            .fetch_all()
            .await?
            .into_iter()
            .filter(Self::is_eligible)
            .collect())
    }

    /// Fetches all items from all instances in parallel.
    async fn fetch_all(&self) -> Result<Vec<MediaItem>> {
        let per_instance = stream::iter(self.instances.iter().cloned())
            .map(|inst| async move {
                let raws = inst
                    .list()
                    .await
                    .context(format!("listing {}", inst.label()))?;
                Ok::<_, anyhow::Error>(
                    raws.into_iter()
                        .map(|raw| MediaItem {
                            arr: inst.clone(),
                            raw,
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .buffer_unordered(self.instances.len())
            .try_collect::<Vec<_>>()
            .await?;

        Ok(per_instance.into_iter().flatten().collect())
    }

    fn is_eligible(m: &MediaItem) -> bool {
        !m.raw.tags.iter().any(|t| *t == IGNORE_TAG)
    }
}
