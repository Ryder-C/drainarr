use anyhow::Result;
use std::sync::Arc;

use crate::{arr::ArrInstance, domain::MediaItem};

const IGNORE_TAG: &str = "drainarr_ignore";

struct CandidateCollector {
    pub instances: Vec<Arc<dyn ArrInstance>>,
}

impl CandidateCollector {
    async fn collect(&self) -> Result<Vec<MediaItem>> {
        todo!()
    }
}
