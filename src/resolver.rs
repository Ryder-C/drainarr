use anyhow::Result;
use std::sync::Arc;

use futures::{Stream, TryStreamExt};

use crate::{
    domain::{Candidate, MediaItem},
    stats::StatsProvider,
};

const STATS_CONCURRENCY: usize = 16;

pub struct RecencyResolver {
    pub stats: Option<Arc<dyn StatsProvider>>,
}

impl RecencyResolver {
    pub fn resolve(
        &self,
        items: impl Stream<Item = Result<MediaItem>>,
    ) -> impl Stream<Item = Result<Candidate>> {
        items
            .map_ok(move |item| async move { Ok::<_, anyhow::Error>(self.resolve_one(item).await) })
            .try_buffer_unordered(STATS_CONCURRENCY)
    }

    async fn resolve_one(&self, item: MediaItem) -> Candidate {
        let watched_at = match &self.stats {
            Some(s) => s
                .last_watched(&item.raw.ids, item.arr.kind())
                .await
                .unwrap_or(None),
            None => None,
        };

        Candidate {
            recency: watched_at.unwrap_or(item.raw.added),
            item,
        }
    }
}
