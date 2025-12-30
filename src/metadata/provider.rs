use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, TitleMatch};

pub trait MetadataProvider {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError>;
    fn fetch_episode_list(&mut self, title_id: &str) -> Result<Vec<EpisodeMatch>, MetadataError>;
}
