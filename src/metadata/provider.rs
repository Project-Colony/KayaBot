use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, TitleMatch};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataSource {
    TheMovieDb,
    AniDb,
    TheTvDb,
    TvMaze,
    Omdb,
}

impl MetadataSource {
    pub fn label(self) -> &'static str {
        match self {
            MetadataSource::TheMovieDb => "TheMovieDB",
            MetadataSource::AniDb => "AniDB",
            MetadataSource::TheTvDb => "TheTVDB",
            MetadataSource::TvMaze => "TVmaze",
            MetadataSource::Omdb => "OMDb",
        }
    }
}

pub trait MetadataProvider {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError>;
    fn fetch_episode_list(&mut self, title_id: &str) -> Result<Vec<EpisodeMatch>, MetadataError>;
}
