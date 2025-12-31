use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;

use crate::metadata::error::MetadataError;
use crate::metadata::locale::MetadataLocale;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::MetadataProvider;

#[derive(Debug, Clone)]
pub struct TmdbClient {
    api_key: String,
    base_url: String,
    locale: Option<MetadataLocale>,
}

#[derive(Debug, Clone, Deserialize)]
struct TmdbTitle {
    id: u32,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    title: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    #[serde(default)]
    vote_average: f32,
    #[serde(default)]
    media_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TmdbEpisode {
    id: u32,
    season_number: u32,
    episode_number: u32,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TmdbMovieDetails {
    id: u32,
    title: String,
    release_date: Option<String>,
}

impl TmdbClient {
    pub fn new(api_key: impl Into<String>, locale: Option<MetadataLocale>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.themoviedb.org/3".to_string(),
            locale,
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
                "TMDB API token is not configured.".to_string(),
            ));
        }
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.api_key)
                .parse()
                .map_err(|err| MetadataError::Other(format!("Invalid TMDB auth header: {err}")))?,
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "application/json".parse().map_err(|err| {
                MetadataError::Other(format!("Invalid TMDB accept header: {err}"))
            })?,
        );
        Ok(headers)
    }

    fn tmdb_locale(&self) -> (Option<String>, Option<String>) {
        fn region_for_language(language: &str) -> Option<&'static str> {
            match language {
                "fr" => Some("FR"),
                "en" => Some("US"),
                "es" => Some("ES"),
                "ja" => Some("JP"),
                _ => None,
            }
        }

        fn language_for_region(region: &str) -> Option<&'static str> {
            match region {
                "FR" => Some("fr"),
                "US" => Some("en"),
                "ES" => Some("es"),
                "JP" => Some("ja"),
                _ => None,
            }
        }

        let mut language = self
            .locale
            .as_ref()
            .and_then(|locale| locale.language.clone());
        let mut region = self
            .locale
            .as_ref()
            .and_then(|locale| locale.region.clone());

        if language.is_none() && region.is_none() {
            language = Some("fr".to_string());
            region = Some("FR".to_string());
        }

        if region.is_none() {
            if let Some(lang) = language.as_deref() {
                if let Some(default_region) = region_for_language(lang) {
                    region = Some(default_region.to_string());
                }
            }
        }

        if language.is_none() {
            if let Some(country) = region.as_deref() {
                if let Some(default_language) = language_for_region(country) {
                    language = Some(default_language.to_string());
                }
            }
        }

        let tmdb_language = match (&language, &region) {
            (Some(lang), Some(country)) => Some(format!("{lang}-{country}")),
            (Some(lang), None) => Some(lang.clone()),
            _ => None,
        };

        (tmdb_language, region)
    }

    fn build_query(&self, query: &[(&str, &str)], allow_region: bool) -> Vec<(String, String)> {
        let mut merged: Vec<(String, String)> = query
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let (language, region) = self.tmdb_locale();
        if let Some(language) = language {
            merged.push(("language".to_string(), language));
        }
        if allow_region {
            if let Some(region) = region {
                merged.push(("region".to_string(), region));
            }
        }
        merged
    }

    fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        query: &[(&str, &str)],
        allow_region: bool,
    ) -> Result<T, MetadataError> {
        let client = self.http_client()?;
        let url = format!("{}{}", self.base_url, path);
        let response = client
            .get(url)
            .headers(self.headers()?)
            .query(&self.build_query(query, allow_region))
            .send()
            .map_err(|err| {
                if err.is_timeout() {
                    MetadataError::Network("TMDB request timed out.".to_string())
                } else {
                    MetadataError::Network(format!("TMDB request failed: {err}"))
                }
            })?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(MetadataError::Other(
                "TMDB authorization failed. Check the configured API token.".to_string(),
            ));
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(MetadataError::NotFound(
                "TMDB did not return any results.".to_string(),
            ));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(MetadataError::RateLimited(
                "TMDB rate limit exceeded.".to_string(),
            ));
        }
        if !status.is_success() {
            return Err(MetadataError::InvalidResponse(format!(
                "TMDB returned status {status}."
            )));
        }
        response.json::<T>().map_err(|err| {
            MetadataError::InvalidResponse(format!("Failed to parse TMDB response: {err}"))
        })
    }

    fn search_titles(&self, query: &str) -> Result<Vec<TmdbTitle>, MetadataError> {
        #[derive(Deserialize)]
        struct SearchResponse {
            results: Vec<TmdbTitle>,
        }
        let response: SearchResponse = self.get_json(
            "/search/multi",
            &[("query", query), ("include_adult", "false")],
            true,
        )?;
        if response.results.is_empty() {
            return Err(MetadataError::NotFound(
                "TMDB returned no matches.".to_string(),
            ));
        }
        Ok(response.results)
    }

    fn fetch_tv_details(&self, title_id: &str) -> Result<TmdbTvDetails, MetadataError> {
        self.get_json(&format!("/tv/{title_id}"), &[], false)
    }

    fn fetch_season(
        &self,
        title_id: &str,
        season_number: u32,
    ) -> Result<TmdbSeasonDetails, MetadataError> {
        self.get_json(
            &format!("/tv/{title_id}/season/{season_number}"),
            &[],
            false,
        )
    }

    fn fetch_movie(&self, title_id: &str) -> Result<TmdbMovieDetails, MetadataError> {
        self.get_json(&format!("/movie/{title_id}"), &[], false)
    }

    fn normalize_title(&self, title: TmdbTitle) -> TitleMatch {
        let source_score = title.vote_average / 10.0;
        let source_trust = 0.9;
        let global_score = source_score * source_trust;
        let name = title
            .name
            .or(title.title)
            .unwrap_or_else(|| "Unknown title".to_string());
        let date = title.release_date.or(title.first_air_date);
        TitleMatch {
            id: title.id.to_string(),
            name,
            year: date
                .as_deref()
                .and_then(|date| date.get(0..4))
                .and_then(|year| year.parse::<u16>().ok()),
            source_score,
            source_trust,
            global_score,
            source: "TheMovieDB".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    tmdb: Some(title.id.to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn normalize_episode(&self, episode: TmdbEpisode) -> EpisodeMatch {
        let source_score = 1.0;
        let source_trust = 0.9;
        let global_score = source_score * source_trust;
        EpisodeMatch {
            id: episode.id.to_string(),
            season: episode.season_number,
            episode: episode.episode_number,
            title: episode.name,
            source_score,
            source_trust,
            global_score,
            source: "TheMovieDB".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    tmdb: Some(episode.id.to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn normalize_movie(&self, details: TmdbMovieDetails) -> MovieMatch {
        let source_score = 1.0;
        let source_trust = 0.9;
        let global_score = source_score * source_trust;
        MovieMatch {
            id: details.id.to_string(),
            title: details.title,
            year: details
                .release_date
                .as_deref()
                .and_then(|date| date.get(0..4))
                .and_then(|year| year.parse::<u16>().ok()),
            source_score,
            source_trust,
            global_score,
            source: "TheMovieDB".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    tmdb: Some(details.id.to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TmdbTvDetails {
    #[serde(default)]
    seasons: Vec<TmdbSeasonRef>,
}

#[derive(Debug, Clone, Deserialize)]
struct TmdbSeasonRef {
    season_number: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct TmdbSeasonDetails {
    #[serde(default)]
    episodes: Vec<TmdbEpisode>,
}

impl MetadataProvider for TmdbClient {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
        if query.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Search query cannot be empty.".to_string(),
            ));
        }
        let results = self.search_titles(query)?;
        Ok(results
            .into_iter()
            .filter(|title| title.media_type.as_deref() != Some("person"))
            .map(|title| self.normalize_title(title))
            .collect())
    }

    fn fetch_episode_list(&mut self, title_id: &str) -> Result<Vec<EpisodeMatch>, MetadataError> {
        if title_id.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Title identifier cannot be empty.".to_string(),
            ));
        }
        let details = self.fetch_tv_details(title_id)?;
        if details.seasons.is_empty() {
            return Err(MetadataError::NotFound(
                "TMDB returned no seasons for this title.".to_string(),
            ));
        }
        let mut episodes = Vec::new();
        for season in details.seasons {
            let season_details = self.fetch_season(title_id, season.season_number)?;
            for episode in season_details.episodes {
                episodes.push(self.normalize_episode(episode));
            }
        }
        if episodes.is_empty() {
            return Err(MetadataError::NotFound(
                "TMDB returned no episodes for this title.".to_string(),
            ));
        }
        Ok(episodes)
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
            "/tests/fixtures/tmdb_search.json"
        ));
        let title: TmdbTitle = serde_json::from_str(payload).expect("fixture should parse");
        let client = TmdbClient::new("test-key", None);
        let normalized = client.normalize_title(title);

        assert_eq!(normalized.id, "550");
        assert_eq!(normalized.name, "Fight Club");
        assert_eq!(normalized.year, Some(1999));
        assert_eq!(normalized.source, "TheMovieDB");
        assert_eq!(normalized.extras.external_ids.tmdb.as_deref(), Some("550"));
    }
}
