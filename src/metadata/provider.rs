use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, MovieMatch, TitleMatch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    pub fn from_label(label: &str) -> Option<Self> {
        match label.to_lowercase().as_str() {
            "themoviedb" | "tmdb" => Some(MetadataSource::TheMovieDb),
            "anidb" => Some(MetadataSource::AniDb),
            "thetvdb" | "tvdb" => Some(MetadataSource::TheTvDb),
            "tvmaze" => Some(MetadataSource::TvMaze),
            "omdb" => Some(MetadataSource::Omdb),
            _ => None,
        }
    }
}

pub trait MetadataProvider: Send {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError>;
    fn fetch_episode_list(&mut self, title_id: &str) -> Result<Vec<EpisodeMatch>, MetadataError>;
    fn fetch_movie_details(&mut self, title_id: &str) -> Result<MovieMatch, MetadataError>;
}
