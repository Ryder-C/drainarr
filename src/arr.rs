pub mod radarr;
pub mod sonarr;

use std::collections::HashMap;

use crate::domain::{MediaKind, RawItem};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::{Method, Response};
use serde::{Deserialize, Serialize};
use url::Url;

type TagMap = HashMap<u32, String>;

#[derive(Deserialize)]
struct Tag {
    id: u32,
    label: String,
}

pub struct ArrHttp {
    pub label: String,
    pub base_url: Url,
    pub api_key: String,
    pub http: reqwest::Client,
}

impl ArrHttp {
    fn url(&self, path: &str) -> Url {
        self.base_url
            .join(&format!("api/v3/{}", path))
            .expect("valid url path")
    }

    async fn request<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<Response> {
        let mut req = self
            .http
            .request(method, self.url(path))
            .header("X-Api-Key", self.api_key.as_str())
            .query(query);
        if let Some(b) = body {
            req = req.json(b);
        }
        req.send().await?.error_for_status().map_err(Into::into)
    }

    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Response> {
        self.request::<()>(Method::GET, path, query, None).await
    }

    async fn delete<B: Serialize>(
        &self,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<Response> {
        self.request(Method::DELETE, path, query, body).await
    }

    async fn put<B: Serialize>(&self, path: &str, body: &B) -> Result<Response> {
        self.request(Method::PUT, path, &[], Some(body)).await
    }

    /// Fetches all tags to convert tag ids to labels
    async fn tag_map(&self) -> Result<TagMap> {
        let tags: Vec<Tag> = self
            .get("tag", &[])
            .await?
            .json()
            .await
            .context("parsing arr /tag")?;

        Ok(tags.into_iter().map(|t| (t.id, t.label)).collect())
    }
}

#[async_trait]
pub trait ArrInstance: Send + Sync {
    /// Human-readable name for logging
    fn label(&self) -> &str;

    /// Movie (Radarr) or Series (Sonarr)
    fn kind(&self) -> MediaKind;

    /// Every item with files on disk
    async fn list(&self) -> Result<Vec<RawItem>>;

    /// Delete one item by its *arr id
    async fn delete(&self, arr_id: u64, season: Option<u32>) -> Result<()>;
}
