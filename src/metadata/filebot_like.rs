#![allow(dead_code)]

use std::collections::HashMap;

use crate::metadata::cache::MetadataCache;
use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};
use crate::metadata::provider::{MetadataProvider, MetadataSource};

#[derive(Debug)]
pub struct FileBotLikeProvider {
    dataset: HashMap<String, TitleRecord>,
    cache: MetadataCache,
    primary_source: MetadataSource,
    secondary_source: Option<MetadataSource>,
    active_source: MetadataSource,
}

#[derive(Debug, Clone)]
struct TitleRecord {
    title: String,
    year: Option<u16>,
    episodes: Vec<EpisodeMatch>,
}

impl Default for FileBotLikeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FileBotLikeProvider {
    pub fn new() -> Self {
        let dataset = HashMap::new();
        Self {
            dataset,
            cache: MetadataCache::new(),
            primary_source: MetadataSource::TheMovieDb,
            secondary_source: None,
            active_source: MetadataSource::TheMovieDb,
        }
    }

    pub fn set_sources(&mut self, primary: MetadataSource, secondary: Option<MetadataSource>) {
        if self.primary_source != primary || self.secondary_source != secondary {
            self.primary_source = primary;
            self.secondary_source = secondary;
            self.active_source = primary;
            self.cache.clear();
        }
    }

    pub fn set_source(&mut self, source: MetadataSource) {
        self.set_sources(source, None);
    }

    pub fn active_source(&self) -> MetadataSource {
        self.active_source
    }

    fn normalize_query(query: &str) -> String {
        query.trim().to_lowercase()
    }

    fn should_fallback(error: &MetadataError) -> bool {
        matches!(error, MetadataError::NotFound(_))
    }

    fn search_title_for_source(
        &mut self,
        query: &str,
        source: MetadataSource,
    ) -> Result<Vec<TitleMatch>, MetadataError> {
        let key = Self::normalize_query(query);
        if key.is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Search query cannot be empty.".to_string(),
            ));
        }

        if let Some(hit) = self.cache.get_title_search(source, &key) {
            return Ok(hit);
        }

        let mut matches = Vec::new();
        for (slug, record) in &self.dataset {
            if record.title.to_lowercase().contains(&key) {
                let source_score = 0.92;
                let source_trust = 0.8;
                let global_score = source_score * source_trust;
                matches.push(TitleMatch {
                    id: slug.clone(),
                    name: record.title.clone(),
                    year: record.year,
                    source_score,
                    source_trust,
                    global_score,
                    source: source.label().to_string(),
                    extras: crate::metadata::models::MetadataExtras::default(),
                });
            }
        }

        if matches.is_empty() {
            return Err(MetadataError::NotFound(format!(
                "No title match for query '{query}'."
            )));
        }

        self.cache.put_title_search(source, &key, matches.clone());
        Ok(matches)
    }

    fn fetch_episode_list_for_source(
        &mut self,
        title_id: &str,
        source: MetadataSource,
    ) -> Result<Vec<EpisodeMatch>, MetadataError> {
        if title_id.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Title identifier cannot be empty.".to_string(),
            ));
        }

        if let Some(hit) = self.cache.get_episode_list(source, title_id) {
            return Ok(hit);
        }

        let record = self
            .dataset
            .get(&title_id.to_lowercase())
            .ok_or_else(|| MetadataError::NotFound(format!("No title id '{title_id}'.")))?;

        let episodes = record.episodes.clone();
        self.cache
            .put_episode_list(source, &title_id.to_lowercase(), episodes.clone());
        Ok(episodes)
    }
}

impl MetadataProvider for FileBotLikeProvider {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
        let primary = self.primary_source;
        match self.search_title_for_source(query, primary) {
            Ok(matches) => {
                self.active_source = primary;
                Ok(matches)
            }
            Err(err) => {
                if Self::should_fallback(&err)
                    && let Some(secondary) = self.secondary_source
                    && secondary != primary
                {
                    let fallback = self.search_title_for_source(query, secondary)?;
                    self.active_source = secondary;
                    return Ok(fallback);
                }
                Err(err)
            }
        }
    }

    fn fetch_episode_list(&mut self, title_id: &str) -> Result<Vec<EpisodeMatch>, MetadataError> {
        let primary = self.active_source;
        match self.fetch_episode_list_for_source(title_id, primary) {
            Ok(episodes) => Ok(episodes),
            Err(err) => {
                if Self::should_fallback(&err)
                    && let Some(secondary) = self.secondary_source
                    && secondary != primary
                {
                    let fallback = self.fetch_episode_list_for_source(title_id, secondary)?;
                    self.active_source = secondary;
                    return Ok(fallback);
                }
                Err(err)
            }
        }
    }

    fn fetch_movie_details(&mut self, title_id: &str) -> Result<MovieMatch, MetadataError> {
        if title_id.trim().is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Title identifier cannot be empty.".to_string(),
            ));
        }

        let record = self
            .dataset
            .get(&title_id.to_lowercase())
            .ok_or_else(|| MetadataError::NotFound(format!("No title id '{title_id}'.")))?;

        let source_score = 1.0;
        let source_trust = 0.8;
        let global_score = source_score * source_trust;
        Ok(MovieMatch {
            id: title_id.to_string(),
            title: record.title.clone(),
            year: record.year,
            source_score,
            source_trust,
            global_score,
            source: self.active_source.label().to_string(),
            extras: crate::metadata::models::MetadataExtras::default(),
        })
    }
}
