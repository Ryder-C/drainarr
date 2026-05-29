pub mod janitorr;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{ExternalIds, MediaKind};

#[async_trait]
pub trait StatsProvider: Send + Sync {
    async fn last_watched(
        &self,
        ids: &ExternalIds,
        kind: MediaKind,
    ) -> Result<Option<DateTime<Utc>>>;
}
