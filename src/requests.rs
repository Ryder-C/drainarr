pub mod seerr;

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::{ExternalIds, MediaKind};

#[async_trait]
pub trait RequestService: Send + Sync {
    async fn clear_request(&self, ids: &ExternalIds, kind: MediaKind) -> Result<()>;
}
