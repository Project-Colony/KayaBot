mod advanced;
mod parsing;

use crate::metadata::models::{EpisodeMatch, TitleMatch};
pub use advanced::rank_candidates;
pub use parsing::{ParsedName, parse_filename};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchStatus {
    Ok,
    Ambiguous,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentGuess {
    Series,
    Movie,
    Ambiguous,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    Movie,
    Series,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchMetadata {
    Series {
        title: Option<String>,
    },
    Episode {
        series_title: Option<String>,
        season: u32,
        episode: u32,
        episode_title: Option<String>,
    },
    Movie {
        title: Option<String>,
        year: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub original: String,
    pub metadata: Option<MatchMetadata>,
    pub candidates: Vec<MatchMetadata>,
    pub content_type: Option<ContentType>,
    pub confidence: f32,
    pub metadata_confidence: Option<f32>,
    pub metadata_candidate_count: Option<usize>,
    pub status: MatchStatus,
    pub title_match: Option<TitleMatch>,
    pub episode_match: Option<EpisodeMatch>,
}

pub fn match_files(files: &[String]) -> Vec<MatchResult> {
    files.iter().map(|file| match_single(file)).collect()
}

pub fn guess_content_type_for_result(result: &MatchResult) -> ContentGuess {
    let mut series_count = 0;
    let mut movie_count = 0;
    let mut saw_ambiguous = false;

    if result.candidates.len() > 1 {
        saw_ambiguous = true;
    }

    for candidate in &result.candidates {
        match candidate {
            MatchMetadata::Series { .. } | MatchMetadata::Episode { .. } => series_count += 1,
            MatchMetadata::Movie { .. } => movie_count += 1,
        }
    }

    if series_count > 0 && movie_count == 0 && !saw_ambiguous {
        ContentGuess::Series
    } else if movie_count > 0 && series_count == 0 && !saw_ambiguous {
        ContentGuess::Movie
    } else if series_count > 0 || movie_count > 0 || saw_ambiguous {
        ContentGuess::Ambiguous
    } else {
        ContentGuess::Unknown
    }
}

fn match_single(filename: &str) -> MatchResult {
    let parsed = parse_filename(filename);
    let mut candidates = Vec::new();

    if let (Some(season), Some(episode)) = (parsed.season, parsed.episode) {
        candidates.push(MatchMetadata::Episode {
            series_title: parsed.title.clone(),
            season,
            episode,
            episode_title: None,
        });
    }

    if let Some(year) = parsed.year {
        candidates.push(MatchMetadata::Movie {
            title: parsed.title.clone(),
            year: Some(year),
        });
    }

    if parsed.season.is_none() && parsed.episode.is_none() && parsed.year.is_none() {
        if parsed.title.is_some() {
            candidates.push(MatchMetadata::Series {
                title: parsed.title.clone(),
            });
        }
    }

    let metadata = candidates.first().cloned();
    let mut confidence: f32 = if metadata.is_some() { 0.85 } else { 0.0 };

    if parsed.used_numeric_heuristic {
        confidence = confidence.min(0.65);
    }

    let status = if candidates.is_empty() {
        MatchStatus::Error
    } else if candidates.len() > 1 || parsed.used_numeric_heuristic {
        MatchStatus::Ambiguous
    } else if matches!(
        metadata,
        Some(MatchMetadata::Series { title: None, .. })
            | Some(MatchMetadata::Episode {
                series_title: None,
                ..
            })
            | Some(MatchMetadata::Movie { title: None, .. })
    ) {
        MatchStatus::Ambiguous
    } else {
        MatchStatus::Ok
    };

    MatchResult {
        original: filename.to_string(),
        metadata,
        candidates,
        content_type: None,
        confidence,
        metadata_confidence: None,
        metadata_candidate_count: None,
        status,
        title_match: None,
        episode_match: None,
    }
}
