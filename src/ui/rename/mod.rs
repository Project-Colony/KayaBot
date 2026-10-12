//! The Rename view: the files to rename, what they match, and the rename.
//!
//! Lookups block on the network, so they run on a worker thread and come back
//! as messages; the view stays responsive while a source is slow.

mod details;
mod files;
mod preview;
mod summary;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use colony_ui::Typography;
use iced::widget::{column, row};
use iced::{Element, Length, Task};

use crate::export::{self, ExportFormat, ExportRow};
use crate::formatting::{self, FormatOptions, MovieFormatInput, SeriesFormatInput};
use crate::lookup::{self, FetchOutcome, SourcePlan};
use crate::matching::{self, ContentGuess, ContentType, MatchMetadata, MatchResult, MatchStatus};
use crate::metadata::aggregate::MetadataPipeline;
use crate::metadata::error::MetadataError;
use crate::metadata::models::{EpisodeMatch, TitleMatch};
use crate::metadata::provider::MetadataSource;
use crate::rename::{self, RenameOutcome};

use super::widgets;

/// The metadata sources, shared with the worker threads.
pub type SharedPipeline = Arc<Mutex<MetadataPipeline>>;

#[derive(Debug, Clone)]
pub enum FetchState {
    Idle,
    Loading,
    Ready,
    Failed(FetchError),
}

#[derive(Debug, Clone)]
pub enum FetchError {
    /// Neither the series field nor the file name gave a title to search.
    NoTitle,
    Metadata(MetadataError),
}

/// Some files could not be told apart as a movie or an episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionNotice {
    Ambiguous,
    Unknown,
}

/// An action waiting for the user to confirm it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirm {
    Rename,
    Clear,
}

/// The line under the lists saying what the last action did.
#[derive(Debug, Clone, PartialEq)]
pub enum Feedback {
    NothingToRename,
    Renamed {
        renamed: usize,
        skipped: usize,
        failed: usize,
    },
    Previewed {
        ready: usize,
        skipped: usize,
        failed: usize,
    },
    NothingToExport,
    Exported(PathBuf),
    ExportFailed(String),
}

/// What happened to one file of the last rename.
#[derive(Debug, Clone)]
pub struct RenameSummary {
    pub original: String,
    pub resolved: String,
    pub collision_adjusted: bool,
    pub outcome: RenameOutcome,
}

impl RenameSummary {
    pub fn failed(&self) -> bool {
        matches!(self.outcome, RenameOutcome::Failed(_))
    }
}

/// A title, season or episode the user typed or picked for one file, over
/// whatever the lookup found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManualOverride {
    pub title: Option<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
}

impl ManualOverride {
    fn is_empty(&self) -> bool {
        self.title
            .as_deref()
            .is_none_or(|title| title.trim().is_empty())
            && self.season.is_none()
            && self.episode.is_none()
    }
}

/// The override fields of the match dialog as typed, so an invalid number
/// stays on screen with its warning instead of being dropped.
#[derive(Debug, Clone, Default)]
pub struct OverrideDraft {
    pub title: String,
    pub season: String,
    pub episode: String,
}

impl OverrideDraft {
    fn from_override(entry: Option<&ManualOverride>) -> Self {
        let entry = entry.cloned().unwrap_or_default();
        Self {
            title: entry.title.unwrap_or_default(),
            season: entry.season.map(|n| n.to_string()).unwrap_or_default(),
            episode: entry.episode.map(|n| n.to_string()).unwrap_or_default(),
        }
    }

    /// The override these fields describe, or `None` while a number is invalid.
    fn parse(&self) -> Option<ManualOverride> {
        let number = |text: &str| -> Option<Option<u32>> {
            let text = text.trim();
            if text.is_empty() {
                Some(None)
            } else {
                text.parse().ok().map(Some)
            }
        };
        let title = self.title.trim();
        Some(ManualOverride {
            title: (!title.is_empty()).then(|| title.to_string()),
            season: number(&self.season)?,
            episode: number(&self.episode)?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    AddFiles,
    AddFolder,
    /// Files chosen in a dialog or dropped on the window, folders expanded.
    Picked(Vec<PathBuf>),
    Dropped(PathBuf),
    Select(usize),
    MoveUp,
    MoveDown,
    Remove,
    Rescan,
    Clear,
    Match,
    Matched(u64, Result<Vec<MatchResult>, MetadataError>),
    Fetch,
    Fetched(Result<FetchOutcome, MetadataError>),
    ContentType(ContentType),
    DetectedName(String),
    ForceSource(bool),
    Source(MetadataSource),
    IncludeEpisodeTitle(bool),
    IncludeYear(bool),
    Template(String),
    TemplatePreset,
    TemplateReset,
    DryRun(bool),
    Rename,
    Confirm,
    Cancel,
    Export(ExportFormat),
    ExportTo(ExportFormat, Option<PathBuf>),
    OpenDetails,
    CloseDetails,
    PickTitle(String),
    EpisodesLoaded(Result<(Vec<EpisodeMatch>, MetadataSource), MetadataError>),
    PickEpisode(u32, u32),
    OverrideTitle(String),
    OverrideSeason(String),
    OverrideEpisode(String),
    ClearOverride,
}

pub struct Rename {
    files: Vec<String>,
    results: Vec<MatchResult>,
    selected: Option<usize>,
    detected_name: String,
    fetch: FetchState,
    /// A match is running for the file list of this generation.
    matching: bool,
    /// Bumped whenever the file list changes, so a match started on an older
    /// list is dropped when it lands.
    generation: u64,
    loading_episodes: bool,
    titles: Vec<TitleMatch>,
    selected_title: Option<String>,
    episodes: Vec<EpisodeMatch>,
    /// The file whose match dialog is open.
    details: Option<usize>,
    draft: OverrideDraft,
    overrides: HashMap<String, ManualOverride>,
    feedback: Option<Feedback>,
    dry_run: bool,
    summaries: Vec<RenameSummary>,
    format: FormatOptions,
    plan: SourcePlan,
    /// The source that answered the last lookup.
    answered_by: Option<MetadataSource>,
    notice: Option<DetectionNotice>,
    pipeline: SharedPipeline,
    confirm: Option<Confirm>,
    pub confirm_before_rename: bool,
    pub confirm_before_clearing: bool,
}

impl Rename {
    pub fn new(pipeline: MetadataPipeline, format: FormatOptions, dry_run: bool) -> Self {
        Self {
            files: Vec::new(),
            results: Vec::new(),
            selected: None,
            detected_name: String::new(),
            fetch: FetchState::Idle,
            matching: false,
            generation: 0,
            loading_episodes: false,
            titles: Vec::new(),
            selected_title: None,
            episodes: Vec::new(),
            details: None,
            draft: OverrideDraft::default(),
            overrides: HashMap::new(),
            feedback: None,
            dry_run,
            summaries: Vec::new(),
            format,
            plan: SourcePlan::default(),
            answered_by: None,
            notice: None,
            pipeline: Arc::new(Mutex::new(pipeline)),
            confirm: None,
            confirm_before_rename: true,
            confirm_before_clearing: true,
        }
    }

    /// Swaps in sources rebuilt with new keys or a new locale. A lookup
    /// already running finishes on the old ones.
    pub fn set_pipeline(&mut self, pipeline: MetadataPipeline) {
        self.pipeline = Arc::new(Mutex::new(pipeline));
    }

    fn busy(&self) -> bool {
        self.matching || self.loading_episodes || matches!(self.fetch, FetchState::Loading)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddFiles => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .pick_files()
                            .await
                            .unwrap_or_default()
                            .iter()
                            .map(|file| file.path().to_path_buf())
                            .collect::<Vec<_>>()
                    },
                    Message::Picked,
                );
            }
            Message::AddFolder => {
                return Task::perform(
                    async {
                        match rfd::AsyncFileDialog::new().pick_folder().await {
                            Some(folder) => collect_files(folder.path()),
                            None => Vec::new(),
                        }
                    },
                    Message::Picked,
                );
            }
            Message::Dropped(path) => {
                return Task::perform(async move { collect_files(&path) }, Message::Picked);
            }
            Message::Picked(paths) => {
                if self.add_files(paths) {
                    self.files_changed();
                }
            }
            Message::Select(index) => {
                if index < self.files.len() {
                    self.selected = Some(index);
                }
            }
            Message::MoveUp => {
                if let Some(index) = self.selected.filter(|&i| i > 0 && i < self.files.len()) {
                    self.files.swap(index, index - 1);
                    self.selected = Some(index - 1);
                    self.files_changed();
                }
            }
            Message::MoveDown => {
                if let Some(index) = self.selected.filter(|&i| i + 1 < self.files.len()) {
                    self.files.swap(index, index + 1);
                    self.selected = Some(index + 1);
                    self.files_changed();
                }
            }
            Message::Remove => {
                if let Some(index) = self.selected.filter(|&i| i < self.files.len()) {
                    self.files.remove(index);
                    self.selected = match self.files.len() {
                        0 => None,
                        len => Some(index.min(len - 1)),
                    };
                    self.files_changed();
                }
            }
            Message::Rescan => {
                self.files.retain(|path| Path::new(path).exists());
                if self.selected.is_some_and(|index| index >= self.files.len()) {
                    self.selected = None;
                }
                self.files_changed();
            }
            Message::Clear => {
                if !self.files.is_empty() {
                    if self.confirm_before_clearing {
                        self.confirm = Some(Confirm::Clear);
                    } else {
                        self.clear();
                    }
                }
            }
            Message::Match => return self.start_match(),
            Message::Matched(generation, results) => {
                if generation == self.generation {
                    self.matching = false;
                    match results {
                        Ok(results) => {
                            self.results = results;
                            self.detect_content_types();
                        }
                        Err(err) => self.fetch = FetchState::Failed(FetchError::Metadata(err)),
                    }
                }
            }
            Message::Fetch => return self.start_fetch(),
            Message::Fetched(outcome) => self.finish_fetch(outcome),
            Message::ContentType(content_type) => {
                self.set_content_type(content_type);
                self.reset_lookup();
            }
            Message::DetectedName(name) => self.detected_name = name,
            Message::ForceSource(force) => {
                if force {
                    self.plan.chosen = self.plan.preferred(self.active_content_type());
                }
                self.plan.force = force;
                self.reset_lookup();
            }
            Message::Source(source) => {
                self.plan.chosen = source;
                match self.active_content_type() {
                    ContentType::Movie => self.plan.preferred_movie = source,
                    ContentType::Series => self.plan.preferred_series = source,
                }
                self.reset_lookup();
                self.notice = None;
            }
            Message::IncludeEpisodeTitle(include) => {
                self.format.include_episode_title = include;
                self.format.save();
            }
            Message::IncludeYear(include) => {
                self.format.include_year = include;
                self.format.save();
            }
            Message::Template(template) => {
                self.set_template(self.active_content_type(), template);
            }
            Message::TemplatePreset => {
                let content_type = self.active_content_type();
                let preset = match content_type {
                    ContentType::Series => formatting::DEFAULT_SERIES_FORMAT,
                    ContentType::Movie => formatting::DEFAULT_MOVIE_FORMAT,
                };
                *self.template_slot(content_type) = Some(preset.to_string());
                self.format.save();
            }
            Message::TemplateReset => {
                let content_type = self.active_content_type();
                *self.template_slot(content_type) = None;
                match content_type {
                    ContentType::Series => self.format.include_episode_title = true,
                    ContentType::Movie => self.format.include_year = true,
                }
                self.format.save();
            }
            Message::DryRun(dry_run) => self.dry_run = dry_run,
            Message::Rename => {
                if self.results.is_empty() {
                    self.feedback = Some(Feedback::NothingToRename);
                } else if !self.dry_run && self.confirm_before_rename {
                    self.confirm = Some(Confirm::Rename);
                } else {
                    self.rename();
                }
            }
            Message::Confirm => match self.confirm.take() {
                Some(Confirm::Rename) => self.rename(),
                Some(Confirm::Clear) => self.clear(),
                None => {}
            },
            Message::Cancel => self.confirm = None,
            Message::Export(format) => {
                if self.summaries.is_empty() {
                    self.feedback = Some(Feedback::NothingToExport);
                    return Task::none();
                }
                return Task::perform(
                    rfd::AsyncFileDialog::new()
                        .set_file_name(format.default_file_name())
                        .save_file(),
                    move |file| {
                        Message::ExportTo(format, file.map(|file| file.path().to_path_buf()))
                    },
                );
            }
            Message::ExportTo(format, Some(path)) => {
                self.feedback = Some(match export::write(&path, format, &self.export_rows()) {
                    Ok(()) => Feedback::Exported(path),
                    Err(err) => Feedback::ExportFailed(err.to_string()),
                });
            }
            Message::ExportTo(_, None) => {}
            Message::OpenDetails => {
                if self.selected.is_none() && !self.files.is_empty() {
                    self.selected = Some(0);
                }
                self.details = self.selected;
                self.draft = OverrideDraft::from_override(self.details_override());
            }
            Message::CloseDetails => self.details = None,
            Message::PickTitle(id) => return self.pick_title(id),
            Message::EpisodesLoaded(loaded) => {
                self.loading_episodes = false;
                match loaded {
                    Ok((episodes, source)) => {
                        self.episodes = episodes;
                        self.answered_by = Some(source);
                    }
                    Err(err) => {
                        self.episodes.clear();
                        self.fetch = FetchState::Failed(FetchError::Metadata(err));
                    }
                }
            }
            Message::PickEpisode(season, episode) => {
                let mut entry = self.details_override().cloned().unwrap_or_default();
                entry.season = Some(season);
                entry.episode = Some(episode);
                if entry.title.is_none() {
                    entry.title = self.selected_title_match().map(|title| title.name.clone());
                }
                self.draft = OverrideDraft::from_override(Some(&entry));
                self.set_details_override(entry);
            }
            Message::OverrideTitle(title) => {
                self.draft.title = title;
                self.apply_draft();
            }
            Message::OverrideSeason(season) => {
                self.draft.season = season;
                self.apply_draft();
            }
            Message::OverrideEpisode(episode) => {
                self.draft.episode = episode;
                self.apply_draft();
            }
            Message::ClearOverride => {
                if let Some(file) = self.details_file() {
                    self.overrides.remove(&file);
                }
                self.draft = OverrideDraft::default();
            }
        }
        Task::none()
    }

    pub fn view<'a>(&'a self, typo: &Typography) -> Element<'a, Message> {
        let lists = row![
            files::view(self, typo),
            preview::actions(self, typo),
            preview::view(self, typo),
        ]
        .spacing(12)
        .height(Length::FillPortion(3));

        let page = column![
            widgets::heading(typo, crate::i18n::t("rename_title"), 22),
            lists,
            preview::options(self, typo),
            summary::view(self, typo),
        ]
        .spacing(12)
        .padding(16);

        if let Some(confirm) = self.confirm {
            widgets::modal(
                page,
                summary::confirm_dialog(self, typo, confirm),
                Message::Cancel,
            )
        } else if let Some(index) = self.details {
            widgets::modal(
                page,
                details::view(self, typo, index),
                Message::CloseDetails,
            )
        } else {
            page.into()
        }
    }

    #[cfg(test)]
    pub(crate) fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Whether a dialog covers the view, so shortcuts leave the list alone.
    pub fn dialog_open(&self) -> bool {
        self.confirm.is_some() || self.details.is_some()
    }

    fn add_files(&mut self, paths: Vec<PathBuf>) -> bool {
        let mut added = false;
        for path in paths {
            let path = path.display().to_string();
            if !self.files.contains(&path) {
                self.files.push(path);
                added = true;
            }
        }
        if added {
            self.selected = None;
        }
        added
    }

    fn files_changed(&mut self) {
        self.generation += 1;
        self.matching = false;
        self.results = matching::match_files(&self.files);
        self.detect_content_types();
        self.feedback = None;
        self.summaries.clear();
        let files: HashSet<&String> = self.files.iter().collect();
        self.overrides.retain(|file, _| files.contains(file));
        if self.details.is_some_and(|index| index >= self.files.len()) {
            self.details = None;
        }
    }

    fn clear(&mut self) {
        self.files.clear();
        self.selected = None;
        self.details = None;
        self.overrides.clear();
        self.files_changed();
    }

    /// Forgets what the last lookup found, after its inputs changed.
    fn reset_lookup(&mut self) {
        self.fetch = FetchState::Idle;
        self.titles.clear();
        self.episodes.clear();
        self.selected_title = None;
        self.feedback = None;
        self.summaries.clear();
    }

    fn start_match(&mut self) -> Task<Message> {
        if self.files.is_empty() || self.busy() {
            return Task::none();
        }
        self.matching = true;
        self.feedback = None;
        self.summaries.clear();
        let results = matching::match_files(&self.files);
        let plan = self.plan;
        let detected = self.detected_name.clone();
        let generation = self.generation;
        self.run(
            move |pipeline| Ok(lookup::match_results(pipeline, results, &plan, &detected)),
            move |results| Message::Matched(generation, results),
        )
    }

    fn start_fetch(&mut self) -> Task<Message> {
        if self.busy() {
            return Task::none();
        }
        self.fetch = FetchState::Loading;
        self.feedback = None;
        self.summaries.clear();

        let hint = self.search_hint();
        if self.detected_name.trim().is_empty()
            && let Some(title) = hint.as_ref().and_then(|hint| hint.title.clone())
        {
            self.detected_name = title;
        }

        let mut content_type = self.active_content_type();
        if let Some(hint) = &hint {
            let has_episode = hint.season.is_some() || hint.episode.is_some();
            let switch_to = if has_episode {
                ContentType::Series
            } else if hint.year.is_some() {
                ContentType::Movie
            } else {
                content_type
            };
            if switch_to != content_type {
                self.set_content_type(switch_to);
                self.notice = None;
                content_type = switch_to;
            }
        }

        let mut query = self.detected_name.trim().to_string();
        if content_type == ContentType::Movie
            && !query.is_empty()
            && let Some(year) = hint.as_ref().and_then(|hint| hint.year)
        {
            query = format!("{query} {year}");
        }
        if query.is_empty() {
            self.fetch = FetchState::Failed(FetchError::NoTitle);
            return Task::none();
        }

        let sources = self.plan.sources(content_type);
        let with_episodes = content_type == ContentType::Series;
        self.run(
            move |pipeline| Ok(lookup::fetch(pipeline, &query, sources, with_episodes)),
            Message::Fetched,
        )
    }

    fn finish_fetch(&mut self, outcome: Result<FetchOutcome, MetadataError>) {
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(err) => {
                self.fetch = FetchState::Failed(FetchError::Metadata(err));
                return;
            }
        };
        self.answered_by = Some(outcome.source);
        self.episodes.clear();
        match outcome.titles {
            Ok(titles) => {
                self.selected_title = lookup::best_title(&titles).map(|best| best.id.clone());
                self.titles = titles;
                self.fetch = match outcome.episodes {
                    Some(Err(err)) => FetchState::Failed(FetchError::Metadata(err)),
                    Some(Ok(episodes)) => {
                        self.episodes = episodes;
                        FetchState::Ready
                    }
                    None => FetchState::Ready,
                };
            }
            Err(err) => {
                self.titles.clear();
                self.selected_title = None;
                self.fetch = FetchState::Failed(FetchError::Metadata(err));
            }
        }
    }

    fn pick_title(&mut self, id: String) -> Task<Message> {
        let Some(file) = self.details_file() else {
            return Task::none();
        };
        let Some(name) = self
            .titles
            .iter()
            .find(|title| title.id == id)
            .map(|title| title.name.clone())
        else {
            return Task::none();
        };
        self.selected_title = Some(id.clone());
        let mut entry = self.overrides.get(&file).cloned().unwrap_or_default();
        entry.title = Some(name.clone());
        self.draft.title = name;
        self.set_details_override(entry);

        let content_type = self
            .content_type_for_file(&file)
            .unwrap_or_else(|| self.active_content_type());
        if content_type != ContentType::Series || self.loading_episodes {
            return Task::none();
        }
        self.loading_episodes = true;
        self.run(
            move |pipeline| {
                let (episodes, source) = lookup::episodes(pipeline, &id);
                episodes.map(|episodes| (episodes, source))
            },
            Message::EpisodesLoaded,
        )
    }

    /// Runs `job` on a worker thread with the metadata sources, then turns
    /// its result into a message. Nothing starts until the task is run. A job
    /// that dies reports an error rather than leaving the view waiting.
    fn run<T: Send + 'static>(
        &self,
        job: impl FnOnce(&mut MetadataPipeline) -> Result<T, MetadataError> + Send + 'static,
        done: impl Fn(Result<T, MetadataError>) -> Message + Send + 'static,
    ) -> Task<Message> {
        let pipeline = Arc::clone(&self.pipeline);
        Task::perform(
            async move {
                let (sender, receiver) = iced::futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let mut pipeline = pipeline.lock().unwrap_or_else(PoisonError::into_inner);
                    let _ = sender.send(job(&mut pipeline));
                });
                receiver.await.unwrap_or_else(|_| {
                    Err(MetadataError::Other(
                        "the lookup stopped unexpectedly".to_string(),
                    ))
                })
            },
            done,
        )
    }

    fn rename(&mut self) {
        self.summaries.clear();
        let mut used_targets = HashSet::new();
        let mut renamed_files = self.files.clone();
        for (index, (original, result)) in self.files.iter().zip(&self.results).enumerate() {
            let summary = self.rename_one(original, result, &mut used_targets);
            if matches!(summary.outcome, RenameOutcome::Renamed) {
                renamed_files[index] = summary.resolved.clone();
            }
            self.summaries.push(summary);
        }

        let count = |pick: fn(&RenameOutcome) -> bool| {
            self.summaries.iter().filter(|s| pick(&s.outcome)).count()
        };
        let skipped = count(|o| matches!(o, RenameOutcome::Skipped(_) | RenameOutcome::Unchanged));
        let failed = count(|o| matches!(o, RenameOutcome::Failed(_)));
        let feedback = if self.dry_run {
            Feedback::Previewed {
                ready: count(|o| matches!(o, RenameOutcome::DryRun)),
                skipped,
                failed,
            }
        } else {
            Feedback::Renamed {
                renamed: count(|o| matches!(o, RenameOutcome::Renamed)),
                skipped,
                failed,
            }
        };

        if !self.dry_run {
            // The files now carry their new names: match those, keeping the
            // summary of what just happened.
            let summaries = std::mem::take(&mut self.summaries);
            self.files = renamed_files;
            self.overrides.clear();
            self.files_changed();
            self.summaries = summaries;
        }
        self.feedback = Some(feedback);
    }

    fn rename_one(
        &self,
        original: &str,
        result: &MatchResult,
        used_targets: &mut HashSet<PathBuf>,
    ) -> RenameSummary {
        let summary = |resolved: String, collision_adjusted, outcome| RenameSummary {
            original: original.to_string(),
            resolved,
            collision_adjusted,
            outcome,
        };
        let original_path = PathBuf::from(original);
        if !original_path.exists() {
            return summary(
                original.to_string(),
                false,
                RenameOutcome::Failed(crate::i18n::t("rename_file_missing").to_string()),
            );
        }
        let preview = self
            .format_preview(result)
            .filter(|preview| !preview.trim().is_empty() && result.status != MatchStatus::Error);
        let Some(preview) = preview else {
            return summary(
                original.to_string(),
                false,
                RenameOutcome::Skipped(crate::i18n::t("rename_no_name").to_string()),
            );
        };
        let report = rename::apply(&original_path, &preview, used_targets, self.dry_run);
        summary(
            report.target.display().to_string(),
            report.collision_adjusted,
            report.outcome,
        )
    }

    fn export_rows(&self) -> Vec<ExportRow> {
        self.summaries
            .iter()
            .map(|summary| ExportRow {
                status: if summary.failed() { "error" } else { "ok" },
                message: summary::message(summary),
                final_path: summary.resolved.clone(),
            })
            .collect()
    }

    fn detect_content_types(&mut self) {
        let mut ambiguous = false;
        let mut unknown = false;
        for result in &mut self.results {
            result.content_type = match matching::guess_content_type_for_result(result) {
                ContentGuess::Series => Some(ContentType::Series),
                ContentGuess::Movie => Some(ContentType::Movie),
                ContentGuess::Ambiguous => {
                    ambiguous = true;
                    None
                }
                ContentGuess::Unknown => {
                    unknown = true;
                    None
                }
            };
        }
        self.notice = if ambiguous {
            Some(DetectionNotice::Ambiguous)
        } else if unknown {
            Some(DetectionNotice::Unknown)
        } else {
            None
        };
    }

    /// The content type the lookup works with: the selected file's, else the
    /// first file's, else a series.
    fn active_content_type(&self) -> ContentType {
        self.selected
            .and_then(|index| self.results.get(index))
            .and_then(lookup::content_type_of)
            .or_else(|| self.results.first().and_then(lookup::content_type_of))
            .unwrap_or(ContentType::Series)
    }

    fn content_type_for_file(&self, file: &str) -> Option<ContentType> {
        self.results
            .iter()
            .find(|result| result.original == file)
            .and_then(lookup::content_type_of)
    }

    /// Sets the content type of the selected file, or of every file when none
    /// is selected.
    fn set_content_type(&mut self, content_type: ContentType) {
        match self.selected {
            Some(index) => {
                if let Some(result) = self.results.get_mut(index) {
                    result.content_type = Some(content_type);
                }
            }
            None => {
                for result in &mut self.results {
                    result.content_type = Some(content_type);
                }
            }
        }
    }

    fn template_slot(&mut self, content_type: ContentType) -> &mut Option<String> {
        match content_type {
            ContentType::Series => &mut self.format.series_template,
            ContentType::Movie => &mut self.format.movie_template,
        }
    }

    /// The custom template for this kind of content, empty when there is none.
    pub fn template(&self, content_type: ContentType) -> &str {
        match content_type {
            ContentType::Series => self.format.series_template.as_deref(),
            ContentType::Movie => self.format.movie_template.as_deref(),
        }
        .unwrap_or("")
    }

    /// The template used when no custom one is set.
    pub fn template_placeholder(&self, content_type: ContentType) -> &'static str {
        match content_type {
            ContentType::Series if self.format.include_episode_title => {
                formatting::DEFAULT_SERIES_FORMAT
            }
            ContentType::Series => "{n} {s}x{e}",
            ContentType::Movie if self.format.include_year => formatting::DEFAULT_MOVIE_FORMAT,
            ContentType::Movie => "{n}",
        }
    }

    /// Sets the custom template; an empty one goes back to the default.
    pub fn set_template(&mut self, content_type: ContentType, template: String) {
        *self.template_slot(content_type) = (!template.trim().is_empty()).then_some(template);
        self.format.save();
    }

    /// The parsed name of the selected file, or of the first one, when it
    /// says anything useful.
    fn search_hint(&self) -> Option<matching::ParsedName> {
        let file = self
            .selected
            .and_then(|index| self.files.get(index))
            .or_else(|| self.files.first())?;
        let parsed = matching::parse_filename(file);
        let useful = parsed.title.is_some()
            || parsed.season.is_some()
            || parsed.episode.is_some()
            || parsed.year.is_some();
        useful.then_some(parsed)
    }

    fn selected_title_match(&self) -> Option<&TitleMatch> {
        let id = self.selected_title.as_deref()?;
        self.titles.iter().find(|title| title.id == id)
    }

    fn details_file(&self) -> Option<String> {
        self.details
            .and_then(|index| self.files.get(index))
            .cloned()
    }

    fn details_override(&self) -> Option<&ManualOverride> {
        self.details
            .and_then(|index| self.files.get(index))
            .and_then(|file| self.overrides.get(file))
    }

    fn set_details_override(&mut self, entry: ManualOverride) {
        let Some(file) = self.details_file() else {
            return;
        };
        if entry.is_empty() {
            self.overrides.remove(&file);
        } else {
            self.overrides.insert(file, entry);
        }
    }

    fn apply_draft(&mut self) {
        if let Some(entry) = self.draft.parse() {
            self.set_details_override(entry);
        }
    }

    /// The name `result` would be renamed to, from the manual override, the
    /// lookup and the file name, in that order of precedence. `None` when the
    /// file name says nothing usable.
    fn format_preview(&self, result: &MatchResult) -> Option<String> {
        let detected = self.detected_name.trim();
        let detected = (!detected.is_empty()).then_some(detected);
        let content_type =
            lookup::content_type_of(result).unwrap_or_else(|| self.active_content_type());
        let format = || self.format.clone();
        let year = result
            .title_match
            .as_ref()
            .and_then(|title| title.year.map(u32::from));
        let parsed_series = result
            .metadata
            .as_ref()
            .and_then(|metadata| match metadata {
                MatchMetadata::Episode { series_title, .. } => series_title.as_deref(),
                MatchMetadata::Series { title } => title.as_deref(),
                MatchMetadata::Movie { .. } => None,
            });

        if let Some(entry) = self.overrides.get(&result.original) {
            let override_title = entry
                .title
                .as_deref()
                .map(str::trim)
                .filter(|title| !title.is_empty());
            let title = override_title
                .or_else(|| result.title_match.as_ref().map(|title| title.name.as_str()))
                .or(detected)
                .or(parsed_series);
            let hint = result
                .episode_match
                .as_ref()
                .map(|episode| (episode.season, episode.episode))
                .or_else(|| lookup::episode_hint(result));
            if entry.season.is_some() || entry.episode.is_some() || hint.is_some() {
                let (season, episode) = hint.unwrap_or((1, 1));
                return Some(formatting::format_series_name(
                    SeriesFormatInput {
                        series: title.unwrap_or("Unknown Series"),
                        season: entry.season.unwrap_or(season),
                        episode: entry.episode.unwrap_or(episode),
                        title: result.episode_match.as_ref().map(|e| e.title.as_str()),
                        year,
                    },
                    format(),
                ));
            }
            if let Some(title) = override_title {
                return Some(if content_type == ContentType::Movie {
                    formatting::format_movie_name(MovieFormatInput { title, year }, format())
                } else {
                    title.to_string()
                });
            }
        }

        if let Some(episode) = &result.episode_match {
            let series = result
                .title_match
                .as_ref()
                .map(|title| title.name.as_str())
                .or(detected)
                .or(match &result.metadata {
                    Some(MatchMetadata::Episode { series_title, .. }) => series_title.as_deref(),
                    _ => None,
                })
                .unwrap_or("Unknown Series");
            return Some(formatting::format_series_name(
                SeriesFormatInput {
                    series,
                    season: episode.season,
                    episode: episode.episode,
                    title: Some(episode.title.as_str()),
                    year,
                },
                format(),
            ));
        }

        if let Some(title) = &result.title_match {
            let title_year = title.year.map(u32::from);
            if content_type == ContentType::Movie
                || matches!(result.metadata, Some(MatchMetadata::Movie { .. }))
            {
                return Some(formatting::format_movie_name(
                    MovieFormatInput {
                        title: &title.name,
                        year: title_year,
                    },
                    format(),
                ));
            }
            if matches!(result.metadata, Some(MatchMetadata::Episode { .. })) {
                let (season, episode) = lookup::episode_hint(result).unwrap_or((1, 1));
                return Some(formatting::format_series_name(
                    SeriesFormatInput {
                        series: &title.name,
                        season,
                        episode,
                        title: None,
                        year: title_year,
                    },
                    format(),
                ));
            }
            return Some(title.name.clone());
        }

        Some(match result.metadata.as_ref()? {
            MatchMetadata::Series { title } => title
                .clone()
                .unwrap_or_else(|| "Unknown Series".to_string()),
            MatchMetadata::Episode {
                series_title,
                season,
                episode,
                episode_title,
            } => {
                let series = detected
                    .map(str::to_string)
                    .or_else(|| series_title.clone())
                    .unwrap_or_else(|| "Unknown Series".to_string());
                formatting::format_series_name(
                    SeriesFormatInput {
                        series: &series,
                        season: *season,
                        episode: *episode,
                        title: episode_title.as_deref().or(series_title.as_deref()),
                        year: None,
                    },
                    format(),
                )
            }
            MatchMetadata::Movie { title, year } => formatting::format_movie_name(
                MovieFormatInput {
                    title: title.as_deref().unwrap_or("Unknown Title"),
                    year: *year,
                },
                format(),
            ),
        })
    }
}

/// `path` itself when it is a file, or every file under it, in name order.
fn collect_files(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut children: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    children.sort();
    children
        .iter()
        .flat_map(|child| collect_files(child))
        .collect()
}

#[cfg(test)]
mod tests;
