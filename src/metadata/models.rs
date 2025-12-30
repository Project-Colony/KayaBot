#[derive(Debug, Clone)]
pub struct TitleMatch {
    pub id: String,
    pub name: String,
    pub year: Option<u16>,
    pub score: f32,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct EpisodeMatch {
    pub id: String,
    pub season: u32,
    pub episode: u32,
    pub title: String,
}

#[derive(Debug, Clone, Default)]
pub struct NormalizedTitle {
    pub source: String,
    pub source_id: String,
    pub title: String,
    pub release_year: Option<u16>,
    pub imdb_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NormalizedEpisode {
    pub source: String,
    pub series_id: String,
    pub series_title: String,
    pub season: u32,
    pub episode: u32,
    pub episode_title: Option<String>,
    pub release_year: Option<u16>,
    pub imdb_id: Option<String>,
}
