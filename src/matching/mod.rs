mod parsing;

use parsing::parse_filename;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStatus {
    Ok,
    Ambiguous,
    Error,
}

#[derive(Debug, Clone)]
pub enum MatchMetadata {
    Series {
        title: Option<String>,
        season: u32,
        episode: u32,
    },
    Movie {
        title: Option<String>,
        year: u32,
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
    files
        .iter()
        .map(|file| match_single(file))
        .collect()
}

fn match_single(filename: &str) -> MatchResult {
    let parsed = parse_filename(filename);
    let mut candidates = Vec::new();

    if let (Some(season), Some(episode)) = (parsed.season, parsed.episode) {
        candidates.push(MatchMetadata::Series {
            title: parsed.title.clone(),
            season,
            episode,
        });
    }

    if let Some(year) = parsed.year {
        candidates.push(MatchMetadata::Movie {
            title: parsed.title.clone(),
            year,
        });
    }

    let metadata = candidates.first().cloned();
    let mut confidence = if metadata.is_some() { 0.85 } else { 0.0 };

    if parsed.used_numeric_heuristic {
        confidence = confidence.min(0.65);
    }

    let status = if candidates.is_empty() {
        MatchStatus::Error
    } else if candidates.len() > 1 || parsed.used_numeric_heuristic {
        MatchStatus::Ambiguous
    } else if matches!(metadata, Some(MatchMetadata::Series { title: None, .. }))
        || matches!(metadata, Some(MatchMetadata::Movie { title: None, .. }))
    {
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
