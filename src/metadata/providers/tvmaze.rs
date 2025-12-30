use std::collections::HashMap;

use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::MetadataProvider;

#[derive(Debug, Clone)]
pub struct TvMazeClient {
    base_url: String,
    user_agent: String,
}

#[derive(Debug, Clone)]
struct TvMazeShow {
    id: u32,
    name: String,
    premiered: Option<String>,
    score: f32,
}

#[derive(Debug, Clone)]
struct TvMazeEpisode {
    id: u32,
    season: u32,
    number: u32,
    name: String,
}

impl TvMazeClient {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self {
            base_url: "https://api.tvmaze.com".to_string(),
            user_agent: user_agent.into(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn auth_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers.insert("User-Agent".to_string(), self.user_agent.clone());
        headers
    }

    fn normalize_title(&self, show: TvMazeShow) -> TitleMatch {
        let source_score = show.score;
        let source_trust = 0.75;
        let global_score = source_score * source_trust;
        TitleMatch {
            id: show.id.to_string(),
            name: show.name,
            year: show
                .premiered
                .as_deref()
                .and_then(|date| date.get(0..4))
                .and_then(|year| year.parse::<u16>().ok()),
            source_score,
            source_trust,
            global_score,
            source: "TVmaze".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    tvmaze: Some(show.id.to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn normalize_episode(&self, episode: TvMazeEpisode) -> EpisodeMatch {
        let source_score = 1.0;
        let source_trust = 0.75;
        let global_score = source_score * source_trust;
        EpisodeMatch {
            id: episode.id.to_string(),
            season: episode.season,
            episode: episode.number,
            title: episode.name,
            source_score,
            source_trust,
            global_score,
            source: "TVmaze".to_string(),
            extras: crate::metadata::models::MetadataExtras {
                external_ids: crate::metadata::models::ExternalIds {
                    tvmaze: Some(episode.id.to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn mock_search_results(&self, query: &str) -> Vec<TvMazeShow> {
        vec![TvMazeShow {
            id: 1,
            name: query.to_string(),
            premiered: None,
            score: 0.78,
        }]
    }

    fn mock_episode_results(&self, _title_id: &str) -> Vec<TvMazeEpisode> {
        vec![TvMazeEpisode {
            id: 1,
            season: 1,
            number: 1,
            name: "Pilot".to_string(),
        }]
    }
}

impl MetadataProvider for TvMazeClient {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
        if query.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Search query cannot be empty.".to_string(),
            ));
        }
        let results = self.mock_search_results(query);
        Ok(results
            .into_iter()
            .map(|show| self.normalize_title(show))
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
            "TVmaze does not provide movie details.".to_string(),
        ))
    }
}
