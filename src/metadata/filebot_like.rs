use std::collections::HashMap;

use crate::metadata::cache::MetadataCache;
use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, TitleMatch};
use crate::metadata::provider::{MetadataProvider, MetadataSource};

#[derive(Debug)]
pub struct FileBotLikeProvider {
    dataset: HashMap<String, TitleRecord>,
    cache: MetadataCache,
    active_source: MetadataSource,
}

#[derive(Debug, Clone)]
struct TitleRecord {
    title: String,
    year: Option<u16>,
    episodes: Vec<EpisodeMatch>,
}

impl FileBotLikeProvider {
    pub fn new() -> Self {
        let dataset = HashMap::new();
        Self {
            dataset,
            cache: MetadataCache::new(),
            active_source: MetadataSource::TheMovieDb,
        }
    }

    pub fn set_source(&mut self, source: MetadataSource) {
        if self.active_source != source {
            self.active_source = source;
            self.cache.clear();
        }
    }

    pub fn active_source(&self) -> MetadataSource {
        self.active_source
    }

    fn normalize_query(query: &str) -> String {
        query.trim().to_lowercase()
    }
}

impl MetadataProvider for FileBotLikeProvider {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
        let key = Self::normalize_query(query);
        if key.is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Search query cannot be empty.".to_string(),
            ));
        }

        if let Some(hit) = self.cache.get_title_search(&key) {
            return Ok(hit);
        }

        let mut matches = Vec::new();
        for (slug, record) in &self.dataset {
            if record.title.to_lowercase().contains(&key) {
                matches.push(TitleMatch {
                    id: slug.clone(),
                    name: record.title.clone(),
                    year: record.year,
                    score: 0.92,
                    source: self.active_source.label().to_string(),
                });
            }
        }

        if matches.is_empty() {
            return Err(MetadataError::NotFound(format!(
                "No title match for query '{query}'."
            )));
        }

        self.cache.put_title_search(&key, matches.clone());
        Ok(matches)
    }

    fn fetch_episode_list(&mut self, title_id: &str) -> Result<Vec<EpisodeMatch>, MetadataError> {
        if title_id.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Title identifier cannot be empty.".to_string(),
            ));
        }

        if let Some(hit) = self.cache.get_episode_list(title_id) {
            return Ok(hit);
        }

        let record = self
            .dataset
            .get(&title_id.to_lowercase())
            .ok_or_else(|| MetadataError::NotFound(format!("No title id '{title_id}'.")))?;

        let episodes = record.episodes.clone();
        self.cache
            .put_episode_list(&title_id.to_lowercase(), episodes.clone());
        Ok(episodes)
    }
}
