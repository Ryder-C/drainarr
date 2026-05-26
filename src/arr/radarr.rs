use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::warn;

use crate::{
    arr::{ArrHttp, ArrInstance, TagMap},
    domain::{ExternalIds, MediaKind, RawItem},
};

struct RadarrClient {
    pub api: ArrHttp,
}

#[async_trait]
impl ArrInstance for RadarrClient {
    fn label(&self) -> &str {
        &self.api.label
    }

    fn kind(&self) -> MediaKind {
        MediaKind::Movie
    }

    async fn list(&self) -> Result<Vec<RawItem>> {
        let tags = self.api.tag_map().await?;

        let movies: Vec<Movie> = self
            .api
            .get("movie", &[])
            .await?
            .json()
            .await
            .context("parsing Radarr /movie")?;

        let items = movies
            .into_iter()
            .map(|m| m.into_raw(&tags))
            .filter(|r| r.size_bytes > 0)
            .collect();

        Ok(items)
    }

    async fn delete(&self, arr_id: u64, season: Option<u32>) -> Result<()> {
        if let Some(s) = season {
            warn!(
                "Radarr movie id {arr_id} requested deletion with season number {s}. Ignoring season and deleting whole movie."
            );
        }

        self.api
            .delete::<()>(
                &format!("movie/{arr_id}"),
                &[("deleteFiles", "true".to_string())],
                None,
            )
            .await
            .context(format!("deleting Radarr movie id {arr_id}"))?;

        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MovieFile {
    date_added: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Movie {
    id: u64,
    title: String,
    #[serde(default)]
    size_on_disk: u64,
    tmdb_id: u32,
    imdb_id: Option<String>,
    added: Option<DateTime<Utc>>,
    tags: Vec<u32>,
    movie_file: Option<MovieFile>,
}

impl Movie {
    /// Converts a Radarr Movie to a RawItem, using the tag map to convert tag ids to labels.
    /// Prefer when the file was imported, fall back to when added, and if somehow neither exist
    /// treat as new.
    fn into_raw(self, tags: &TagMap) -> RawItem {
        let added = self
            .movie_file
            .and_then(|f| f.date_added)
            .or(self.added)
            .unwrap_or(Utc::now());
        RawItem {
            arr_id: self.id,
            season: None,
            title: self.title,
            size_bytes: self.size_on_disk,
            added,
            ids: ExternalIds {
                imdb: self.imdb_id,
                tmdb: Some(self.tmdb_id),
                tvdb: None,
            },
            tags: self
                .tags
                .iter()
                .filter_map(|id| tags.get(id).cloned())
                .collect(),
        }
    }
}
