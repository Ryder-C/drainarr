use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use url::Url;

use crate::{
    domain::{ExternalIds, MediaKind},
    stats::StatsProvider,
};

pub struct JanitorrStats {
    pub base_url: Url,
    pub http: reqwest::Client,
}

impl JanitorrStats {
    /// Page through an endpoint and return the most recent play time.
    async fn latest_play(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<Option<DateTime<Utc>>> {
        let mut latest = None;
        let mut page = 0;
        loop {
            let history = self.fetch_page(path, query, page).await?;
            for record in history.content {
                if latest.is_none_or(|l| record.played_at > l) {
                    latest = Some(record.played_at);
                }
            }
            if page + 1 >= history.total_pages {
                break;
            }
            page += 1;
        }

        Ok(latest)
    }

    /// GET one page of history
    async fn fetch_page(
        &self,
        path: &str,
        query: &[(&str, String)],
        page: u32,
    ) -> Result<HistoryPage> {
        let url = self.base_url.join(path).expect("valid path");

        self.http
            .get(url)
            .query(query)
            .query(&[("page", page)])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("failed to parse history page")
    }

    fn movie_query(ids: &ExternalIds) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        if let Some(imdb) = &ids.imdb {
            q.push(("imdbId", imdb.clone()));
        }
        if let Some(tmdb) = ids.tmdb {
            q.push(("tmdbId", tmdb.to_string()));
        }
        q
    }

    fn series_query(ids: &ExternalIds) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        if let Some(imdb) = &ids.imdb {
            q.push(("imdbId", imdb.clone()));
        }
        if let Some(tmdb) = ids.tmdb {
            q.push(("tmdbId", tmdb.to_string()));
        }
        if let Some(tvdb) = ids.tvdb {
            q.push(("tvdbId", tvdb.to_string()));
        }
        q
    }
}

#[async_trait]
impl StatsProvider for JanitorrStats {
    async fn last_watched(
        &self,
        ids: &ExternalIds,
        kind: MediaKind,
    ) -> Result<Option<DateTime<Utc>>> {
        let (path, query) = match kind {
            MediaKind::Movie => ("history/movies", Self::movie_query(ids)),
            MediaKind::Series => ("history/shows", Self::series_query(ids)),
        };

        // If there are somehow no usable ids treat as unwatched
        if query.is_empty() {
            return Ok(None);
        }

        self.latest_play(path, &query).await
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryPage {
    content: Vec<PlayRecord>,
    total_pages: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayRecord {
    played_at: DateTime<Utc>,
}
