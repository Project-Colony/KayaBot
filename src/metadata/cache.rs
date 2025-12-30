use std::collections::HashMap;

use crate::metadata::models::{EpisodeMatch, TitleMatch};

#[derive(Debug, Default)]
pub struct MetadataCache {
    title_searches: HashMap<String, Vec<TitleMatch>>,
    episode_lists: HashMap<String, Vec<EpisodeMatch>>,
}

impl MetadataCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_title_search(&self, query: &str) -> Option<Vec<TitleMatch>> {
        self.title_searches.get(query).cloned()
    }

    pub fn put_title_search(&mut self, query: &str, results: Vec<TitleMatch>) {
        self.title_searches.insert(query.to_string(), results);
    }

    pub fn get_episode_list(&self, title_id: &str) -> Option<Vec<EpisodeMatch>> {
        self.episode_lists.get(title_id).cloned()
    }

    pub fn put_episode_list(&mut self, title_id: &str, episodes: Vec<EpisodeMatch>) {
        self.episode_lists.insert(title_id.to_string(), episodes);
    }

    pub fn clear(&mut self) {
        self.title_searches.clear();
        self.episode_lists.clear();
    }
}
