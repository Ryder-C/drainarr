use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    arr::{ArrHttp, ArrInstance, TagMap},
    domain::{ExternalIds, MediaKind, RawItem},
};

struct SonarrClient {
    pub api: ArrHttp,
}

impl SonarrClient {
    async fn list_series(&self) -> Result<Vec<Series>> {
        self.api
            .get("series", &[])
            .await?
            .json()
            .await
            .context("parsing Sonarr /series")
    }

    async fn get_series(&self, series_id: u64) -> Result<Series> {
        self.api
            .get(&format!("series/{series_id}"), &[])
            .await?
            .json()
            .await
            .context(format!("parsing Sonarr /series/{series_id}"))
    }

    async fn delete_series(&self, arr_id: u64) -> Result<()> {
        let _ = self
            .api
            .delete::<()>(
                &format!("series/{arr_id}"),
                &[("deleteFiles", "true".to_string())],
                None,
            )
            .await
            .context(format!("deleting Sonarr series id {arr_id}"))?;
        Ok(())
    }

    async fn delete_season(&self, arr_id: u64, season: u32) -> Result<()> {
        let files = self.episode_files(arr_id).await?;
        let (in_season, others): (Vec<_>, Vec<_>) =
            files.into_iter().partition(|f| f.season_number == season);

        if in_season.is_empty() {
            return Ok(());
        }
        if others.is_empty() {
            return self.delete_series(arr_id).await;
        }

        self.unmonitor_season(arr_id, season).await?;
        self.delete_episode_files(in_season.into_iter().map(|f| f.id).collect())
            .await
    }

    async fn episode_files(&self, series_id: u64) -> Result<Vec<EpisodeFile>> {
        self.api
            .get("episodefile", &[("seriesId", series_id.to_string())])
            .await?
            .json()
            .await
            .context(format!(
                "parsing Sonarr /episodefile for series id {series_id}"
            ))
    }

    async fn delete_episode_files(&self, ids: Vec<u64>) -> Result<()> {
        let _ = self
            .api
            .delete(
                "episodefile/bulk",
                &[],
                Some(&json!({ "episodeFileIds": ids })),
            )
            .await
            .context("bulk-deleting episode files")?;

        Ok(())
    }

    async fn unmonitor_season(&self, series_id: u64, season: u32) -> Result<()> {
        let mut series = self.get_series(series_id).await?;

        series
            .seasons
            .iter_mut()
            .find(|s| s.season_number == season)
            .map(|s| {
                s.monitored = false;
            });

        self.api
            .put(&format!("series/{series_id}"), &series)
            .await
            .context("unmonitoring season")?;

        Ok(())
    }
}

#[async_trait]
impl ArrInstance for SonarrClient {
    fn label(&self) -> &str {
        &self.api.label
    }

    fn kind(&self) -> MediaKind {
        MediaKind::Series
    }

    async fn list(&self) -> Result<Vec<RawItem>> {
        let tags = self.api.tag_map().await?;

        let series = self.list_series().await?;

        let items = series
            .into_iter()
            .flat_map(|s| s.into_raw(&tags))
            .filter(|r| r.size_bytes > 0)
            .collect();

        Ok(items)
    }

    async fn delete(&self, arr_id: u64, season: Option<u32>) -> Result<()> {
        match season {
            Some(s) => self.delete_season(arr_id, s).await,
            None => self.delete_series(arr_id).await,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Series {
    id: u64,
    title: String,
    seasons: Vec<Season>,
    tvdb_id: u32,
    tmdb_id: Option<u32>,
    added: DateTime<Utc>,
    tags: Vec<u32>,
}

impl Series {
    fn into_raw(self, tags: &TagMap) -> Vec<RawItem> {
        self.seasons
            .into_iter()
            .filter_map(|s| {
                let size_bytes = s.statistics.size_on_disk;
                let episode_count = s.statistics.episode_file_count;

                // Skip seasons with no files
                if episode_count == 0 {
                    return None;
                }

                let ids = ExternalIds {
                    imdb: None,
                    tmdb: self.tmdb_id,
                    tvdb: Some(self.tvdb_id),
                };

                Some(RawItem {
                    arr_id: self.id,
                    season: Some(s.season_number),
                    title: self.title.clone(),
                    size_bytes,
                    added: self.added,
                    ids,
                    tags: self
                        .tags
                        .iter()
                        .filter_map(|id| tags.get(id).cloned())
                        .collect(),
                })
            })
            .collect()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Season {
    season_number: u32,
    monitored: bool,
    #[serde(default)]
    statistics: SeasonStats,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SeasonStats {
    size_on_disk: u64,
    episode_file_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EpisodeFile {
    id: u64,
    season_number: u32,
}
