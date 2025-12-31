#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::MetadataProvider;

#[derive(Debug, Clone)]
pub struct AniDbClient {
    api_key: String,
    base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AniDbTitle {
    id: u32,
    title: String,
    year: Option<u16>,
    rating: f32,
    #[serde(default)]
    aliases: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AniDbEpisode {
    id: u32,
    number: u32,
    title: String,
    #[serde(default)]
    season: Option<u32>,
}

impl AniDbClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.anidb.net".to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn auth_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("X-AniDB-API-Key".to_string(), self.api_key.clone());
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers
    }

    fn http_client(&self) -> Result<reqwest::blocking::Client, MetadataError> {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|err| MetadataError::Network(format!("Failed to build HTTP client: {err}")))
    }

    fn headers(&self) -> Result<reqwest::header::HeaderMap, MetadataError> {
        if self.api_key.trim().is_empty() {
            return Err(MetadataError::Other(
                "AniDB API key is not configured.".to_string(),
            ));
        }
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            "application/json".parse().map_err(|err| {
                MetadataError::Other(format!("Invalid AniDB accept header: {err}"))
            })?,
        );
        headers.insert(
            "X-AniDB-API-Key",
            self.api_key.parse().map_err(|err| {
                MetadataError::Other(format!("Invalid AniDB API key header: {err}"))
            })?,
        );
        Ok(headers)
    }

    fn get_json(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, MetadataError> {
        let client = self.http_client()?;
        let url = format!("{}{}", self.base_url, path);
        let response = client
            .get(url)
            .headers(self.headers()?)
            .query(query)
            .send()
            .map_err(|err| {
                if err.is_timeout() {
                    MetadataError::Network("AniDB request timed out.".to_string())
                } else {
                    MetadataError::Network(format!("AniDB request failed: {err}"))
                }
            })?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(MetadataError::NotFound(
                "AniDB did not return any results.".to_string(),
            ));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(MetadataError::RateLimited(
                "AniDB rate limit exceeded.".to_string(),
            ));
        }
        if !status.is_success() {
            return Err(MetadataError::InvalidResponse(format!(
                "AniDB returned status {status}."
            )));
        }
        response.json::<Value>().map_err(|err| {
            MetadataError::InvalidResponse(format!("Failed to parse AniDB response: {err}"))
        })
    }

    fn normalize_title(&self, title: AniDbTitle) -> TitleMatch {
        let source_score = title.rating;
        let source_trust = 0.7;
        let global_score = source_score * source_trust;
        TitleMatch {
            id: title.id.to_string(),
            name: title.title,
            year: title.year,
            source_score,
            source_trust,
            global_score,
            source: "AniDB".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                aliases: title.aliases,
                external_ids: crate::metadata::models::ExternalIds {
                    anidb: Some(title.id.to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn normalize_episode(&self, episode: AniDbEpisode) -> EpisodeMatch {
        let source_score = 1.0;
        let source_trust = 0.7;
        let global_score = source_score * source_trust;
        EpisodeMatch {
            id: episode.id.to_string(),
            season: episode.season.unwrap_or(1),
            episode: episode.number,
            title: episode.title,
            source_score,
            source_trust,
            global_score,
            source: "AniDB".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    anidb: Some(episode.id.to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn parse_title_list(&self, payload: Value) -> Result<Vec<AniDbTitle>, MetadataError> {
        let candidates = payload
            .get("data")
            .or_else(|| payload.get("results"))
            .or_else(|| payload.get("anime"))
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                MetadataError::InvalidResponse(
                    "AniDB response did not contain a title list.".to_string(),
                )
            })?;

        let mut titles = Vec::new();
        for entry in candidates {
            let Some(id) = entry.get("id").and_then(Value::as_u64) else {
                continue;
            };
            let name = entry
                .get("title")
                .and_then(Value::as_str)
                .or_else(|| entry.get("name").and_then(Value::as_str))
                .unwrap_or_default()
                .to_string();
            if name.trim().is_empty() {
                continue;
            }
            let year = entry
                .get("year")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok());
            let rating = entry
                .get("rating")
                .and_then(Value::as_f64)
                .unwrap_or(0.5) as f32;
            let aliases = entry
                .get("aliases")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|value| value.as_str().map(|alias| alias.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            titles.push(AniDbTitle {
                id: id as u32,
                title: name,
                year,
                rating,
                aliases,
            });
        }

        if titles.is_empty() {
            return Err(MetadataError::NotFound(
                "AniDB returned no matches.".to_string(),
            ));
        }

        Ok(titles)
    }

    fn parse_episode_list(&self, payload: Value) -> Result<Vec<AniDbEpisode>, MetadataError> {
        let candidates = payload
            .get("data")
            .or_else(|| payload.get("episodes"))
            .or_else(|| payload.get("list"))
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                MetadataError::InvalidResponse(
                    "AniDB response did not contain an episode list.".to_string(),
                )
            })?;

        let mut episodes = Vec::new();
        for entry in candidates {
            let Some(id) = entry.get("id").and_then(Value::as_u64) else {
                continue;
            };
            let number = entry
                .get("number")
                .or_else(|| entry.get("episode"))
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(0);
            if number == 0 {
                continue;
            }
            let season = entry
                .get("season")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok());
            let title = entry
                .get("title")
                .and_then(Value::as_str)
                .or_else(|| entry.get("name").and_then(Value::as_str))
                .unwrap_or("Episode")
                .to_string();

            episodes.push(AniDbEpisode {
                id: id as u32,
                number,
                title,
                season,
            });
        }

        if episodes.is_empty() {
            return Err(MetadataError::NotFound(
                "AniDB returned no episodes.".to_string(),
            ));
        }

        Ok(episodes)
    }

    fn search_titles(&self, query: &str) -> Result<Vec<AniDbTitle>, MetadataError> {
        let payload = self.get_json("/v1/anime/search", &[("q", query)])?;
        self.parse_title_list(payload)
    }

    fn fetch_episodes(&self, title_id: &str) -> Result<Vec<AniDbEpisode>, MetadataError> {
        let payload = self.get_json(&format!("/v1/anime/{title_id}/episodes"), &[])?;
        self.parse_episode_list(payload)
    }

    fn is_timeout_error(error: &MetadataError) -> bool {
        matches!(error, MetadataError::Network(message) if message.contains("timed out"))
    }
}

impl MetadataProvider for AniDbClient {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
        if query.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Search query cannot be empty.".to_string(),
            ));
        }
        match self.search_titles(query) {
            Ok(results) => Ok(results
                .into_iter()
                .map(|title| self.normalize_title(title))
                .collect()),
            Err(error) if Self::is_timeout_error(&error) => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    fn fetch_episode_list(&mut self, title_id: &str) -> Result<Vec<EpisodeMatch>, MetadataError> {
        if title_id.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Title identifier cannot be empty.".to_string(),
            ));
        }
        match self.fetch_episodes(title_id) {
            Ok(episodes) => Ok(episodes
                .into_iter()
                .map(|episode| self.normalize_episode(episode))
                .collect()),
            Err(error) if Self::is_timeout_error(&error) => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    fn fetch_movie_details(&mut self, _title_id: &str) -> Result<MovieMatch, MetadataError> {
        Err(MetadataError::NotFound(
            "AniDB is series-focused and does not provide movie details.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_title_from_fixture() {
        let payload = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/anidb_title.json"
        ));
        let title: AniDbTitle = serde_json::from_str(payload).expect("fixture should parse");
        let client = AniDbClient::new("test-key");
        let normalized = client.normalize_title(title);

        assert_eq!(normalized.id, "42");
        assert_eq!(normalized.name, "Example Anime");
        assert_eq!(normalized.year, Some(2006));
        assert_eq!(normalized.source, "AniDB");
        assert_eq!(
            normalized.extras.external_ids.anidb.as_deref(),
            Some("42")
        );
    }
}
