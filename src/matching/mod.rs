mod advanced;
mod parsing;

pub use advanced::{NormalizedName, RankedCandidate, normalize_name, rank_candidates};
use parsing::parse_filename;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStatus {
    Ok,
    Ambiguous,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentGuess {
    Series,
    Movie,
    Ambiguous,
    Unknown,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub original: String,
    pub metadata: Option<MatchMetadata>,
    pub candidates: Vec<MatchMetadata>,
    pub confidence: f32,
    pub status: MatchStatus,
}

pub fn match_files(files: &[String]) -> Vec<MatchResult> {
    files.iter().map(|file| match_single(file)).collect()
}

pub fn guess_content_type(results: &[MatchResult]) -> ContentGuess {
    let mut series_count = 0;
    let mut movie_count = 0;
    let mut saw_ambiguous = false;

    for result in results {
        match result.candidates.as_slice() {
            [MatchMetadata::Series { .. }] | [MatchMetadata::Episode { .. }] => series_count += 1,
            [MatchMetadata::Movie { .. }] => movie_count += 1,
            [] => {}
            _ => saw_ambiguous = true,
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
        confidence,
        status,
    }
}
