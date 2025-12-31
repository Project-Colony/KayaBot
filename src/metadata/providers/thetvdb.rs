use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;

use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::MetadataProvider;

#[derive(Debug, Clone)]
pub struct TheTvDbClient {
    api_key: String,
    base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TheTvDbSeries {
    #[serde(alias = "tvdb_id")]
    id: String,
    name: String,
    #[serde(default)]
    year: Option<u16>,
    #[serde(default, alias = "score")]
    score: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct TheTvDbEpisode {
    id: String,
    #[serde(rename = "season", alias = "seasonNumber", alias = "season_number")]
    season: u32,
    #[serde(rename = "episode", alias = "number", alias = "episodeNumber")]
    episode: u32,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TheTvDbMovieDetails {
    #[serde(alias = "movie_id")]
    id: String,
    #[serde(alias = "name")]
    title: String,
    #[serde(default)]
    year: Option<u16>,
}

impl TheTvDbClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.thetvdb.com".to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn auth_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.api_key),
        );
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
                "TheTVDB API key is not configured.".to_string(),
            ));
        }
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.api_key)
                .parse()
                .map_err(|err| {
                    MetadataError::Other(format!("Invalid TheTVDB auth header: {err}"))
                })?,
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "application/json".parse().map_err(|err| {
                MetadataError::Other(format!("Invalid TheTVDB accept header: {err}"))
            })?,
        );
        Ok(headers)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, MetadataError> {
        let client = self.http_client()?;
        let url = format!("{}{}", self.base_url, path);
        let response = client
            .get(url)
            .headers(self.headers()?)
            .query(query)
            .send()
            .map_err(|err| {
                if err.is_timeout() {
                    MetadataError::Network("TheTVDB request timed out.".to_string())
                } else {
                    MetadataError::Network(format!("TheTVDB request failed: {err}"))
                }
            })?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(MetadataError::NotFound(
                "TheTVDB did not return any results.".to_string(),
            ));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(MetadataError::RateLimited(
                "TheTVDB rate limit exceeded.".to_string(),
            ));
        }
        if !status.is_success() {
            return Err(MetadataError::InvalidResponse(format!(
                "TheTVDB returned status {status}."
            )));
        }
        response.json::<T>().map_err(|err| {
            MetadataError::InvalidResponse(format!("Failed to parse TheTVDB response: {err}"))
        })
    }

    fn search_series(&self, query: &str) -> Result<Vec<TheTvDbSeries>, MetadataError> {
        #[derive(Deserialize)]
        struct SearchResponse {
            data: Option<Vec<TheTvDbSeries>>,
        }
        let response: SearchResponse =
            self.get_json("/search", &[("query", query), ("type", "series")])?;
        let results = response.data.unwrap_or_default();
        if results.is_empty() {
            return Err(MetadataError::NotFound(
                "TheTVDB returned no matches.".to_string(),
            ));
        }
        Ok(results)
    }

    fn fetch_episodes(&self, title_id: &str) -> Result<Vec<TheTvDbEpisode>, MetadataError> {
        #[derive(Deserialize)]
        struct EpisodeResponse {
            data: Option<Vec<TheTvDbEpisode>>,
        }
        let response: EpisodeResponse =
            self.get_json(&format!("/series/{title_id}/episodes"), &[])?;
        let episodes = response.data.unwrap_or_default();
        if episodes.is_empty() {
            return Err(MetadataError::NotFound(
                "TheTVDB returned no episodes.".to_string(),
            ));
        }
        Ok(episodes)
    }

    fn fetch_movie(&self, title_id: &str) -> Result<TheTvDbMovieDetails, MetadataError> {
        #[derive(Deserialize)]
        struct MovieResponse {
            data: Option<TheTvDbMovieDetails>,
        }
        let response: MovieResponse = self.get_json(&format!("/movies/{title_id}"), &[])?;
        response.data.ok_or_else(|| {
            MetadataError::NotFound("TheTVDB returned no movie details.".to_string())
        })
    }

    fn normalize_title(&self, series: TheTvDbSeries) -> TitleMatch {
        let source_score = series.score;
        let source_trust = 0.9;
        let global_score = source_score * source_trust;
        TitleMatch {
            id: series.id.clone(),
            name: series.name,
            year: series.year,
            source_score,
            source_trust,
            global_score,
            source: "TheTVDB".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    tvdb: Some(series.id),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn normalize_episode(&self, episode: TheTvDbEpisode) -> EpisodeMatch {
        let source_score = 1.0;
        let source_trust = 0.9;
        let global_score = source_score * source_trust;
        EpisodeMatch {
            id: episode.id.clone(),
            season: episode.season,
            episode: episode.episode,
            title: episode.name,
            source_score,
            source_trust,
            global_score,
            source: "TheTVDB".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    tvdb: Some(episode.id),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn normalize_movie(&self, details: TheTvDbMovieDetails) -> MovieMatch {
        let source_score = 1.0;
        let source_trust = 0.9;
        let global_score = source_score * source_trust;
        MovieMatch {
            id: details.id.clone(),
            title: details.title,
            year: details.year,
            source_score,
            source_trust,
            global_score,
            source: "TheTVDB".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    tvdb: Some(details.id),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

}

impl MetadataProvider for TheTvDbClient {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
        if query.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Search query cannot be empty.".to_string(),
            ));
        }
        let results = self.search_series(query)?;
        Ok(results
            .into_iter()
            .map(|series| self.normalize_title(series))
            .collect())
    }

    fn fetch_episode_list(&mut self, title_id: &str) -> Result<Vec<EpisodeMatch>, MetadataError> {
        if title_id.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Title identifier cannot be empty.".to_string(),
            ));
        }
        let episodes = self.fetch_episodes(title_id)?;
        Ok(episodes
            .into_iter()
            .map(|episode| self.normalize_episode(episode))
            .collect())
    }

    fn fetch_movie_details(&mut self, title_id: &str) -> Result<MovieMatch, MetadataError> {
        if title_id.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Title identifier cannot be empty.".to_string(),
            ));
        }
        let details = self.fetch_movie(title_id)?;
        Ok(self.normalize_movie(details))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_title_from_fixture() {
        let payload = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/thetvdb_series.json"
        ));
        let series: TheTvDbSeries = serde_json::from_str(payload).expect("fixture should parse");
        let client = TheTvDbClient::new("test-key");
        let normalized = client.normalize_title(series);

        assert_eq!(normalized.id, "tvdb-99");
        assert_eq!(normalized.name, "Example Series");
        assert_eq!(normalized.year, Some(2010));
        assert_eq!(normalized.source, "TheTVDB");
        assert_eq!(
            normalized.extras.external_ids.tvdb.as_deref(),
            Some("tvdb-99")
        );
    }
}
