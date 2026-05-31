use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::{Method, Response};
use serde::Deserialize;
use url::Url;

use crate::{
    domain::{ExternalIds, MediaKind},
    requests::RequestService,
};

pub struct SeerrClient {
    pub base_url: Url,
    pub api_key: String,
    pub http: reqwest::Client,
}

impl SeerrClient {
    fn url(&self, path: &str) -> Url {
        self.base_url
            .join(&format!("api/v1/{path}"))
            .expect("valid path")
    }

    async fn request(&self, method: Method, path: &str) -> Result<Response> {
        self.http
            .request(method, self.url(path))
            .send()
            .await?
            .error_for_status()
            .context("request failed")
    }

    async fn get(&self, path: &str) -> Result<Response> {
        self.request(Method::GET, path).await
    }

    async fn delete(&self, path: &str) -> Result<Response> {
        self.request(Method::DELETE, path).await
    }

    async fn lookup(&self, tmdb_id: u32, kind: MediaKind) -> Result<MediaInfo> {
        let details = match kind {
            MediaKind::Movie => self.movie_details(tmdb_id).await?,
            MediaKind::Series => self.tv_details(tmdb_id).await?,
        };

        Ok(details.media_info)
    }

    async fn movie_details(&self, tmdb_id: u32) -> Result<MediaDetails> {
        self.get(&format!("movie/{tmdb_id}"))
            .await?
            .json()
            .await
            .context("failed to parse movie details")
    }

    async fn tv_details(&self, tmdb_id: u32) -> Result<MediaDetails> {
        self.get(&format!("tv/{tmdb_id}"))
            .await?
            .json()
            .await
            .context("failed to parse tv details")
    }

    async fn delete_request(&self, id: u32) -> Result<()> {
        let _ = self
            .delete(&format!("request/{id}"))
            .await
            .context(format!("deleting seerr request {id}"))?;
        Ok(())
    }

    fn request_matches(req: &MediaRequest, season: Option<u32>) -> bool {
        match season {
            None => true,
            Some(s) => {
                !req.seasons.is_empty() && req.seasons.iter().any(|si| si.season_number == s)
            }
        }
    }
}

#[async_trait]
impl RequestService for SeerrClient {
    async fn clear_request(
        &self,
        ids: &ExternalIds,
        kind: MediaKind,
        season: Option<u32>,
    ) -> Result<()> {
        // Seerr only supports looking up by tmdb so just ignore any media without it
        let Some(tmdb_id) = ids.tmdb else {
            return Ok(());
        };

        let info = self.lookup(tmdb_id, kind).await?;

        // Clear requests
        for req in info.requests {
            if Self::request_matches(&req, season) {
                self.delete_request(req.id).await?;
            }
        }

        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaDetails {
    media_info: MediaInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaInfo {
    requests: Vec<MediaRequest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaRequest {
    id: u32,
    seasons: Vec<SeasonInfo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeasonInfo {
    id: u32,
    season_number: u32,
}
