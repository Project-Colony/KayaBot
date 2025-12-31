#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;

use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::MetadataProvider;

#[derive(Debug, Clone)]
pub struct OmdbClient {
    api_key: String,
    base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct OmdbSearchItem {
    #[serde(rename = "imdbID", alias = "imdb_id")]
    imdb_id: String,
    #[serde(rename = "Title", alias = "title")]
    title: String,
    #[serde(
        rename = "Year",
        alias = "year",
        default,
        deserialize_with = "deserialize_year"
    )]
    year: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
struct OmdbMovieDetails {
    #[serde(rename = "imdbID", alias = "imdb_id")]
    imdb_id: String,
    #[serde(rename = "Title", alias = "title")]
    title: String,
    #[serde(
        rename = "Year",
        alias = "year",
        default,
        deserialize_with = "deserialize_year"
    )]
    year: Option<u16>,
}

impl OmdbClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://www.omdbapi.com".to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn auth_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("X-OMDb-API-Key".to_string(), self.api_key.clone());
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers
    }

    fn http_client(&self) -> Result<reqwest::blocking::Client, MetadataError> {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|err| MetadataError::Network(format!("Failed to build HTTP client: {err}")))
    }

    fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        query: &[(&str, &str)],
    ) -> Result<T, MetadataError> {
        if self.api_key.trim().is_empty() {
            return Err(MetadataError::Other(
                "OMDb API key is not configured.".to_string(),
            ));
        }
        let client = self.http_client()?;
        let response = client
            .get(&self.base_url)
            .header(reqwest::header::ACCEPT, "application/json")
            .query(query)
            .send()
            .map_err(|err| {
                if err.is_timeout() {
                    MetadataError::Network("OMDb request timed out.".to_string())
                } else {
                    MetadataError::Network(format!("OMDb request failed: {err}"))
                }
            })?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(MetadataError::NotFound(
                "OMDb did not return any results.".to_string(),
            ));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(MetadataError::RateLimited(
                "OMDb rate limit exceeded.".to_string(),
            ));
        }
        if !status.is_success() {
            return Err(MetadataError::InvalidResponse(format!(
                "OMDb returned status {status}."
            )));
        }
        response.json::<T>().map_err(|err| {
            MetadataError::InvalidResponse(format!("Failed to parse OMDb response: {err}"))
        })
    }

    fn search_titles(&self, query: &str) -> Result<Vec<OmdbSearchItem>, MetadataError> {
        #[derive(Deserialize)]
        struct SearchResponse {
            #[serde(default, rename = "Search")]
            search: Vec<OmdbSearchItem>,
            #[serde(default, rename = "Response")]
            response: String,
            #[serde(default, rename = "Error")]
            error: Option<String>,
        }
        let response: SearchResponse = self.get_json(&[
            ("s", query),
            ("apikey", &self.api_key),
        ])?;
        if response.response.to_lowercase() == "false" {
            let message = response
                .error
                .unwrap_or_else(|| "OMDb returned no matches.".to_string());
            return Err(map_omdb_error(&message));
        }
        if response.search.is_empty() {
            return Err(MetadataError::NotFound(
                "OMDb returned no matches.".to_string(),
            ));
        }
        Ok(response.search)
    }

    fn fetch_movie(&self, title_id: &str) -> Result<OmdbMovieDetails, MetadataError> {
        #[derive(Deserialize)]
        struct MovieResponse {
            #[serde(default, rename = "Response")]
            response: String,
            #[serde(default, rename = "Error")]
            error: Option<String>,
            #[serde(flatten)]
            details: Option<OmdbMovieDetails>,
        }
        let response: MovieResponse = self.get_json(&[
            ("i", title_id),
            ("apikey", &self.api_key),
            ("plot", "short"),
        ])?;
        if response.response.to_lowercase() == "false" {
            let message = response
                .error
                .unwrap_or_else(|| "OMDb returned no movie details.".to_string());
            return Err(map_omdb_error(&message));
        }
        response.details.ok_or_else(|| {
            MetadataError::InvalidResponse("OMDb response missing details.".to_string())
        })
    }

    fn normalize_title(&self, item: OmdbSearchItem) -> TitleMatch {
        let source_score = 0.7;
        let source_trust = 0.7;
        let global_score = source_score * source_trust;
        TitleMatch {
            id: item.imdb_id.clone(),
            name: item.title,
            year: item.year,
            source_score,
            source_trust,
            global_score,
            source: "OMDb".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    imdb: Some(item.imdb_id.clone()),
                    omdb: Some(item.imdb_id),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn normalize_movie(&self, details: OmdbMovieDetails) -> MovieMatch {
        let source_score = 0.7;
        let source_trust = 0.7;
        let global_score = source_score * source_trust;
        MovieMatch {
            id: details.imdb_id.clone(),
            title: details.title,
            year: details.year,
            source_score,
            source_trust,
            global_score,
            source: "OMDb".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    imdb: Some(details.imdb_id.clone()),
                    omdb: Some(details.imdb_id),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

}

fn map_omdb_error(message: &str) -> MetadataError {
    let normalized = message.to_lowercase();
    if normalized.contains("invalid api key")
        || normalized.contains("no api key")
        || normalized.contains("apikey")
    {
        return MetadataError::Other(format!("OMDb authentication failed: {message}"));
    }
    if normalized.contains("limit")
        || normalized.contains("quota")
        || normalized.contains("rate")
        || normalized.contains("requests")
    {
        return MetadataError::RateLimited(format!("OMDb quota exceeded: {message}"));
    }
    if normalized.contains("not found") {
        return MetadataError::NotFound(message.to_string());
    }
    MetadataError::InvalidResponse(message.to_string())
}

fn deserialize_year<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };
    match value {
        serde_json::Value::Number(number) => Ok(number.as_u64().map(|val| val as u16)),
        serde_json::Value::String(text) => {
            let year = text
                .chars()
                .take(4)
                .collect::<String>()
                .parse::<u16>()
                .ok();
            Ok(year)
        }
        _ => Ok(None),
    }
}

impl MetadataProvider for OmdbClient {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
        if query.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Search query cannot be empty.".to_string(),
            ));
        }
        let results = self.search_titles(query)?;
        Ok(results
            .into_iter()
            .map(|item| self.normalize_title(item))
            .collect())
    }

    fn fetch_episode_list(&mut self, _title_id: &str) -> Result<Vec<EpisodeMatch>, MetadataError> {
        Err(MetadataError::NotFound(
            "OMDb does not provide episode lists.".to_string(),
        ))
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
            "/tests/fixtures/omdb_search.json"
        ));
        let item: OmdbSearchItem = serde_json::from_str(payload).expect("fixture should parse");
        let client = OmdbClient::new("test-key");
        let normalized = client.normalize_title(item);

        assert_eq!(normalized.id, "tt1234567");
        assert_eq!(normalized.name, "Example Movie");
        assert_eq!(normalized.year, Some(2001));
        assert_eq!(normalized.source, "OMDb");
        assert_eq!(
            normalized.extras.external_ids.imdb.as_deref(),
            Some("tt1234567")
        );
    }
}
