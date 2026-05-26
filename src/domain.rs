use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::arr::ArrInstance;

pub enum MediaKind {
    Movie,
    Series,
}

pub struct ExternalIds {
    pub imdb: Option<String>,
    pub tmdb: Option<u32>,
    pub tvdb: Option<u32>,
}

pub struct RawItem {
    pub arr_id: u64,
    pub season: Option<u32>,
    pub title: String,
    pub size_bytes: u64,
    pub added: DateTime<Utc>,
    pub ids: ExternalIds,
    pub tags: Vec<String>,
}

pub struct MediaItem {
    pub arr: Arc<dyn ArrInstance>,
    pub raw: RawItem,
}

pub struct Candidate {
    pub item: MediaItem,
    pub recency: DateTime<Utc>,
}
