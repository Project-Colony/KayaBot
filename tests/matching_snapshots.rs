use kayabot::matching::{MatchMetadata, MatchResult, match_files};

fn format_metadata(metadata: &MatchMetadata) -> String {
    match metadata {
        MatchMetadata::Series { title } => {
            format!("Series title={}", title.as_deref().unwrap_or("<none>"))
        }
        MatchMetadata::Episode {
            series_title,
            season,
            episode,
            episode_title,
        } => format!(
            "Episode series_title={} season={} episode={} episode_title={}",
            series_title.as_deref().unwrap_or("<none>"),
            season,
            episode,
            episode_title.as_deref().unwrap_or("<none>")
        ),
        MatchMetadata::Movie { title, year } => format!(
            "Movie title={} year={}",
            title.as_deref().unwrap_or("<none>"),
            year.map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string())
        ),
    }
}

fn format_results(results: &[MatchResult]) -> String {
    let mut lines = Vec::new();
    for result in results {
        lines.push(format!(
            "file={} status={:?} confidence={:.2}",
            result.original, result.status, result.confidence
        ));
        if let Some(metadata) = &result.metadata {
            lines.push(format!("  metadata: {}", format_metadata(metadata)));
        }
        for candidate in &result.candidates {
            lines.push(format!("  candidate: {}", format_metadata(candidate)));
        }
    }
    lines.join("\n")
}

#[test]
fn snapshot_matching_results() {
    let files = vec![
        "My.Show.S01E02.1080p.mkv".to_string(),
        "Example Movie 2020 4k.mkv".to_string(),
        "Mystery.Title.mkv".to_string(),
    ];

    let results = match_files(&files);
    insta::assert_snapshot!(format_results(&results));
}
