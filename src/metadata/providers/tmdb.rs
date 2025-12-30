use std::collections::HashMap;

use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::MetadataProvider;

#[derive(Debug, Clone)]
pub struct TmdbClient {
    api_key: String,
    base_url: String,
}

#[derive(Debug, Clone)]
struct TmdbTitle {
    id: u32,
    name: String,
    release_date: Option<String>,
    vote_average: f32,
}

#[derive(Debug, Clone)]
struct TmdbEpisode {
    id: u32,
    season_number: u32,
    episode_number: u32,
    name: String,
}

#[derive(Debug, Clone)]
struct TmdbMovieDetails {
    id: u32,
    title: String,
    release_date: Option<String>,
}

impl TmdbClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.themoviedb.org/3".to_string(),
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

    fn normalize_title(&self, title: TmdbTitle) -> TitleMatch {
        TitleMatch {
            id: title.id.to_string(),
            name: title.name,
            year: title
                .release_date
                .as_deref()
                .and_then(|date| date.get(0..4))
                .and_then(|year| year.parse::<u16>().ok()),
            score: title.vote_average / 10.0,
            source: "TheMovieDB".to_string(),
        }
    }

    fn normalize_episode(&self, episode: TmdbEpisode) -> EpisodeMatch {
        EpisodeMatch {
            id: episode.id.to_string(),
            season: episode.season_number,
            episode: episode.episode_number,
            title: episode.name,
        }
    }

    fn normalize_movie(&self, details: TmdbMovieDetails) -> MovieMatch {
        MovieMatch {
            id: details.id.to_string(),
            title: details.title,
            year: details
                .release_date
                .as_deref()
                .and_then(|date| date.get(0..4))
                .and_then(|year| year.parse::<u16>().ok()),
            source: "TheMovieDB".to_string(),
        }
    }

    fn mock_search_results(&self, query: &str) -> Vec<TmdbTitle> {
        vec![TmdbTitle {
            id: 1,
            name: query.to_string(),
            release_date: None,
            vote_average: 8.0,
        }]
    }

    fn mock_episode_results(&self, _title_id: &str) -> Vec<TmdbEpisode> {
        vec![TmdbEpisode {
            id: 1,
            season_number: 1,
            episode_number: 1,
            name: "Pilot".to_string(),
        }]
    }

    fn mock_movie_details(&self, title_id: &str) -> TmdbMovieDetails {
        TmdbMovieDetails {
            id: title_id.parse::<u32>().unwrap_or(1),
            title: "Example Movie".to_string(),
            release_date: None,
        }
    }
}

impl MetadataProvider for TmdbClient {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
        if query.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Search query cannot be empty.".to_string(),
            ));
        }
        let results = self.mock_search_results(query);
        Ok(results
            .into_iter()
            .map(|title| self.normalize_title(title))
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
