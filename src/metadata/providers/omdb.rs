use std::collections::HashMap;

use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::MetadataProvider;

#[derive(Debug, Clone)]
pub struct OmdbClient {
    api_key: String,
    base_url: String,
}

#[derive(Debug, Clone)]
struct OmdbSearchItem {
    imdb_id: String,
    title: String,
    year: Option<u16>,
}

#[derive(Debug, Clone)]
struct OmdbMovieDetails {
    imdb_id: String,
    title: String,
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

    fn normalize_title(&self, item: OmdbSearchItem) -> TitleMatch {
        TitleMatch {
            id: item.imdb_id,
            name: item.title,
            year: item.year,
            score: 0.7,
            source: "OMDb".to_string(),
        }
    }

    fn normalize_movie(&self, details: OmdbMovieDetails) -> MovieMatch {
        MovieMatch {
            id: details.imdb_id,
            title: details.title,
            year: details.year,
            source: "OMDb".to_string(),
        }
    }

    fn mock_search_results(&self, query: &str) -> Vec<OmdbSearchItem> {
        vec![OmdbSearchItem {
            imdb_id: "tt0000001".to_string(),
            title: query.to_string(),
            year: None,
        }]
    }

    fn mock_movie_details(&self, _title_id: &str) -> OmdbMovieDetails {
        OmdbMovieDetails {
            imdb_id: "tt0000001".to_string(),
            title: "Example Movie".to_string(),
            year: None,
        }
    }
}

impl MetadataProvider for OmdbClient {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
        if query.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Search query cannot be empty.".to_string(),
            ));
        }
        let results = self.mock_search_results(query);
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
        let details = self.mock_movie_details(title_id);
        Ok(self.normalize_movie(details))
    }
}
