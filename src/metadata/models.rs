#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExternalIds {
    pub imdb: Option<String>,
    pub tmdb: Option<String>,
    pub tvdb: Option<String>,
    pub tvmaze: Option<String>,
    pub omdb: Option<String>,
    pub other: Vec<ExternalId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalId {
    pub source: String,
    pub id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataExtras {
    pub aliases: Vec<String>,
    pub language: Option<String>,
    pub genres: Vec<String>,
    pub external_ids: ExternalIds,
    pub synopsis: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleMatch {
    pub id: String,
    pub name: String,
    pub year: Option<u16>,
    pub source_score: f32,
    pub source_trust: f32,
    pub global_score: f32,
    pub source: String,
    pub extras: MetadataExtras,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeMatch {
    pub id: String,
    pub season: u32,
    pub episode: u32,
    pub title: String,
    pub source_score: f32,
    pub source_trust: f32,
    pub global_score: f32,
    pub source: String,
    pub extras: MetadataExtras,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieMatch {
    pub id: String,
    pub title: String,
    pub year: Option<u16>,
    pub source_score: f32,
    pub source_trust: f32,
    pub global_score: f32,
    pub source: String,
    pub extras: MetadataExtras,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NormalizedTitle {
    pub source: String,
    pub source_id: String,
    pub title: String,
    pub release_year: Option<u16>,
    pub imdb_id: Option<String>,
    pub extras: MetadataExtras,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NormalizedEpisode {
    pub source: String,
    pub series_id: String,
    pub series_title: String,
    pub season: u32,
    pub episode: u32,
    pub episode_title: Option<String>,
    pub release_year: Option<u16>,
    pub imdb_id: Option<String>,
    pub extras: MetadataExtras,
}
