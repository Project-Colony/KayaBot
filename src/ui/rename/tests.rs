use std::fs;
use std::path::PathBuf;

use super::*;
use crate::lookup::tests::MockProvider;
use crate::test_support::{TempDir, fixture};

fn state() -> Rename {
    let mut state = Rename::new(
        MockProvider::pipeline("Lupin"),
        FormatOptions::default(),
        false,
    );
    state.preferred_series(MetadataSource::TheMovieDb);
    state
}

impl Rename {
    fn preferred_series(&mut self, source: MetadataSource) {
        self.plan.preferred_series = source;
        self.plan.preferred_movie = source;
    }
}

/// Copies sample files into a fresh directory and adds them to the list.
fn with_files(state: &mut Rename, names: &[&str]) -> (TempDir, Vec<PathBuf>) {
    let dir = TempDir::new("rename-view");
    let paths: Vec<PathBuf> = names
        .iter()
        .map(|name| {
            let target = dir.join(name);
            fs::copy(fixture(name), &target).expect("copy fixture");
            target
        })
        .collect();
    let _ = state.update(Message::Picked(paths.clone()));
    (dir, paths)
}

#[test]
fn fetching_fills_titles_and_episodes() {
    let mut state = state();
    let _dir = with_files(&mut state, &["Lupin.S01E05.720p.WEBRip.mp4"]);
    let _ = state.update(Message::Select(0));

    let _ = state.update(Message::Fetch);
    assert!(matches!(state.fetch, FetchState::Loading));
    assert_eq!(state.detected_name, "Lupin");

    // What the worker thread would send back.
    let outcome = {
        let mut pipeline = state.pipeline.lock().expect("pipeline");
        lookup::fetch(
            &mut pipeline,
            "Lupin",
            vec![MetadataSource::TheMovieDb],
            true,
        )
    };
    let _ = state.update(Message::Fetched(Ok(outcome)));

    assert!(matches!(state.fetch, FetchState::Ready));
    assert_eq!(state.titles.len(), 1);
    assert_eq!(state.episodes.len(), 2);
    assert_eq!(
        state.selected_title.as_deref(),
        Some("TheMovieDB:mock-series")
    );
    assert_eq!(state.answered_by, Some(MetadataSource::TheMovieDb));
}

#[test]
fn a_failed_lookup_is_reported() {
    let mut state = state();
    let _ = state.update(Message::Fetch);
    assert!(matches!(
        state.fetch,
        FetchState::Failed(FetchError::NoTitle)
    ));

    let _ = state.update(Message::Fetched(Err(MetadataError::Network("down".into()))));
    assert!(matches!(
        state.fetch,
        FetchState::Failed(FetchError::Metadata(MetadataError::Network(_)))
    ));
}

#[test]
fn a_match_for_an_older_file_list_is_dropped() {
    let mut state = state();
    let (_dir, paths) = with_files(&mut state, &["Spirited.Away.2001.avi"]);
    let generation = state.generation;
    let _ = state.update(Message::Match);
    assert!(state.matching);

    let _ = state.update(Message::Picked(vec![paths[0].with_file_name("other.mkv")]));
    let _ = state.update(Message::Matched(generation, Ok(Vec::new())));
    assert_eq!(state.results.len(), 2, "the stale result was ignored");
    assert!(!state.matching);
}

#[test]
fn a_dry_run_touches_nothing() {
    let mut state = state();
    let (_dir, paths) = with_files(
        &mut state,
        &["The Office US - S02E03 - Office Olympics.mkv"],
    );
    let _ = state.update(Message::DryRun(true));

    let _ = state.update(Message::Rename);

    assert_eq!(state.confirm, None, "a dry run never asks");
    let summary = &state.summaries[0];
    assert!(matches!(summary.outcome, RenameOutcome::DryRun));
    assert!(paths[0].exists());
    assert_ne!(PathBuf::from(&summary.resolved), paths[0]);
    assert!(!PathBuf::from(&summary.resolved).exists());
    assert_eq!(
        state.feedback,
        Some(Feedback::Previewed {
            ready: 1,
            skipped: 0,
            failed: 0
        })
    );
}

#[test]
fn a_real_rename_waits_for_confirmation() {
    let mut state = state();
    let (_dir, paths) = with_files(&mut state, &["Arcane_S02E01_1080p.mkv"]);

    let _ = state.update(Message::Rename);
    assert_eq!(state.confirm, Some(Confirm::Rename));
    assert!(paths[0].exists(), "nothing happens before the confirmation");

    let _ = state.update(Message::Cancel);
    assert_eq!(state.confirm, None);
    assert!(paths[0].exists());

    let _ = state.update(Message::Rename);
    let _ = state.update(Message::Confirm);
    let summary = &state.summaries[0];
    assert!(matches!(summary.outcome, RenameOutcome::Renamed));
    assert!(!paths[0].exists());
    assert!(PathBuf::from(&summary.resolved).exists());
    assert_eq!(
        state.files,
        std::slice::from_ref(&summary.resolved),
        "the list follows the new name"
    );
    assert_eq!(
        state.feedback,
        Some(Feedback::Renamed {
            renamed: 1,
            skipped: 0,
            failed: 0
        })
    );
    assert_eq!(state.export_rows()[0].status, "ok");
}

#[test]
fn without_confirmation_the_rename_is_immediate() {
    let mut state = state();
    state.confirm_before_rename = false;
    let (_dir, paths) = with_files(&mut state, &["Arcane_S02E01_Part1.mkv"]);

    let _ = state.update(Message::Rename);

    assert_eq!(state.confirm, None);
    assert!(!paths[0].exists());
}

#[test]
fn clearing_the_list_asks_first_when_set_to() {
    let mut state = state();
    let _dir = with_files(
        &mut state,
        &[
            "Spirited.Away.2001.avi",
            "Amelie.2001.FRENCH.1080p.BluRay.mkv",
        ],
    );

    let _ = state.update(Message::Clear);
    assert_eq!(state.confirm, Some(Confirm::Clear));
    assert_eq!(state.files.len(), 2);
    let _ = state.update(Message::Confirm);
    assert!(state.files.is_empty() && state.results.is_empty());

    state.confirm_before_clearing = false;
    let _dir = with_files(&mut state, &["Spirited.Away.2001.avi"]);
    let _ = state.update(Message::Clear);
    assert_eq!(state.confirm, None);
    assert!(state.files.is_empty());
}

#[test]
fn the_list_can_be_reordered_and_trimmed() {
    let mut state = state();
    let names = ["a.mkv", "b.mkv", "c.mkv"].map(|name| PathBuf::from("/media").join(name));
    let _ = state.update(Message::Picked(names.to_vec()));
    let _ = state.update(Message::Select(2));
    let _ = state.update(Message::MoveUp);
    assert_eq!(state.selected, Some(1));
    assert!(state.files[1].ends_with("c.mkv"));
    let _ = state.update(Message::MoveDown);
    assert!(state.files[2].ends_with("c.mkv"));
    let _ = state.update(Message::Remove);
    assert_eq!(state.files.len(), 2);
    assert_eq!(state.selected, Some(1));
    // Adding a file twice keeps one entry.
    let _ = state.update(Message::Picked(vec![names[0].clone()]));
    assert_eq!(state.files.len(), 2);
}

#[test]
fn an_override_needs_valid_numbers() {
    let mut state = state();
    let _ = state.update(Message::Picked(vec![PathBuf::from(
        "/media/Lupin.s01e05.mkv",
    )]));
    let _ = state.update(Message::OpenDetails);
    assert_eq!(state.details, Some(0));

    let _ = state.update(Message::OverrideSeason("two".to_string()));
    assert_eq!(
        state.draft.season, "two",
        "an invalid number stays on screen"
    );
    assert!(state.overrides.is_empty());

    let _ = state.update(Message::OverrideSeason("2".to_string()));
    let _ = state.update(Message::OverrideTitle("Lupin".to_string()));
    let preview = state.format_preview(&state.results[0]).expect("a name");
    assert!(preview.starts_with("Lupin 2x5"), "{preview}");

    let _ = state.update(Message::ClearOverride);
    assert!(state.overrides.is_empty());
    let _ = state.update(Message::CloseDetails);
    assert_eq!(state.details, None);
}

#[test]
fn a_name_without_anything_usable_proposes_nothing() {
    let mut state = state();
    let _ = state.update(Message::Picked(vec![PathBuf::from("/media/....mkv")]));
    assert_eq!(state.format_preview(&state.results[0]), None);
}

#[test]
fn templates_are_kept_per_content_type() {
    let mut state = state();
    state.set_template(ContentType::Movie, "{n} [{y}]".to_string());
    assert_eq!(state.template(ContentType::Movie), "{n} [{y}]");
    assert_eq!(state.template(ContentType::Series), "");
    assert_eq!(
        state.template_placeholder(ContentType::Series),
        formatting::DEFAULT_SERIES_FORMAT
    );
    state.set_template(ContentType::Movie, "  ".to_string());
    assert_eq!(state.template(ContentType::Movie), "");
}
