use std::sync::Arc;

use futures::{StreamExt, stream};

use crate::{
    domain::{Candidate, MediaItem},
    stats::StatsProvider,
};

const STATS_CONCURRENCY: usize = 16;

pub struct RecencyResolver {
    pub stats: Option<Arc<dyn StatsProvider>>,
}

impl RecencyResolver {
    pub async fn resolve(&self, items: Vec<MediaItem>) -> Vec<Candidate> {
        stream::iter(items)
            .map(|item| self.resolve_one(item))
            .buffer_unordered(STATS_CONCURRENCY)
            .collect()
            .await
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
