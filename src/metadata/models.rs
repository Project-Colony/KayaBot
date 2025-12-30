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
