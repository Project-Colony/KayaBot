#![allow(dead_code)]

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde::de::{self, Deserializer};

use crate::metadata::error::MetadataError;
use crate::metadata::locale::MetadataLocale;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::MetadataProvider;

#[derive(Debug, Clone)]
pub struct TheTvDbClient {
    api_key: String,
    base_url: String,
    token: Option<TheTvDbToken>,
    locale: Option<MetadataLocale>,
}

#[derive(Debug, Clone)]
struct TheTvDbToken {
    value: String,
    expires_at: Option<Instant>,
}

#[derive(Debug, Clone, Deserialize)]
struct TheTvDbSeries {
    #[serde(
        alias = "tvdb_id",
        alias = "id",
        deserialize_with = "deserialize_string"
    )]
    id: String,
    name: String,
    #[serde(default)]
    year: Option<u16>,
    #[serde(default, alias = "score")]
    score: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct TheTvDbEpisode {
    #[serde(deserialize_with = "deserialize_string")]
    id: String,
    #[serde(rename = "season", alias = "seasonNumber", alias = "season_number")]
    season: u32,
    #[serde(rename = "episode", alias = "number", alias = "episodeNumber")]
    episode: u32,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TheTvDbMovieDetails {
    #[serde(
        alias = "movie_id",
        alias = "id",
        deserialize_with = "deserialize_string"
    )]
    id: String,
    #[serde(alias = "name")]
    title: String,
    #[serde(default)]
    year: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
struct TheTvDbTokenResponse {
    data: TheTvDbTokenData,
}

#[derive(Debug, Clone, Deserialize)]
struct TheTvDbTokenData {
    token: String,
}

impl TheTvDbClient {
    pub fn new(api_key: impl Into<String>, locale: Option<MetadataLocale>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api4.thetvdb.com/v4".to_string(),
            token: None,
            locale,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn auth_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        if let Some(token) = &self.token {
            headers.insert(
                "Authorization".to_string(),
                format!("Bearer {}", token.value),
            );
        }
        headers.insert("Accept".to_string(), "application/json".to_string());
        if let Some(language) = self.accept_language() {
            headers.insert("Accept-Language".to_string(), language);
        }
        headers
    }

    fn http_client(&self) -> Result<reqwest::blocking::Client, MetadataError> {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|err| MetadataError::Network(format!("Failed to build HTTP client: {err}")))
    }

    fn headers_for_token(&self, token: &str) -> Result<reqwest::header::HeaderMap, MetadataError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().map_err(|err| {
                MetadataError::Other(format!("Invalid TheTVDB auth header: {err}"))
            })?,
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "application/json".parse().map_err(|err| {
                MetadataError::Other(format!("Invalid TheTVDB accept header: {err}"))
            })?,
        );
        if let Some(language) = self.accept_language() {
            headers.insert(
                reqwest::header::ACCEPT_LANGUAGE,
                language.parse().map_err(|err| {
                    MetadataError::Other(format!("Invalid TheTVDB language header: {err}"))
                })?,
            );
        }
        Ok(headers)
    }

    fn accept_language(&self) -> Option<String> {
        let locale = self.locale.as_ref()?;
        let language = locale.language.as_deref()?;
        let region = locale.region.as_deref();
        Some(match region {
            Some(region) => format!("{language}-{region}"),
            None => language.to_string(),
        })
    }

    fn build_query(&self, query: &[(String, String)]) -> Vec<(String, String)> {
        let mut merged = query.to_vec();
        if let Some(locale) = &self.locale {
            if let Some(language) = locale.language.as_deref() {
                merged.push(("language".to_string(), language.to_string()));
            }
            if let Some(region) = locale.region.as_deref() {
                merged.push(("country".to_string(), region.to_string()));
            }
        }
        merged
    }

    fn authenticate(&self) -> Result<TheTvDbToken, MetadataError> {
        if self.api_key.trim().is_empty() {
            return Err(MetadataError::Other(
                "TheTVDB API key is not configured.".to_string(),
            ));
        }
        #[derive(serde::Serialize)]
        struct LoginRequest<'a> {
            apikey: &'a str,
        }
        let client = self.http_client()?;
        let url = format!("{}/login", self.base_url);
        let response = client
            .post(url)
            .json(&LoginRequest {
                apikey: self.api_key.as_str(),
            })
            .send()
            .map_err(|err| super::request_error("TheTVDB auth", err))?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(MetadataError::Other(
                "TheTVDB authorization failed. Check the configured API key.".to_string(),
            ));
        }
        if !status.is_success() {
            return Err(MetadataError::InvalidResponse(format!(
                "TheTVDB auth returned status {status}."
            )));
        }
        let payload = response
            .json::<TheTvDbTokenResponse>()
            .map_err(|err| super::parse_error("TheTVDB auth", err))?;
        Ok(TheTvDbToken {
            value: payload.data.token,
            expires_at: Some(Instant::now() + Duration::from_secs(23 * 60 * 60)),
        })
    }

    fn ensure_token(&mut self) -> Result<(), MetadataError> {
        let needs_refresh = self
            .token
            .as_ref()
            .and_then(|token| token.expires_at)
            .map(|expires_at| Instant::now() >= expires_at)
            .unwrap_or(true);
        if needs_refresh {
            self.token = Some(self.authenticate()?);
        }
        Ok(())
    }

    fn refresh_token(&mut self) -> Result<(), MetadataError> {
        if self.token.is_none() {
            return self.ensure_token();
        }
        let token_value = self
            .token
            .as_ref()
            .map(|token| token.value.clone())
            .unwrap_or_default();
        let client = self.http_client()?;
        let url = format!("{}/refresh_token", self.base_url);
        let response = client
            .get(url)
            .headers(self.headers_for_token(&token_value)?)
            .send()
            .map_err(|err| super::request_error("TheTVDB refresh", err))?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            self.token = Some(self.authenticate()?);
            return Ok(());
        }
        if !status.is_success() {
            return Err(MetadataError::InvalidResponse(format!(
                "TheTVDB refresh returned status {status}."
            )));
        }
        let payload = response
            .json::<TheTvDbTokenResponse>()
            .map_err(|err| super::parse_error("TheTVDB refresh", err))?;
        self.token = Some(TheTvDbToken {
            value: payload.data.token,
            expires_at: Some(Instant::now() + Duration::from_secs(23 * 60 * 60)),
        });
        Ok(())
    }

    fn get_json<T: for<'de> Deserialize<'de>>(
        &mut self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<T, MetadataError> {
        self.get_json_with_retry(path, query, true)
    }

    fn get_json_with_retry<T: for<'de> Deserialize<'de>>(
        &mut self,
        path: &str,
        query: &[(String, String)],
        allow_retry: bool,
    ) -> Result<T, MetadataError> {
        self.ensure_token()?;
        let token_value = self
            .token
            .as_ref()
            .map(|token| token.value.clone())
            .unwrap_or_default();
        let client = self.http_client()?;
        let url = format!("{}{}", self.base_url, path);
        let response = client
            .get(url)
            .headers(self.headers_for_token(&token_value)?)
            .query(&self.build_query(query))
            .send()
            .map_err(|err| super::request_error("TheTVDB", err))?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            if allow_retry {
                self.refresh_token()?;
                return self.get_json_with_retry(path, query, false);
            }
            return Err(MetadataError::Other(
                "TheTVDB authorization failed after refresh.".to_string(),
            ));
        }
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
        response
            .json::<T>()
            .map_err(|err| super::parse_error("TheTVDB", err))
    }

    fn search_series(&mut self, query: &str) -> Result<Vec<TheTvDbSeries>, MetadataError> {
        #[derive(Deserialize)]
        struct SearchResponse {
            data: Option<Vec<TheTvDbSeries>>,
        }
        let response: SearchResponse = self.get_json(
            "/search",
            &[
                ("query".to_string(), query.to_string()),
                ("type".to_string(), "series".to_string()),
            ],
        )?;
        let results = response.data.unwrap_or_default();
        if results.is_empty() {
            return Err(MetadataError::NotFound(
                "TheTVDB returned no matches.".to_string(),
            ));
        }
        Ok(results)
    }

    fn fetch_episodes(&mut self, title_id: &str) -> Result<Vec<TheTvDbEpisode>, MetadataError> {
        #[derive(Deserialize)]
        struct EpisodeResponse {
            data: Option<EpisodePayload>,
        }

        #[derive(Deserialize)]
        struct EpisodePayload {
            #[serde(default)]
            episodes: Vec<TheTvDbEpisode>,
            #[serde(default)]
            links: Option<EpisodeLinks>,
        }

        #[derive(Deserialize)]
        struct EpisodeLinks {
            #[serde(default)]
            next: Option<u32>,
        }

        let mut page = 1;
        let mut episodes = Vec::new();
        loop {
            let response: EpisodeResponse = self.get_json(
                &format!("/series/{title_id}/episodes/default"),
                &[("page".to_string(), page.to_string())],
            )?;
            if let Some(data) = response.data {
                episodes.extend(data.episodes);
                if data.links.and_then(|links| links.next).is_some() {
                    page += 1;
                    continue;
                }
            }
            break;
        }
        if episodes.is_empty() {
            return Err(MetadataError::NotFound(
                "TheTVDB returned no episodes.".to_string(),
            ));
        }
        Ok(episodes)
    }

    fn fetch_movie(&mut self, title_id: &str) -> Result<TheTvDbMovieDetails, MetadataError> {
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

fn deserialize_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(value) => Ok(value),
        serde_json::Value::Number(number) => Ok(number.to_string()),
        serde_json::Value::Bool(value) => Ok(value.to_string()),
        _ => Err(de::Error::custom("Expected string-like value")),
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
        let client = TheTvDbClient::new("test-key", None);
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

    #[test]
    fn normalize_title_falls_back_when_translation_missing() {
        let payload = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/thetvdb_series_missing_translation.json"
        ));
        let series: TheTvDbSeries = serde_json::from_str(payload).expect("fixture should parse");
        let client = TheTvDbClient::new("test-key", None);
        let normalized = client.normalize_title(series);

        assert_eq!(normalized.id, "tvdb-100");
        assert_eq!(normalized.name, "Original Series");
        assert_eq!(normalized.year, Some(2015));
    }
}
