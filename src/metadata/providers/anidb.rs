use std::collections::HashMap;

use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::MetadataProvider;

#[derive(Debug, Clone)]
pub struct AniDbClient {
    api_key: String,
    base_url: String,
}

#[derive(Debug, Clone)]
struct AniDbTitle {
    id: u32,
    title: String,
    year: Option<u16>,
    rating: f32,
}

#[derive(Debug, Clone)]
struct AniDbEpisode {
    id: u32,
    number: u32,
    title: String,
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
            season: 1,
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

    fn mock_search_results(&self, query: &str) -> Vec<AniDbTitle> {
        vec![AniDbTitle {
            id: 1,
            title: query.to_string(),
            year: None,
            rating: 0.85,
        }]
    }

    fn mock_episode_results(&self, _title_id: &str) -> Vec<AniDbEpisode> {
        vec![AniDbEpisode {
            id: 1,
            number: 1,
            title: "Episode 1".to_string(),
        }]
    }
}

impl MetadataProvider for AniDbClient {
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

    fn fetch_movie_details(&mut self, _title_id: &str) -> Result<MovieMatch, MetadataError> {
        Err(MetadataError::NotFound(
            "AniDB is series-focused and does not provide movie details.".to_string(),
        ))
    }
}
