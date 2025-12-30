use std::collections::HashMap;

use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::MetadataProvider;

#[derive(Debug, Clone)]
pub struct TheTvDbClient {
    api_key: String,
    base_url: String,
}

#[derive(Debug, Clone)]
struct TheTvDbSeries {
    id: String,
    name: String,
    year: Option<u16>,
    score: f32,
}

#[derive(Debug, Clone)]
struct TheTvDbEpisode {
    id: String,
    season: u32,
    episode: u32,
    name: String,
}

#[derive(Debug, Clone)]
struct TheTvDbMovieDetails {
    id: String,
    title: String,
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

    fn normalize_title(&self, series: TheTvDbSeries) -> TitleMatch {
        TitleMatch {
            id: series.id,
            name: series.name,
            year: series.year,
            score: series.score,
            source: "TheTVDB".to_string(),
        }
    }

    fn normalize_episode(&self, episode: TheTvDbEpisode) -> EpisodeMatch {
        EpisodeMatch {
            id: episode.id,
            season: episode.season,
            episode: episode.episode,
            title: episode.name,
        }
    }

    fn normalize_movie(&self, details: TheTvDbMovieDetails) -> MovieMatch {
        MovieMatch {
            id: details.id,
            title: details.title,
            year: details.year,
            source: "TheTVDB".to_string(),
        }
    }

    fn mock_search_results(&self, query: &str) -> Vec<TheTvDbSeries> {
        vec![TheTvDbSeries {
            id: "tvdb-1".to_string(),
            name: query.to_string(),
            year: None,
            score: 0.87,
        }]
    }

    fn mock_episode_results(&self, _title_id: &str) -> Vec<TheTvDbEpisode> {
        vec![TheTvDbEpisode {
            id: "tvdb-ep-1".to_string(),
            season: 1,
            episode: 1,
            name: "Pilot".to_string(),
        }]
    }

    fn mock_movie_details(&self, title_id: &str) -> TheTvDbMovieDetails {
        TheTvDbMovieDetails {
            id: title_id.to_string(),
            title: "Example Movie".to_string(),
            year: None,
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
        let results = self.mock_search_results(query);
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
        let episodes = self.mock_episode_results(title_id);
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
        let details = self.mock_movie_details(title_id);
        Ok(self.normalize_movie(details))
    }
}
