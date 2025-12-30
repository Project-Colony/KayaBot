use std::collections::HashMap;

use crate::metadata::models::{EpisodeMatch, TitleMatch};
use crate::metadata::provider::MetadataSource;

#[derive(Debug, Default)]
pub struct MetadataCache {
    title_searches: HashMap<(MetadataSource, String), Vec<TitleMatch>>,
    episode_lists: HashMap<(MetadataSource, String), Vec<EpisodeMatch>>,
}

impl MetadataCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_title_search(&self, source: MetadataSource, query: &str) -> Option<Vec<TitleMatch>> {
        self.title_searches
            .get(&(source, query.to_string()))
            .cloned()
    }

    pub fn put_title_search(
        &mut self,
        source: MetadataSource,
        query: &str,
        results: Vec<TitleMatch>,
    ) {
        self.title_searches
            .insert((source, query.to_string()), results);
    }

    pub fn get_episode_list(
        &self,
        source: MetadataSource,
        title_id: &str,
    ) -> Option<Vec<EpisodeMatch>> {
        self.episode_lists
            .get(&(source, title_id.to_string()))
            .cloned()
    }

    pub fn put_episode_list(
        &mut self,
        source: MetadataSource,
        title_id: &str,
        episodes: Vec<EpisodeMatch>,
    ) {
        self.episode_lists
            .insert((source, title_id.to_string()), episodes);
    }

    pub fn clear(&mut self) {
        self.title_searches.clear();
        self.episode_lists.clear();
    }
}
