mod formatting;
mod matching;
mod metadata;

use eframe::egui::{
    self, Button, Color32, FontId, Frame, Layout, RichText, ScrollArea, Stroke, Vec2,
};
use formatting::{
    DEFAULT_MOVIE_FORMAT, DEFAULT_SERIES_FORMAT, FormatOptions, MovieFormatInput, SeriesFormatInput,
};
use matching::ContentType;
use metadata::aggregate::MetadataPipeline;
use metadata::models::{EpisodeMatch, TitleMatch};
use metadata::provider::{MetadataProvider, MetadataSource};
use metadata::providers::{
    anidb::AniDbClient, omdb::OmdbClient, thetvdb::TheTvDbClient, tmdb::TmdbClient,
    tvmaze::TvMazeClient,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftNav {
    Rename,
    Episodes,
    Subtitles,
    Sfv,
    Filter,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsSection {
    Program,
    Language,
    Appearance,
    Experience,
    Utilities,
}

#[derive(Debug, Clone)]
enum FetchStatus {
    Idle,
    Loading,
    Ready,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RenameUiState {
    Loading,
    Success(usize),
    Error(String),
    Empty,
}

#[derive(Debug, Clone)]
enum RenameOutcome {
    Renamed,
    DryRun,
    Unchanged,
    Skipped(String),
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenameResultStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone)]
struct RenameSummary {
    original: String,
    proposed: String,
    resolved: String,
    collision_adjusted: bool,
    outcome: RenameOutcome,
    result_status: RenameResultStatus,
    message: String,
}

#[derive(Debug, Serialize)]
struct RenameExportRow {
    status: String,
    message: String,
    final_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct AppConfig {
    original_files: Vec<String>,
    match_results: Vec<matching::MatchResult>,
    format_options: FormatOptions,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            original_files: Vec::new(),
            match_results: Vec::new(),
            format_options: FormatOptions::default(),
        }
    }
}

impl AppConfig {
    fn load() -> Option<Self> {
        let path = Self::config_path()?;
        let contents = fs::read_to_string(path).ok()?;
        toml::from_str(&contents).ok()
    }

    fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!("Failed to create config directory: {err}");
                return;
            }
        }
        let Ok(payload) = toml::to_string_pretty(self) else {
            return;
        };
        if let Err(err) = fs::write(path, payload) {
            eprintln!("Failed to save config: {err}");
        }
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|base| base.join("Colony").join("KayaBot").join("config.toml"))
    }
}

struct RenameApp {
    active_left_nav: LeftNav,
    original_files: Vec<String>,
    match_results: Vec<matching::MatchResult>,
    selected_file_index: Option<usize>,
    detected_series_name: String,
    fetch_status: FetchStatus,
    title_matches: Vec<TitleMatch>,
    selected_title_id: Option<String>,
    episode_matches: Vec<EpisodeMatch>,
    show_match_picker: bool,
    match_picker_file_index: Option<usize>,
    manual_overrides: HashMap<String, ManualOverride>,
    rename_feedback_message: Option<String>,
    rename_dry_run: bool,
    rename_summaries: Vec<RenameSummary>,
    format_options: FormatOptions,
    metadata_provider: MetadataPipeline,
    rename_ui_state: RenameUiState,
    active_metadata_source: MetadataSource,
    preferred_movie_source: MetadataSource,
    preferred_series_source: MetadataSource,
    detection_notice: Option<String>,
    force_metadata_source: bool,
    api_config: ApiConfig,
    active_settings_section: SettingsSection,
    user_preferences: UserPreferences,
    system_visuals: Option<egui::Visuals>,
}

impl Default for RenameApp {
    fn default() -> Self {
        let app_config = AppConfig::load();
        let original_files = app_config
            .as_ref()
            .map(|config| config.original_files.clone())
            .unwrap_or_default();
        let match_results = app_config
            .as_ref()
            .map(|config| config.match_results.clone())
            .unwrap_or_default();
        let format_options = app_config
            .as_ref()
            .map(|config| config.format_options.clone())
            .or_else(FormatOptions::load)
            .unwrap_or_default();
        let rename_ui_state = if match_results.is_empty() {
            RenameUiState::Empty
        } else {
            RenameUiState::Success(match_results.len())
        };
        let api_config = ApiConfig::from_env();
        let mut metadata_provider = MetadataPipeline::new(vec![
            (
                MetadataSource::TheMovieDb,
                Box::new(TmdbClient::new(api_config.tmdb_token.clone())),
            ),
            (
                MetadataSource::AniDb,
                Box::new(AniDbClient::new(api_config.anidb_api_key.clone())),
            ),
            (
                MetadataSource::TheTvDb,
                Box::new(TheTvDbClient::new(api_config.tvdb_api_key.clone())),
            ),
            (
                MetadataSource::TvMaze,
                Box::new(TvMazeClient::new(api_config.tvmaze_user_agent.clone())),
            ),
            (
                MetadataSource::Omdb,
                Box::new(OmdbClient::new(api_config.omdb_api_key.clone())),
            ),
        ]);
        metadata_provider.set_active_sources(vec![
            MetadataSource::TheTvDb,
            MetadataSource::TvMaze,
            MetadataSource::AniDb,
        ]);

        let mut app = Self {
            active_left_nav: LeftNav::Rename,
            original_files,
            match_results,
            selected_file_index: None,
            detected_series_name: String::new(),
            fetch_status: FetchStatus::Idle,
            title_matches: Vec::new(),
            selected_title_id: None,
            episode_matches: Vec::new(),
            show_match_picker: false,
            match_picker_file_index: None,
            manual_overrides: HashMap::new(),
            rename_feedback_message: None,
            rename_dry_run: false,
            rename_summaries: Vec::new(),
            format_options,
            metadata_provider,
            rename_ui_state,
            active_metadata_source: MetadataSource::TheTvDb,
            preferred_movie_source: MetadataSource::TheMovieDb,
            preferred_series_source: MetadataSource::TheTvDb,
            detection_notice: None,
            force_metadata_source: false,
            api_config,
            active_settings_section: SettingsSection::Program,
            user_preferences: UserPreferences::load().unwrap_or_default(),
            system_visuals: None,
        };
        if !app.match_results.is_empty() {
            app.apply_content_detection();
            app.refresh_rename_ui_state();
        }
        app
    }
}

impl eframe::App for RenameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.system_visuals.is_none() {
            self.system_visuals = Some(ctx.style().visuals.clone());
        }
        self.apply_theme(ctx);
        self.handle_dropped_files(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.left_sidebar(ui);
                ui.add_space(10.0);
                self.main_content(ctx, ui);
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.user_preferences.save();
    }
}

impl RenameApp {
    fn apply_theme(&mut self, ctx: &egui::Context) {
        let visuals = match self.user_preferences.theme {
            ThemeChoice::System => self
                .system_visuals
                .clone()
                .unwrap_or_else(egui::Visuals::default),
            ThemeChoice::Latte => self.catppuccin_visuals(ThemeChoice::Latte),
            ThemeChoice::Frappe => self.catppuccin_visuals(ThemeChoice::Frappe),
            ThemeChoice::Macchiato => self.catppuccin_visuals(ThemeChoice::Macchiato),
            ThemeChoice::Mocha => self.catppuccin_visuals(ThemeChoice::Mocha),
        };
        ctx.set_visuals(visuals);
    }

    fn catppuccin_visuals(&self, theme: ThemeChoice) -> egui::Visuals {
        let palette = ThemePalette::from_theme(theme);
        let mut visuals = if theme == ThemeChoice::Latte {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };

        visuals.override_text_color = Some(palette.text);
        visuals.window_fill = palette.base;
        visuals.panel_fill = palette.mantle;
        visuals.faint_bg_color = palette.surface0;
        visuals.extreme_bg_color = palette.crust;
        visuals.widgets.noninteractive.bg_fill = palette.mantle;
        visuals.widgets.noninteractive.fg_stroke.color = palette.text;
        visuals.widgets.noninteractive.bg_stroke.color = palette.overlay0;
        visuals.widgets.inactive.bg_fill = palette.surface0;
        visuals.widgets.inactive.fg_stroke.color = palette.text;
        visuals.widgets.inactive.bg_stroke.color = palette.overlay0;
        visuals.widgets.hovered.bg_fill = palette.surface1;
        visuals.widgets.hovered.fg_stroke.color = palette.text;
        visuals.widgets.hovered.bg_stroke.color = palette.accent_border;
        visuals.widgets.active.bg_fill = palette.surface2;
        visuals.widgets.active.fg_stroke.color = palette.text;
        visuals.widgets.active.bg_stroke.color = palette.accent_border;
        visuals.widgets.open.bg_fill = palette.surface1;
        visuals.widgets.open.fg_stroke.color = palette.text;
        visuals.widgets.open.bg_stroke.color = palette.accent_border;
        visuals.selection.bg_fill = palette.accent;
        visuals.selection.stroke.color = palette.text;
        visuals.hyperlink_color = palette.accent;
        visuals.warn_fg_color = palette.warning;
        visuals.error_fg_color = palette.danger;
        visuals
    }

    fn theme_palette(&self) -> ThemePalette {
        ThemePalette::from_theme(self.user_preferences.theme)
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped_files = ctx.input(|input| input.raw.dropped_files.clone());
        if dropped_files.is_empty() {
            return;
        }

        let mut added = false;
        for dropped in dropped_files {
            if let Some(path) = dropped.path {
                let mut paths = Vec::new();
                self.collect_paths_from_input(&path, &mut paths);
                if self.add_original_files(paths) {
                    added = true;
                }
            } else if !dropped.name.is_empty() {
                if !self.original_files.contains(&dropped.name) {
                    self.original_files.push(dropped.name);
                    added = true;
                }
            }
        }

        if added {
            self.refresh_after_file_update();
        }
    }

    fn collect_paths_from_input(&self, path: &std::path::Path, collected: &mut Vec<PathBuf>) {
        if path.is_file() {
            collected.push(path.to_path_buf());
        } else if path.is_dir() {
            self.collect_files_in_dir(path, collected);
        }
    }

    fn collect_files_in_dir(&self, dir: &std::path::Path, collected: &mut Vec<PathBuf>) {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                collected.push(path);
            } else if path.is_dir() {
                self.collect_files_in_dir(&path, collected);
            }
        }
    }

    fn add_original_files<I>(&mut self, paths: I) -> bool
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let mut added = false;
        for path in paths {
            let path_string = path.display().to_string();
            if !self.original_files.contains(&path_string) {
                self.original_files.push(path_string);
                added = true;
            }
        }
        if added {
            self.selected_file_index = None;
        }
        added
    }

    fn refresh_after_file_update(&mut self) {
        self.match_results = matching::match_files(&self.original_files);
        self.apply_content_detection();
        self.rename_feedback_message = None;
        self.rename_summaries.clear();
        self.prune_manual_overrides();
        self.refresh_rename_ui_state();
        self.persist_config();
    }

    fn persist_config(&self) {
        let config = AppConfig {
            original_files: self.original_files.clone(),
            match_results: self.match_results.clone(),
            format_options: self.format_options.clone(),
        };
        config.save();
    }

    fn prune_manual_overrides(&mut self) {
        let valid_files: HashSet<String> = self.original_files.iter().cloned().collect();
        self.manual_overrides
            .retain(|original, _| valid_files.contains(original));
        if let Some(index) = self.match_picker_file_index {
            if index >= self.original_files.len() {
                self.match_picker_file_index = None;
            }
        }
    }

    fn move_selected_file_up(&mut self) {
        let Some(index) = self.selected_file_index else {
            return;
        };
        if index == 0 || index >= self.original_files.len() {
            return;
        }
        self.original_files.swap(index, index - 1);
        self.selected_file_index = Some(index - 1);
        self.refresh_after_file_update();
    }

    fn move_selected_file_down(&mut self) {
        let Some(index) = self.selected_file_index else {
            return;
        };
        if index + 1 >= self.original_files.len() {
            return;
        }
        self.original_files.swap(index, index + 1);
        self.selected_file_index = Some(index + 1);
        self.refresh_after_file_update();
    }

    fn delete_selected_file(&mut self) {
        let Some(index) = self.selected_file_index else {
            return;
        };
        if index >= self.original_files.len() {
            return;
        }
        self.original_files.remove(index);
        if self.original_files.is_empty() {
            self.selected_file_index = None;
        } else if index >= self.original_files.len() {
            self.selected_file_index = Some(self.original_files.len() - 1);
        }
        self.refresh_after_file_update();
    }

    fn rescan_original_files(&mut self) {
        self.original_files
            .retain(|path| std::path::Path::new(path).exists());
        if let Some(index) = self.selected_file_index {
            if index >= self.original_files.len() {
                self.selected_file_index = None;
            }
        }
        self.refresh_after_file_update();
    }

    fn load_files_from_picker(&mut self) {
        if let Some(files) = rfd::FileDialog::new().pick_files() {
            if self.add_original_files(files) {
                self.refresh_after_file_update();
            }
        }
    }

    fn load_folder_from_picker(&mut self) {
        let Some(folder) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let mut collected = Vec::new();
        self.collect_paths_from_input(&folder, &mut collected);
        if self.add_original_files(collected) {
            self.refresh_after_file_update();
        }
    }
    fn left_sidebar(&mut self, ui: &mut egui::Ui) {
        let sidebar_width = 110.0;
        let palette = self.theme_palette();
        let frame = Frame::none()
            .fill(palette.mantle)
            .stroke(Stroke::new(1.0, palette.overlay0))
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(6.0, 10.0));

        ui.allocate_ui_with_layout(
            Vec2::new(sidebar_width, ui.available_height()),
            Layout::top_down(egui::Align::Min),
            |ui| {
                frame.show(ui, |ui| {
                    ui.set_min_width(sidebar_width - 12.0);
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 10.0);

                    self.left_nav_button(ui, LeftNav::Rename, "📝", "Rename");
                    self.left_nav_button(ui, LeftNav::Episodes, "📀", "Episodes");
                    self.left_nav_button(ui, LeftNav::Subtitles, "💬", "Subtitles");
                    self.left_nav_button(ui, LeftNav::Sfv, "✅", "SFV");
                    self.left_nav_button(ui, LeftNav::Filter, "🧪", "Filter");
                    self.left_nav_button(ui, LeftNav::Settings, "⚙️", "Settings");
                });
            },
        );
    }

    fn left_nav_button(&mut self, ui: &mut egui::Ui, id: LeftNav, icon: &str, label: &str) {
        let palette = self.theme_palette();
        let is_active = self.active_left_nav == id;
        let fill = if is_active {
            palette.surface1
        } else {
            palette.surface0
        };
        let stroke = if is_active {
            Stroke::new(1.5, palette.accent_border)
        } else {
            Stroke::new(1.0, palette.overlay0)
        };

        let response = ui.add_sized(
            Vec2::new(86.0, 78.0),
            Button::new(
                RichText::new(format!("{icon}\n{label}"))
                    .size(12.0)
                    .color(palette.text),
            )
            .wrap(true)
            .fill(fill)
            .stroke(stroke),
        );

        if response.clicked() {
            self.active_left_nav = id;
            println!("Left nav clicked: {id:?}");
        }
    }

    fn main_content(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let available_width = ui.available_width();
        ui.allocate_ui_with_layout(
            Vec2::new(available_width, ui.available_height()),
            Layout::top_down(egui::Align::Min),
            |ui| match self.active_left_nav {
                LeftNav::Rename => self.rename_content(ctx, ui),
                LeftNav::Settings => self.settings_content(ui),
                LeftNav::Episodes | LeftNav::Subtitles | LeftNav::Sfv | LeftNav::Filter => {
                    self.placeholder_content(ui)
                }
            },
        );
    }

    fn rename_content(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        self.refresh_rename_ui_state();
        ui.add_space(8.0);
        let title = RichText::new("Rename").font(FontId::proportional(28.0));
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 40.0),
            Layout::centered_and_justified(egui::Direction::LeftToRight),
            |ui| {
                ui.label(title);
            },
        );

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let left_width = (ui.available_width() - 120.0) * 0.5;
            let right_width = left_width;
            let original_files = self
                .original_files
                .iter()
                .enumerate()
                .map(|(index, name)| OriginalFileRow {
                    index,
                    name: name.clone(),
                })
                .collect::<Vec<_>>();
            let match_rows = self.new_names_rows();

            Self::list_panel(
                self,
                ui,
                "original_files_panel",
                "Original Files",
                left_width,
                &original_files,
                |_, ui| {
                    ui.add_space(6.0);
                },
                |app, ui| {
                    ui.horizontal(|ui| {
                        let move_down_clicked = ui
                            .add_sized(Vec2::new(32.0, 26.0), Button::new("⬇"))
                            .clicked();
                        let move_up_clicked = ui
                            .add_sized(Vec2::new(32.0, 26.0), Button::new("⬆"))
                            .clicked();
                        let delete_clicked = ui
                            .add_sized(Vec2::new(32.0, 26.0), Button::new("❌"))
                            .clicked();
                        ui.menu_button("📂 Load", |ui| {
                            if ui.button("Fichiers…").clicked() {
                                app.load_files_from_picker();
                                ui.close_menu();
                            }
                            if ui.button("Dossier…").clicked() {
                                app.load_folder_from_picker();
                                ui.close_menu();
                            }
                        });
                        let refresh_clicked = ui
                            .add_sized(Vec2::new(32.0, 26.0), Button::new("🔄"))
                            .clicked();

                        if move_down_clicked {
                            app.move_selected_file_down();
                        }
                        if move_up_clicked {
                            app.move_selected_file_up();
                        }
                        if delete_clicked {
                            app.delete_selected_file();
                        }
                        if refresh_clicked {
                            app.rescan_original_files();
                        }
                    });
                },
            );

            ui.add_space(10.0);
            self.center_buttons(ui);
            ui.add_space(10.0);

            Self::list_panel(
                self,
                ui,
                "new_names_panel",
                "New Names",
                right_width,
                &match_rows,
                |app, ui| {
                    ui.add_space(6.0);
                    app.fetch_data_panel(ui);
                },
                |app, ui| {
                    ui.horizontal(|ui| {
                        ui.add_sized(Vec2::new(32.0, 26.0), Button::new("⬇"));
                        ui.add_sized(Vec2::new(32.0, 26.0), Button::new("⬆"));
                        ui.add_sized(Vec2::new(70.0, 26.0), Button::new("📂 Load"));
                        let fetch_label = match app.fetch_status {
                            FetchStatus::Loading => "Fetching...",
                            _ => "Fetch Data",
                        };
                        let fetch_clicked = ui
                            .add_enabled(
                                !matches!(app.fetch_status, FetchStatus::Loading),
                                Button::new(fetch_label),
                            )
                            .clicked();
                        if fetch_clicked {
                            app.fetch_metadata();
                        }
                        let adjust_clicked = ui
                            .add_sized(Vec2::new(32.0, 26.0), Button::new("🔧"))
                            .clicked();
                        if adjust_clicked {
                            if app.selected_file_index.is_none() && !app.original_files.is_empty() {
                                app.selected_file_index = Some(0);
                            }
                            app.match_picker_file_index = app.selected_file_index;
                            app.show_match_picker = app.selected_file_index.is_some();
                        }
                    });
                },
            );
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        self.rename_status_message(ui);

        if !self.rename_summaries.is_empty() {
            let palette = self.theme_palette();
            ui.add_space(10.0);
            Frame::none()
                .fill(palette.mantle)
                .stroke(Stroke::new(1.0, palette.overlay0))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(10.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Résumé de renommage").strong());
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Exporter JSON").clicked() {
                                self.export_rename_summary_json();
                            }
                            if ui.button("Exporter CSV").clicked() {
                                self.export_rename_summary_csv();
                            }
                        });
                    });
                    ui.add_space(6.0);
                    ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                        ui.columns(3, |columns| {
                            columns[0].label(RichText::new("Statut").strong());
                            columns[1].label(RichText::new("Message").strong());
                            columns[2].label(RichText::new("Chemin final").strong());
                        });
                        ui.separator();
                        for summary in &self.rename_summaries {
                            ui.columns(3, |columns| {
                                let status_color = match summary.result_status {
                                    RenameResultStatus::Ok => palette.success,
                                    RenameResultStatus::Error => palette.danger,
                                };
                                columns[0].label(
                                    RichText::new(Self::rename_status_label(
                                        summary.result_status,
                                    ))
                                    .color(status_color),
                                );
                                columns[1].label(&summary.message);
                                columns[2].label(&summary.resolved);
                            });
                        }
                    });
                });
        }

        if self.show_match_picker {
            self.match_picker_window(ctx);
        }
    }

    fn settings_content(&mut self, ui: &mut egui::Ui) {
        let palette = self.theme_palette();
        ui.add_space(8.0);
        let title = RichText::new("Settings").font(FontId::proportional(28.0));
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 40.0),
            Layout::centered_and_justified(egui::Direction::LeftToRight),
            |ui| {
                ui.label(title);
            },
        );
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(8.0);

        ui.vertical(|ui| {
            let available_width = ui.available_width();
            let menu_frame = Frame::none()
                .fill(palette.mantle)
                .stroke(Stroke::new(1.0, palette.overlay0))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(8.0, 8.0));

            menu_frame.show(ui, |ui| {
                ui.set_min_width(available_width);
                ui.label(RichText::new("Menu").strong());
                ui.add_space(6.0);
                self.settings_section_button(ui, SettingsSection::Program, "🧰", "Program");
                self.settings_section_button(ui, SettingsSection::Language, "🌍", "Language");
                self.settings_section_button(ui, SettingsSection::Appearance, "🎨", "Appearance");
                self.settings_section_button(ui, SettingsSection::Experience, "✨", "Experience");
                self.settings_section_button(ui, SettingsSection::Utilities, "🛠️", "Utilities");
            });

            ui.add_space(12.0);

            Frame::none()
                .fill(palette.base)
                .stroke(Stroke::new(1.0, palette.overlay0))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(12.0, 12.0))
                .show(ui, |ui| {
                    ui.set_min_width(available_width);
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let settings_changed = match self.active_settings_section {
                                SettingsSection::Program => self.settings_program(ui),
                                SettingsSection::Language => self.settings_language(ui),
                                SettingsSection::Appearance => self.settings_appearance(ui),
                                SettingsSection::Experience => self.settings_experience(ui),
                                SettingsSection::Utilities => self.settings_utilities(ui),
                            };
                            if settings_changed {
                                self.user_preferences.save();
                            }
                        });
                });
        });
    }

    fn settings_section_button(
        &mut self,
        ui: &mut egui::Ui,
        section: SettingsSection,
        icon: &str,
        label: &str,
    ) {
        let palette = self.theme_palette();
        let is_active = self.active_settings_section == section;
        let fill = if is_active {
            palette.surface1
        } else {
            palette.surface0
        };
        let stroke = if is_active {
            Stroke::new(1.5, palette.accent_border)
        } else {
            Stroke::new(1.0, palette.overlay0)
        };
        let response = ui.add_sized(
            Vec2::new(ui.available_width(), 34.0),
            Button::new(RichText::new(format!("{icon} {label}")).size(12.0))
                .fill(fill)
                .stroke(stroke),
        );
        if response.clicked() {
            self.active_settings_section = section;
        }
    }

    fn settings_program(&mut self, ui: &mut egui::Ui) -> bool {
        let palette = self.theme_palette();
        let mut changed = false;
        ui.label(RichText::new("Program").font(FontId::proportional(18.0)));
        ui.add_space(6.0);
        changed |= ui
            .checkbox(
                &mut self.user_preferences.open_last_session,
                "Restore last session on launch",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.auto_save_queue,
                "Auto-save rename queue",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.check_updates_on_launch,
                "Check for updates on launch",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.confirm_before_rename,
                "Ask for confirmation before renaming",
            )
            .changed();
        ui.add_space(12.0);
        ui.label(RichText::new("API configuration").strong());
        ui.label(
            RichText::new(
                "Keys are loaded from config.toml in the KayaBot config directory or \
environment variables.",
            )
            .size(11.0)
            .color(palette.subtext0),
        );
        if let Some(path) = ApiConfig::config_path() {
            ui.label(
                RichText::new(format!("Config path: {}", path.display()))
                    .size(11.0)
                    .color(palette.subtext0),
            );
        }
        ui.add_space(6.0);
        self.settings_status_row(ui, "TMDB", self.api_config.tmdb_configured());
        self.settings_status_row(ui, "TVDB", self.api_config.tvdb_configured());
        self.settings_status_row(ui, "OMDB", self.api_config.omdb_configured());
        self.settings_status_row(ui, "AniDB", self.api_config.anidb_configured());
        self.settings_status_row(ui, "TVMaze", self.api_config.tvmaze_configured());
        changed
    }

    fn settings_language(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.label(RichText::new("Language").font(FontId::proportional(18.0)));
        ui.add_space(6.0);
        let language_response = egui::ComboBox::from_id_source("settings_language")
            .selected_text(self.user_preferences.language.label())
            .show_ui(ui, |ui| {
                for language in LanguageChoice::all() {
                    ui.selectable_value(
                        &mut self.user_preferences.language,
                        language,
                        language.label(),
                    );
                }
            });
        changed |= language_response.response.changed();
        let region_response = egui::ComboBox::from_id_source("settings_region")
            .selected_text(self.user_preferences.region.label())
            .show_ui(ui, |ui| {
                for region in RegionChoice::all() {
                    ui.selectable_value(&mut self.user_preferences.region, region, region.label());
                }
            });
        changed |= region_response.response.changed();
        let date_response = egui::ComboBox::from_id_source("settings_date_format")
            .selected_text(self.user_preferences.date_format.label())
            .show_ui(ui, |ui| {
                for format in DateFormat::all() {
                    ui.selectable_value(
                        &mut self.user_preferences.date_format,
                        format,
                        format.label(),
                    );
                }
            });
        changed |= date_response.response.changed();
        changed
    }

    fn settings_appearance(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.label(RichText::new("Appearance").font(FontId::proportional(18.0)));
        ui.add_space(6.0);
        let theme_response = egui::ComboBox::from_id_source("settings_theme")
            .selected_text(self.user_preferences.theme.label())
            .show_ui(ui, |ui| {
                for theme in ThemeChoice::all() {
                    ui.selectable_value(&mut self.user_preferences.theme, theme, theme.label());
                }
            });
        changed |= theme_response.response.changed();
        let density_response = egui::ComboBox::from_id_source("settings_density")
            .selected_text(self.user_preferences.density.label())
            .show_ui(ui, |ui| {
                for density in DensityChoice::all() {
                    ui.selectable_value(
                        &mut self.user_preferences.density,
                        density,
                        density.label(),
                    );
                }
            });
        changed |= density_response.response.changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.show_section_headers,
                "Show section headers",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.animate_transitions,
                "Animate transitions",
            )
            .changed();
        changed
    }

    fn settings_experience(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.label(RichText::new("Experience").font(FontId::proportional(18.0)));
        ui.add_space(6.0);
        changed |= ui
            .checkbox(
                &mut self.user_preferences.show_tips,
                "Show tips and onboarding hints",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.enable_sound_cues,
                "Enable subtle sound cues",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.show_status_toasts,
                "Show status notifications",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.highlight_matches,
                "Highlight confident matches",
            )
            .changed();
        changed
    }

    fn settings_utilities(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.label(RichText::new("Utilities").font(FontId::proportional(18.0)));
        ui.add_space(6.0);
        changed |= ui
            .checkbox(
                &mut self.user_preferences.enable_quick_actions,
                "Enable quick actions toolbar",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.confirm_before_clearing,
                "Confirm before clearing lists",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.copy_results_to_clipboard,
                "Copy results to clipboard after rename",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.user_preferences.keep_logs,
                "Keep local activity logs",
            )
            .changed();
        ui.add_space(8.0);
        if ui.button("Reset preferences to defaults").clicked() {
            self.user_preferences = UserPreferences::default();
            changed = true;
        }
        changed
    }

    fn settings_status_row(&self, ui: &mut egui::Ui, label: &str, configured: bool) {
        let palette = self.theme_palette();
        let status = if configured { "Configured" } else { "Missing" };
        let color = if configured {
            palette.success
        } else {
            palette.danger
        };
        ui.horizontal(|ui| {
            ui.label(format!("{label}:"));
            ui.label(RichText::new(status).color(color));
        });
    }

    fn placeholder_content(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        let title = RichText::new("Coming soon").font(FontId::proportional(28.0));
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 40.0),
            Layout::centered_and_justified(egui::Direction::LeftToRight),
            |ui| {
                ui.label(title);
            },
        );
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label("This section is under construction.");
    }

    fn list_panel<C, F, T>(
        app: &mut RenameApp,
        ui: &mut egui::Ui,
        panel_id: &str,
        title: &str,
        width: f32,
        items: &[T],
        content: C,
        toolbar: F,
    ) where
        C: FnOnce(&mut RenameApp, &mut egui::Ui),
        F: FnOnce(&mut RenameApp, &mut egui::Ui),
        T: ListItem,
    {
        let palette = app.theme_palette();
        let panel_frame = Frame::none()
            .fill(palette.mantle)
            .stroke(Stroke::new(1.0, palette.overlay0))
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(8.0, 8.0));

        let response = ui.allocate_ui_with_layout(
            Vec2::new(width, ui.available_height()),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.push_id(panel_id, |ui| {
                    let available_height = ui.available_height();
                    panel_frame.show(ui, |ui| {
                        ui.set_min_height(available_height);
                        ui.label(RichText::new(title).size(14.0));
                        ui.add_space(4.0);

                        let remaining = ui.available_height();
                        let toolbar_height = 34.0;
                        let list_height = (remaining - toolbar_height).max(120.0);
                        Frame::none()
                            .stroke(Stroke::new(1.0, palette.overlay0))
                            .fill(palette.base)
                            .rounding(egui::Rounding::same(2.0))
                            .show(ui, |ui| {
                                ui.set_min_height(list_height);
                                ScrollArea::vertical()
                                    .id_source((panel_id, "list_scroll"))
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for item in items {
                                            item.render(app, ui);
                                        }
                                    });
                            });

                        content(app, ui);
                        ui.add_space(8.0);
                        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.set_min_height(toolbar_height);
                            toolbar(app, ui);
                        });
                    });
                });
            },
        );

        response.response.context_menu(|ui| {
            ui.push_id(panel_id, |ui| {
                app.fetch_match_context_menu(ui);
            });
        });
    }

    fn fetch_match_context_menu(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Fetch & Match Data:").strong());
        ui.add_space(4.0);
        ui.label(RichText::new("Episode Mode:").strong());
        ui.push_id("episode_sources", |ui| {
            let episode_sources = [
                MetadataSource::TheMovieDb,
                MetadataSource::AniDb,
                MetadataSource::TheTvDb,
                MetadataSource::TvMaze,
            ];
            for source in episode_sources {
                let selected = self.active_metadata_source == source;
                if ui.selectable_label(selected, source.label()).clicked() {
                    self.switch_metadata_source(source, ContentType::Series);
                    ui.close_menu();
                }
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new("Movie Mode:").strong());
        ui.push_id("movie_sources", |ui| {
            let movie_sources = [MetadataSource::TheMovieDb, MetadataSource::Omdb];
            for source in movie_sources {
                let selected = self.active_metadata_source == source;
                if ui.selectable_label(selected, source.label()).clicked() {
                    self.switch_metadata_source(source, ContentType::Movie);
                    ui.close_menu();
                }
            }
        });
    }

    fn switch_metadata_source(&mut self, source: MetadataSource, content_type: ContentType) {
        self.active_metadata_source = source;
        self.set_content_type_for_selected_or_all(content_type);
        self.refresh_active_sources(content_type);
        self.fetch_status = FetchStatus::Idle;
        self.title_matches.clear();
        self.episode_matches.clear();
        self.selected_title_id = None;
        self.rename_feedback_message = None;
        self.rename_summaries.clear();
        self.detection_notice = None;
        match content_type {
            ContentType::Movie => self.preferred_movie_source = source,
            ContentType::Series => self.preferred_series_source = source,
        }
        self.refresh_rename_ui_state();
    }

    fn refresh_active_sources(&mut self, content_type: ContentType) {
        let sources = if self.force_metadata_source {
            vec![self.active_metadata_source]
        } else {
            let mut sources = vec![self.active_metadata_source];
            sources.extend(self.fallback_sources(content_type, self.active_metadata_source));
            sources
        };
        self.metadata_provider.set_active_sources(sources);
    }

    fn fallback_sources(
        &self,
        content_type: ContentType,
        primary: MetadataSource,
    ) -> Vec<MetadataSource> {
        match content_type {
            ContentType::Movie => match primary {
                MetadataSource::TheMovieDb => vec![MetadataSource::Omdb],
                MetadataSource::Omdb => vec![MetadataSource::TheMovieDb],
                _ => vec![MetadataSource::TheMovieDb, MetadataSource::Omdb],
            },
            ContentType::Series => match primary {
                MetadataSource::TheTvDb => vec![MetadataSource::TvMaze, MetadataSource::AniDb],
                MetadataSource::TvMaze => vec![MetadataSource::TheTvDb, MetadataSource::AniDb],
                MetadataSource::AniDb => vec![MetadataSource::TheTvDb, MetadataSource::TvMaze],
                _ => vec![
                    MetadataSource::TheTvDb,
                    MetadataSource::TvMaze,
                    MetadataSource::AniDb,
                ],
            },
        }
    }

    fn center_buttons(&mut self, ui: &mut egui::Ui) {
        let palette = self.theme_palette();
        let frame = Frame::none()
            .fill(palette.mantle)
            .stroke(Stroke::new(1.0, palette.overlay0))
            .rounding(egui::Rounding::same(4.0));

        ui.allocate_ui_with_layout(
            Vec2::new(100.0, ui.available_height()),
            Layout::top_down(egui::Align::Center),
            |ui| {
                frame.show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), ui.available_height()),
                        Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.add_space(10.0);
                            self.large_action_button(ui, "⇆", "Match");
                            ui.add_space(20.0);
                            self.large_action_button(ui, "➜", "Rename");
                        },
                    );
                });
            },
        );
    }

    fn large_action_button(&mut self, ui: &mut egui::Ui, icon: &str, label: &str) {
        let palette = self.theme_palette();
        let response = ui.add_sized(
            Vec2::new(70.0, 70.0),
            Button::new(RichText::new(format!("{icon}\n{label}")).size(14.0))
                .wrap(true)
                .fill(palette.base)
                .stroke(Stroke::new(1.0, palette.overlay0)),
        );
        if response.clicked() {
            println!("Action clicked: {label}");
            if label == "Match" {
                self.match_results = self.match_files_with_metadata();
                self.apply_content_detection();
                self.rename_feedback_message = None;
                self.rename_summaries.clear();
                self.refresh_rename_ui_state();
                self.persist_config();
            } else if label == "Rename" {
                self.perform_rename();
            }
        }
    }

    fn perform_rename(&mut self) {
        self.rename_summaries.clear();
        self.rename_feedback_message = None;

        if self.match_results.is_empty() {
            self.rename_feedback_message = Some("Aucun résultat à renommer.".to_string());
            return;
        }

        let mut used_targets = HashSet::new();
        let mut updated_files = self.original_files.clone();

        for (index, original) in self.original_files.iter().enumerate() {
            let Some(result) = self.match_results.get(index) else {
                break;
            };
            let summary = self.rename_single_file(
                index,
                original,
                result,
                &mut used_targets,
                &mut updated_files,
            );
            self.rename_summaries.push(summary);
        }

        if !self.rename_dry_run {
            self.original_files = updated_files;
            self.manual_overrides.clear();
            self.match_results = matching::match_files(&self.original_files);
            self.apply_content_detection();
            self.persist_config();
        }

        self.refresh_rename_ui_state();
        self.rename_feedback_message = Some(self.rename_feedback_label());
    }

    fn rename_single_file(
        &self,
        index: usize,
        original: &str,
        result: &matching::MatchResult,
        used_targets: &mut HashSet<PathBuf>,
        updated_files: &mut [String],
    ) -> RenameSummary {
        let preview = self.format_preview(result);
        let original_path = PathBuf::from(original);

        if !original_path.exists() {
            return self.build_rename_summary(
                original,
                preview,
                original.to_string(),
                false,
                RenameOutcome::Failed("Fichier introuvable.".to_string()),
            );
        }

        if matches!(result.status, matching::MatchStatus::Error)
            || preview.trim().is_empty()
            || preview == "—"
        {
            return self.build_rename_summary(
                original,
                preview,
                original.to_string(),
                false,
                RenameOutcome::Skipped("Aucun nom proposé.".to_string()),
            );
        }

        let target_path = self.build_target_path(&original_path, &preview);
        let (resolved_path, collision_adjusted) =
            self.resolve_collision(&target_path, used_targets);
        let resolved_string = resolved_path.display().to_string();
        used_targets.insert(resolved_path.clone());

        if resolved_path == original_path {
            return self.build_rename_summary(
                original,
                preview,
                resolved_string,
                collision_adjusted,
                RenameOutcome::Unchanged,
            );
        }

        if self.rename_dry_run {
            return self.build_rename_summary(
                original,
                preview,
                resolved_string,
                collision_adjusted,
                RenameOutcome::DryRun,
            );
        }

        match self.rename_file(&original_path, &resolved_path) {
            Ok(()) => {
                if let Some(slot) = updated_files.get_mut(index) {
                    *slot = resolved_string.clone();
                }
                self.build_rename_summary(
                    original,
                    preview,
                    resolved_string,
                    collision_adjusted,
                    RenameOutcome::Renamed,
                )
            }
            Err(err) => self.build_rename_summary(
                original,
                preview,
                resolved_string,
                collision_adjusted,
                RenameOutcome::Failed(err),
            ),
        }
    }

    fn build_target_path(&self, original: &Path, preview: &str) -> PathBuf {
        let parent = original.parent().unwrap_or_else(|| Path::new(""));
        let extension = original.extension().and_then(|ext| ext.to_str());
        let file_name = match extension {
            Some(ext) => {
                let ext_suffix = format!(".{ext}");
                if preview.to_lowercase().ends_with(&ext_suffix.to_lowercase()) {
                    preview.to_string()
                } else {
                    format!("{preview}{ext_suffix}")
                }
            }
            None => preview.to_string(),
        };
        parent.join(file_name)
    }

    fn resolve_collision(&self, target: &Path, used_targets: &HashSet<PathBuf>) -> (PathBuf, bool) {
        if !target.exists() && !used_targets.contains(target) {
            return (target.to_path_buf(), false);
        }

        let parent = target.parent().unwrap_or_else(|| Path::new(""));
        let stem = target
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("file");
        let extension = target.extension().and_then(|value| value.to_str());

        for suffix in 1..=9999 {
            let file_name = match extension {
                Some(ext) => format!("{stem} ({suffix}).{ext}"),
                None => format!("{stem} ({suffix})"),
            };
            let candidate = parent.join(file_name);
            if !candidate.exists() && !used_targets.contains(&candidate) {
                return (candidate, true);
            }
        }

        (target.to_path_buf(), true)
    }

    fn rename_file(&self, original: &Path, target: &Path) -> Result<(), String> {
        if let Err(err) = fs::rename(original, target) {
            if let Err(copy_err) = fs::copy(original, target) {
                return Err(format!("Copie impossible: {copy_err}"));
            }
            if let Err(remove_err) = fs::remove_file(original) {
                return Err(format!("Suppression impossible: {remove_err}"));
            }
            if !Self::is_cross_device_link(&err) {
                eprintln!("Renommage direct échoué ({err}), copie + suppression appliquées.");
            }
        }
        Ok(())
    }

    fn is_cross_device_link(err: &std::io::Error) -> bool {
        #[cfg(unix)]
        {
            err.raw_os_error() == Some(18)
        }
        #[cfg(windows)]
        {
            err.raw_os_error() == Some(17)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = err;
            false
        }
    }

    fn rename_feedback_label(&self) -> String {
        let mut renamed = 0usize;
        let mut dry_run = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;
        let mut unchanged = 0usize;

        for summary in &self.rename_summaries {
            match summary.outcome {
                RenameOutcome::Renamed => renamed += 1,
                RenameOutcome::DryRun => dry_run += 1,
                RenameOutcome::Skipped(_) => skipped += 1,
                RenameOutcome::Failed(_) => failed += 1,
                RenameOutcome::Unchanged => unchanged += 1,
            }
        }

        if self.rename_dry_run {
            format!(
                "Prévisualisation: {dry_run} prêt(s) • {} ignoré(s) • {failed} échec(s)",
                skipped + unchanged
            )
        } else {
            format!(
                "Renommés: {renamed} • Ignorés: {} • Échecs: {failed}",
                skipped + unchanged
            )
        }
    }

    fn rename_result_status(outcome: &RenameOutcome) -> RenameResultStatus {
        match outcome {
            RenameOutcome::Failed(_) => RenameResultStatus::Error,
            _ => RenameResultStatus::Ok,
        }
    }

    fn rename_summary_message(
        &self,
        outcome: &RenameOutcome,
        collision_adjusted: bool,
        resolved: &str,
    ) -> String {
        let collision_note = if collision_adjusted {
            " (collision résolue)"
        } else {
            ""
        };
        match outcome {
            RenameOutcome::Renamed => format!("Renommé → {resolved}{collision_note}"),
            RenameOutcome::DryRun => format!("Dry-run → {resolved}{collision_note}"),
            RenameOutcome::Unchanged => format!("Déjà nommé → {resolved}{collision_note}"),
            RenameOutcome::Skipped(message) => format!("Ignoré: {message}"),
            RenameOutcome::Failed(message) => format!("Échec: {message}"),
        }
    }

    fn build_rename_summary(
        &self,
        original: &str,
        proposed: String,
        resolved: String,
        collision_adjusted: bool,
        outcome: RenameOutcome,
    ) -> RenameSummary {
        let message = self.rename_summary_message(&outcome, collision_adjusted, &resolved);
        let result_status = Self::rename_result_status(&outcome);
        RenameSummary {
            original: original.to_string(),
            proposed,
            resolved,
            collision_adjusted,
            outcome,
            result_status,
            message,
        }
    }

    fn rename_note_for_summary(
        &self,
        summary: &RenameSummary,
        palette: &ThemePalette,
    ) -> (String, Color32) {
        match &summary.outcome {
            RenameOutcome::Renamed => (summary.message.clone(), palette.success),
            RenameOutcome::DryRun => (summary.message.clone(), palette.warning),
            RenameOutcome::Unchanged => (summary.message.clone(), palette.subtext0),
            RenameOutcome::Skipped(_) => (summary.message.clone(), palette.subtext0),
            RenameOutcome::Failed(_) => (summary.message.clone(), palette.danger),
        }
    }

    fn rename_status_label(status: RenameResultStatus) -> &'static str {
        match status {
            RenameResultStatus::Ok => "ok",
            RenameResultStatus::Error => "erreur",
        }
    }

    fn rename_export_rows(&self) -> Vec<RenameExportRow> {
        self.rename_summaries
            .iter()
            .map(|summary| RenameExportRow {
                status: Self::rename_status_label(summary.result_status).to_string(),
                message: summary.message.clone(),
                final_path: summary.resolved.clone(),
            })
            .collect()
    }

    fn export_rename_summary_csv(&mut self) {
        if self.rename_summaries.is_empty() {
            self.rename_feedback_message = Some("Aucun résumé à exporter.".to_string());
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("rename-summary.csv")
            .save_file()
        else {
            return;
        };
        match self.write_rename_summary_csv(&path) {
            Ok(()) => {
                self.rename_feedback_message =
                    Some(format!("Résumé CSV exporté vers {}.", path.display()));
            }
            Err(err) => {
                self.rename_feedback_message =
                    Some(format!("Erreur export CSV: {err}"));
            }
        }
    }

    fn export_rename_summary_json(&mut self) {
        if self.rename_summaries.is_empty() {
            self.rename_feedback_message = Some("Aucun résumé à exporter.".to_string());
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("rename-summary.json")
            .save_file()
        else {
            return;
        };
        match self.write_rename_summary_json(&path) {
            Ok(()) => {
                self.rename_feedback_message =
                    Some(format!("Résumé JSON exporté vers {}.", path.display()));
            }
            Err(err) => {
                self.rename_feedback_message =
                    Some(format!("Erreur export JSON: {err}"));
            }
        }
    }

    fn write_rename_summary_csv(&self, path: &Path) -> Result<(), String> {
        let mut output = String::from("status,message,final_path\n");
        for row in self.rename_export_rows() {
            output.push_str(&format!(
                "{},{},{}\n",
                Self::csv_escape(&row.status),
                Self::csv_escape(&row.message),
                Self::csv_escape(&row.final_path)
            ));
        }
        fs::write(path, output).map_err(|err| err.to_string())
    }

    fn write_rename_summary_json(&self, path: &Path) -> Result<(), String> {
        let output = serde_json::to_string_pretty(&self.rename_export_rows())
            .map_err(|err| err.to_string())?;
        fs::write(path, output).map_err(|err| err.to_string())
    }

    fn csv_escape(value: &str) -> String {
        let needs_quotes = value.contains(['"', ',', '\n', '\r']);
        if needs_quotes {
            let escaped = value.replace('"', "\"\"");
            format!("\"{escaped}\"")
        } else {
            value.to_string()
        }
    }

    fn match_files_with_metadata(&mut self) -> Vec<matching::MatchResult> {
        let base_results = matching::match_files(&self.original_files);
        let mut results = Vec::with_capacity(base_results.len());
        for mut result in base_results {
            self.apply_metadata_match(&mut result);
            results.push(result);
        }
        results
    }

    fn content_type_for_result(&self, result: &matching::MatchResult) -> Option<ContentType> {
        result.content_type.or_else(|| match result.metadata {
            Some(matching::MatchMetadata::Movie { .. }) => Some(ContentType::Movie),
            Some(matching::MatchMetadata::Series { .. })
            | Some(matching::MatchMetadata::Episode { .. }) => Some(ContentType::Series),
            None => None,
        })
    }

    fn active_content_type(&self) -> ContentType {
        self.selected_file_index
            .and_then(|index| self.match_results.get(index))
            .and_then(|result| self.content_type_for_result(result))
            .or_else(|| {
                self.match_results
                    .first()
                    .and_then(|result| self.content_type_for_result(result))
            })
            .unwrap_or(ContentType::Series)
    }

    fn content_type_for_file(&self, file_name: &str) -> Option<ContentType> {
        self.match_results
            .iter()
            .find(|result| result.original == file_name)
            .and_then(|result| self.content_type_for_result(result))
    }

    fn set_content_type_for_selected_or_all(&mut self, content_type: ContentType) {
        if let Some(index) = self.selected_file_index {
            if let Some(result) = self.match_results.get_mut(index) {
                result.content_type = Some(content_type);
            }
        } else {
            for result in &mut self.match_results {
                result.content_type = Some(content_type);
            }
        }
    }

    fn apply_metadata_match(&mut self, result: &mut matching::MatchResult) {
        let content_type = self
            .content_type_for_result(result)
            .unwrap_or(ContentType::Series);
        if !self.force_metadata_source {
            self.active_metadata_source = match content_type {
                ContentType::Movie => self.preferred_movie_source,
                ContentType::Series => self.preferred_series_source,
            };
        }
        self.refresh_active_sources(content_type);

        let Some(query) = self.match_query_for_result(result) else {
            return;
        };
        let Ok(title_matches) = self.metadata_provider.search_title(&query) else {
            return;
        };
        result.metadata_candidate_count = Some(title_matches.len());
        let Some(best_title) = title_matches.iter().max_by(|a, b| {
            a.global_score
                .partial_cmp(&b.global_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) else {
            return;
        };
        result.metadata_confidence = Some(best_title.global_score);
        result.title_match = Some(best_title.clone());

        let Some((season, episode)) = self.episode_hint(result) else {
            return;
        };
        let Ok(episodes) = self.metadata_provider.fetch_episode_list(&best_title.id) else {
            return;
        };
        let mut candidate_count = 0usize;
        let mut best_episode: Option<&EpisodeMatch> = None;
        for episode_match in episodes.iter().filter(|episode_match| {
            episode_match.season == season && episode_match.episode == episode
        }) {
            candidate_count += 1;
            let should_replace = best_episode
                .as_ref()
                .map(|current| episode_match.global_score > current.global_score)
                .unwrap_or(true);
            if should_replace {
                best_episode = Some(episode_match);
            }
        }
        if candidate_count > 0 {
            result.metadata_candidate_count = Some(candidate_count);
        }
        if let Some(best_episode) = best_episode.cloned() {
            result.metadata_confidence = Some(best_episode.global_score);
            result.episode_match = Some(best_episode);
        }
    }

    fn match_query_for_result(&self, result: &matching::MatchResult) -> Option<String> {
        let detected = self.detected_series_name.trim();
        match &result.metadata {
            Some(matching::MatchMetadata::Series { title }) => title
                .clone()
                .or_else(|| (!detected.is_empty()).then(|| detected.to_string())),
            Some(matching::MatchMetadata::Episode { series_title, .. }) => series_title
                .clone()
                .or_else(|| (!detected.is_empty()).then(|| detected.to_string())),
            Some(matching::MatchMetadata::Movie { title, year }) => {
                let Some(title) = title.clone() else {
                    return None;
                };
                let mut query = title;
                if let Some(year) = year {
                    query = format!("{query} {year}");
                }
                Some(query)
            }
            None => None,
        }
    }

    fn episode_hint(&self, result: &matching::MatchResult) -> Option<(u32, u32)> {
        match &result.metadata {
            Some(matching::MatchMetadata::Episode {
                season, episode, ..
            }) => Some((*season, *episode)),
            _ => None,
        }
    }

    fn fetch_metadata(&mut self) {
        self.fetch_status = FetchStatus::Loading;
        self.rename_feedback_message = None;
        self.rename_summaries.clear();
        self.refresh_rename_ui_state();

        let parsed_hint = self.detect_search_hint();
        if self.detected_series_name.trim().is_empty() {
            if let Some(title) = parsed_hint.as_ref().and_then(|hint| hint.title.clone()) {
                self.detected_series_name = title;
            }
        }

        let mut content_type = self.active_content_type();
        if let Some(hint) = parsed_hint.as_ref() {
            let has_episode = hint.season.is_some() || hint.episode.is_some();
            let has_year = hint.year.is_some();
            if has_episode && content_type != ContentType::Series {
                self.switch_metadata_source(self.preferred_series_source, ContentType::Series);
                content_type = ContentType::Series;
            } else if has_year && !has_episode && content_type != ContentType::Movie {
                self.switch_metadata_source(self.preferred_movie_source, ContentType::Movie);
                content_type = ContentType::Movie;
            }
        }

        if !self.force_metadata_source {
            self.active_metadata_source = match content_type {
                ContentType::Movie => self.preferred_movie_source,
                ContentType::Series => self.preferred_series_source,
            };
        }
        self.refresh_active_sources(content_type);

        let mut query = self.detected_series_name.trim().to_string();
        if query.is_empty() {
            if let Some(title) = parsed_hint.as_ref().and_then(|hint| hint.title.clone()) {
                query = title;
            }
        }
        if content_type == ContentType::Movie {
            if let Some(year) = parsed_hint.as_ref().and_then(|hint| hint.year) {
                if !query.is_empty() {
                    query = format!("{query} {year}");
                }
            }
        }
        let query = query.trim();
        if query.is_empty() {
            self.fetch_status =
                FetchStatus::Error("Aucun titre détecté pour la recherche.".to_string());
            self.refresh_rename_ui_state();
            return;
        }
        match self.metadata_provider.search_title(query) {
            Ok(matches) => {
                self.title_matches = matches.clone();
                self.active_metadata_source = self.metadata_provider.active_source();
                if let Some(best) = matches.iter().max_by(|a, b| {
                    a.global_score
                        .partial_cmp(&b.global_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    self.selected_title_id = Some(best.id.clone());
                }
                if content_type == ContentType::Series {
                    self.fetch_episodes();
                } else {
                    self.episode_matches.clear();
                }
                self.fetch_status = FetchStatus::Ready;
            }
            Err(err) => {
                self.title_matches.clear();
                self.episode_matches.clear();
                self.selected_title_id = None;
                self.active_metadata_source = self.metadata_provider.active_source();
                self.fetch_status = FetchStatus::Error(err.to_string());
            }
        }
        self.refresh_rename_ui_state();
    }

    fn fetch_episodes(&mut self) {
        let Some(title_id) = self.selected_title_id.clone() else {
            self.fetch_status = FetchStatus::Error("Select a title match first.".to_string());
            return;
        };
        match self.metadata_provider.fetch_episode_list(&title_id) {
            Ok(episodes) => {
                self.episode_matches = episodes;
                self.active_metadata_source = self.metadata_provider.active_source();
            }
            Err(err) => {
                self.episode_matches.clear();
                self.active_metadata_source = self.metadata_provider.active_source();
                self.fetch_status = FetchStatus::Error(err.to_string());
            }
        }
        self.refresh_rename_ui_state();
    }

    fn detect_search_hint(&self) -> Option<matching::ParsedName> {
        let file = self
            .selected_file_index
            .and_then(|index| self.original_files.get(index))
            .or_else(|| self.original_files.first())?;
        let parsed = matching::parse_filename(file);
        if parsed.title.is_none()
            && parsed.season.is_none()
            && parsed.episode.is_none()
            && parsed.year.is_none()
        {
            None
        } else {
            Some(parsed)
        }
    }

    fn fetch_data_panel(&mut self, ui: &mut egui::Ui) {
        let palette = self.theme_palette();
        ui.group(|ui| {
            ui.label(RichText::new("Fetch Inputs").size(12.0));
            ui.horizontal(|ui| {
                let mut content_type = self.active_content_type();
                ui.label("Type");
                egui::ComboBox::from_id_source("content_type")
                    .selected_text(match content_type {
                        ContentType::Movie => "Movie",
                        ContentType::Series => "Series",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut content_type, ContentType::Movie, "Movie");
                        ui.selectable_value(&mut content_type, ContentType::Series, "Series");
                    });
                if content_type != self.active_content_type() {
                    self.set_content_type_for_selected_or_all(content_type);
                    if !self.force_metadata_source {
                        self.active_metadata_source = match content_type {
                            ContentType::Movie => self.preferred_movie_source,
                            ContentType::Series => self.preferred_series_source,
                        };
                    }
                    self.refresh_active_sources(content_type);
                    self.fetch_status = FetchStatus::Idle;
                    self.title_matches.clear();
                    self.episode_matches.clear();
                    self.selected_title_id = None;
                }
                ui.label("Series");
                ui.add(
                    egui::TextEdit::singleline(&mut self.detected_series_name)
                        .hint_text("Detected name")
                        .desired_width(140.0),
                );
            });
            let force_response = ui.checkbox(&mut self.force_metadata_source, "Forcer la source");
            if force_response.changed() {
                self.refresh_active_sources(self.active_content_type());
                self.fetch_status = FetchStatus::Idle;
                self.title_matches.clear();
                self.episode_matches.clear();
                self.selected_title_id = None;
            }
            if let Some(message) = &self.detection_notice {
                ui.add_space(4.0);
                ui.label(RichText::new(message).color(palette.warning));
            }

            match &self.fetch_status {
                FetchStatus::Loading => {
                    ui.label(RichText::new("Loading metadata...").color(palette.accent));
                }
                FetchStatus::Error(message) => {
                    ui.label(RichText::new(message).color(palette.danger));
                }
                _ => {}
            }

            ui.add_space(6.0);
            ui.label(RichText::new("Formatting").size(12.0));
            let mut format_changed = false;
            ui.horizontal(|ui| {
                format_changed |= ui
                    .checkbox(
                        &mut self.format_options.include_episode_title,
                        "Include episode title",
                    )
                    .changed();
                format_changed |= ui
                    .checkbox(&mut self.format_options.include_year, "Include year")
                    .changed();
            });
            let active_type = self.active_content_type();
            let default_template = match active_type {
                ContentType::Series => {
                    if self.format_options.include_episode_title {
                        DEFAULT_SERIES_FORMAT
                    } else {
                        "{n} {s}x{e}"
                    }
                }
                ContentType::Movie => {
                    if self.format_options.include_year {
                        DEFAULT_MOVIE_FORMAT
                    } else {
                        "{n}"
                    }
                }
            };
            let current_template = match active_type {
                ContentType::Series => self
                    .format_options
                    .series_template
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or(default_template),
                ContentType::Movie => self
                    .format_options
                    .movie_template
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or(default_template),
            };
            let mut template_input = current_template.to_string();
            let mut template_changed = false;
            let mut preset_clicked = false;
            let mut reset_clicked = false;
            ui.horizontal(|ui| {
                ui.label("Template");
                template_changed = ui
                    .add(
                        egui::TextEdit::singleline(&mut template_input)
                            .desired_width(220.0)
                            .hint_text("Template de renommage"),
                    )
                    .changed();
                preset_clicked = ui.button("Preset").clicked();
                reset_clicked = ui.button("Reset").clicked();
            });
            if preset_clicked {
                let preset_template = match active_type {
                    ContentType::Series => DEFAULT_SERIES_FORMAT,
                    ContentType::Movie => DEFAULT_MOVIE_FORMAT,
                };
                match active_type {
                    ContentType::Series => {
                        self.format_options.series_template = Some(preset_template.to_string());
                    }
                    ContentType::Movie => {
                        self.format_options.movie_template = Some(preset_template.to_string());
                    }
                }
                format_changed = true;
            } else if reset_clicked {
                match active_type {
                    ContentType::Series => {
                        self.format_options.series_template = None;
                        self.format_options.include_episode_title = true;
                    }
                    ContentType::Movie => {
                        self.format_options.movie_template = None;
                        self.format_options.include_year = true;
                    }
                }
                format_changed = true;
            } else if template_changed {
                let normalized = if template_input.trim().is_empty() {
                    None
                } else {
                    Some(template_input)
                };
                match active_type {
                    ContentType::Series => {
                        self.format_options.series_template = normalized;
                    }
                    ContentType::Movie => {
                        self.format_options.movie_template = normalized;
                    }
                }
                format_changed = true;
            }
            if format_changed {
                self.format_options.save();
                self.persist_config();
            }
            ui.label(
                RichText::new(default_template)
                    .size(10.0)
                    .color(palette.subtext0),
            );
            ui.add_space(6.0);
            ui.label(RichText::new("Rename").size(12.0));
            ui.checkbox(&mut self.rename_dry_run, "Dry-run / prévisualisation");
            ui.add_space(6.0);
            ui.label(RichText::new("Proposed Results").size(12.0));
            ui.label(
                RichText::new(format!(
                    "Source: {}{}",
                    self.metadata_provider.active_source().label(),
                    if self.force_metadata_source {
                        " (forcée)"
                    } else {
                        ""
                    }
                ))
                .size(10.0)
                .color(palette.subtext0),
            );
            if active_type == ContentType::Series {
                if self.episode_matches.is_empty() {
                    ui.label(RichText::new("No episodes loaded yet.").color(palette.subtext0));
                } else {
                    ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                        for episode in &self.episode_matches {
                            ui.label(format!(
                                "S{:02}E{:02} - {} ({})",
                                episode.season, episode.episode, episode.title, episode.id
                            ));
                        }
                    });
                }
            } else if self.title_matches.is_empty() {
                ui.label(RichText::new("No title matches yet.").color(palette.subtext0));
            } else {
                ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                    for title in &self.title_matches {
                        ui.label(self.format_title_match_summary(title));
                    }
                });
            }

            if let Some(message) = &self.rename_feedback_message {
                ui.add_space(6.0);
                ui.label(RichText::new(message).strong());
            }
        });
    }

    fn match_picker_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_match_picker;
        egui::Window::new("Ajuster le match")
            .open(&mut open)
            .show(ctx, |ui| {
                let Some(file_index) = self.match_picker_file_index.or(self.selected_file_index)
                else {
                    ui.label("Sélectionnez un fichier pour ajuster le match.");
                    return;
                };
                let Some(file_name) = self.original_files.get(file_index).cloned() else {
                    ui.label("Fichier introuvable.");
                    return;
                };
                let content_type = self
                    .content_type_for_file(&file_name)
                    .unwrap_or_else(|| self.active_content_type());
                ui.label(RichText::new(format!("Fichier: {file_name}")).strong());
                ui.add_space(6.0);

                let mut selected_id = None;
                let mut should_fetch = false;
                let title_matches = self.title_matches.clone();
                let episode_matches = self.episode_matches.clone();
                let selected_title_id = self.selected_title_id.clone();
                if self.title_matches.is_empty() {
                    ui.label("Fetch data first to see matches.");
                } else {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Matches disponibles").strong());
                            ui.add_space(4.0);
                            for title in &title_matches {
                                let label = self.format_title_match_summary(title);
                                let selected = selected_title_id.as_deref() == Some(&title.id);
                                if ui.selectable_label(selected, label).clicked() {
                                    selected_id = Some(title.id.clone());
                                    should_fetch = content_type == ContentType::Series;
                                    let mut override_entry = self
                                        .manual_overrides
                                        .get(&file_name)
                                        .cloned()
                                        .unwrap_or_default();
                                    override_entry.title = Some(title.name.clone());
                                    self.set_manual_override(&file_name, override_entry);
                                }
                            }
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            let palette = self.theme_palette();
                            ui.label(RichText::new("Source détaillée").strong());
                            ui.add_space(4.0);
                            if let Some(title) = self.selected_title_match() {
                                self.render_title_match_details(ui, title);
                            } else {
                                ui.label(
                                    RichText::new("Sélectionnez un match.").color(palette.subtext0),
                                );
                            }
                            if content_type == ContentType::Series {
                                ui.add_space(10.0);
                                ui.label(RichText::new("Épisodes").strong());
                                if self.episode_matches.is_empty() {
                                    ui.label(
                                        RichText::new("Aucun épisode chargé.")
                                            .color(palette.subtext0),
                                    );
                                } else {
                                    ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                                        let current_override = self
                                            .manual_override_for(&file_name)
                                            .cloned()
                                            .unwrap_or_default();
                                        for episode in &episode_matches {
                                            let label = format!(
                                                "S{:02}E{:02} - {}",
                                                episode.season, episode.episode, episode.title
                                            );
                                            let selected = current_override.season
                                                == Some(episode.season)
                                                && current_override.episode
                                                    == Some(episode.episode);
                                            if ui.selectable_label(selected, label).clicked() {
                                                let mut override_entry = current_override.clone();
                                                override_entry.season = Some(episode.season);
                                                override_entry.episode = Some(episode.episode);
                                                if override_entry.title.is_none() {
                                                    if let Some(title) = self.selected_title_match()
                                                    {
                                                        override_entry.title =
                                                            Some(title.name.clone());
                                                    }
                                                }
                                                self.set_manual_override(
                                                    &file_name,
                                                    override_entry,
                                                );
                                            }
                                        }
                                    });
                                }
                            }
                        });
                    });
                }
                if let Some(selected_id) = selected_id {
                    self.selected_title_id = Some(selected_id);
                    if should_fetch {
                        self.fetch_episodes();
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(RichText::new("Override manuel").strong());
                let palette = self.theme_palette();
                let current_override = self
                    .manual_override_for(&file_name)
                    .cloned()
                    .unwrap_or_default();
                let mut title_input = current_override.title.unwrap_or_default();
                let mut season_input = current_override
                    .season
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                let mut episode_input = current_override
                    .episode
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                let mut override_changed = false;
                ui.horizontal(|ui| {
                    ui.label("Titre");
                    override_changed |= ui.text_edit_singleline(&mut title_input).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Saison");
                    override_changed |= ui.text_edit_singleline(&mut season_input).changed();
                    ui.label("Épisode");
                    override_changed |= ui.text_edit_singleline(&mut episode_input).changed();
                });

                let season_value = if season_input.trim().is_empty() {
                    None
                } else {
                    season_input.trim().parse::<u32>().ok()
                };
                let episode_value = if episode_input.trim().is_empty() {
                    None
                } else {
                    episode_input.trim().parse::<u32>().ok()
                };
                let invalid_season = !season_input.trim().is_empty() && season_value.is_none();
                let invalid_episode = !episode_input.trim().is_empty() && episode_value.is_none();
                if override_changed && !invalid_season && !invalid_episode {
                    let title_value = if title_input.trim().is_empty() {
                        None
                    } else {
                        Some(title_input.trim().to_string())
                    };
                    self.set_manual_override(
                        &file_name,
                        ManualOverride {
                            title: title_value,
                            season: season_value,
                            episode: episode_value,
                        },
                    );
                }
                if invalid_season || invalid_episode {
                    ui.label(
                        RichText::new("Saison/épisode invalide.")
                            .size(10.0)
                            .color(palette.warning),
                    );
                }
                if ui.button("Effacer l'override").clicked() {
                    self.manual_overrides.remove(&file_name);
                }
            });
        self.show_match_picker = open;
        if !open {
            self.match_picker_file_index = None;
        }
    }

    fn new_names_rows(&self) -> Vec<NewNameRow> {
        let palette = self.theme_palette();
        match &self.rename_ui_state {
            RenameUiState::Loading => vec![NewNameRow {
                name: "Chargement des résultats…".to_string(),
                status: None,
                original: None,
                content_type_label: None,
                confidence: None,
                candidate_count: None,
                rename_note: None,
                status_color: None,
                muted_color: palette.subtext0,
            }],
            RenameUiState::Error(message) => {
                vec![NewNameRow {
                    name: format!("Erreur: {message}"),
                    status: None,
                    original: None,
                    content_type_label: None,
                    confidence: None,
                    candidate_count: None,
                    rename_note: None,
                    status_color: None,
                    muted_color: palette.subtext0,
                }]
            }
            RenameUiState::Empty => vec![NewNameRow {
                name: "Aucun résultat disponible.".to_string(),
                status: None,
                original: None,
                content_type_label: None,
                confidence: None,
                candidate_count: None,
                rename_note: None,
                status_color: None,
                muted_color: palette.subtext0,
            }],
            RenameUiState::Success(_) => self
                .match_results
                .iter()
                .map(|result| NewNameRow {
                    name: self.format_preview(result),
                    status: Some(result.status),
                    original: Some(result.original.clone()),
                    content_type_label: self.content_type_for_result(result).map(|content_type| {
                        match content_type {
                            ContentType::Movie => "Movie".to_string(),
                            ContentType::Series => "Series".to_string(),
                        }
                    }),
                    confidence: result
                        .metadata_confidence
                        .or_else(|| Some(result.confidence)),
                    candidate_count: result
                        .metadata_candidate_count
                        .or_else(|| Some(result.candidates.len())),
                    rename_note: self
                        .rename_summaries
                        .iter()
                        .find(|summary| summary.original == result.original)
                        .map(|summary| self.rename_note_for_summary(summary, &palette)),
                    status_color: Some(match result.status {
                        matching::MatchStatus::Ok => palette.success,
                        matching::MatchStatus::Ambiguous => palette.warning,
                        matching::MatchStatus::Error => palette.danger,
                    }),
                    muted_color: palette.subtext0,
                })
                .collect(),
        }
    }

    fn rename_status_message(&self, ui: &mut egui::Ui) {
        let palette = self.theme_palette();
        let (message, color) = match &self.rename_ui_state {
            RenameUiState::Loading => ("Statut: chargement…".to_string(), palette.accent),
            RenameUiState::Success(count) => (
                format!("Statut: succès — {count} résultat(s) prêts."),
                palette.success,
            ),
            RenameUiState::Error(message) => {
                (format!("Statut: erreur — {message}"), palette.danger)
            }
            RenameUiState::Empty => (
                "Statut: aucun résultat pour le moment.".to_string(),
                palette.subtext0,
            ),
        };
        ui.label(RichText::new(message).color(color));
    }

    fn refresh_rename_ui_state(&mut self) {
        let next_state = match &self.fetch_status {
            FetchStatus::Loading => RenameUiState::Loading,
            FetchStatus::Error(message) => RenameUiState::Error(message.clone()),
            FetchStatus::Idle | FetchStatus::Ready => {
                if self.match_results.is_empty() {
                    RenameUiState::Empty
                } else {
                    RenameUiState::Success(self.match_results.len())
                }
            }
        };
        if self.rename_ui_state != next_state {
            self.rename_ui_state = next_state;
        }
    }

    fn apply_content_detection(&mut self) {
        use matching::ContentGuess;

        self.detection_notice = None;
        let mut has_ambiguous = false;
        let mut has_unknown = false;

        for result in &mut self.match_results {
            match matching::guess_content_type_for_result(result) {
                ContentGuess::Series => result.content_type = Some(ContentType::Series),
                ContentGuess::Movie => result.content_type = Some(ContentType::Movie),
                ContentGuess::Ambiguous => {
                    result.content_type = None;
                    has_ambiguous = true;
                }
                ContentGuess::Unknown => {
                    result.content_type = None;
                    has_unknown = true;
                }
            }
        }

        if has_ambiguous {
            self.detection_notice = Some(
                "Type ambigu détecté pour certains fichiers. Choisissez Film ou Série.".to_string(),
            );
        } else if has_unknown {
            self.detection_notice =
                Some("Type inconnu pour certains fichiers. Choisissez Film ou Série.".to_string());
        }

        if !self.match_results.is_empty() {
            let content_type = self.active_content_type();
            if !self.force_metadata_source {
                let preferred = match content_type {
                    ContentType::Movie => self.preferred_movie_source,
                    ContentType::Series => self.preferred_series_source,
                };
                self.active_metadata_source = preferred;
            }
            self.refresh_active_sources(content_type);
        }
    }

    fn manual_override_for(&self, original: &str) -> Option<&ManualOverride> {
        self.manual_overrides.get(original)
    }

    fn set_manual_override(&mut self, original: &str, override_entry: ManualOverride) {
        if override_entry.is_empty() {
            self.manual_overrides.remove(original);
        } else {
            self.manual_overrides
                .insert(original.to_string(), override_entry);
        }
    }

    fn format_preview(&self, result: &matching::MatchResult) -> String {
        let detected = self.detected_series_name.trim();
        let content_type = self
            .content_type_for_result(result)
            .unwrap_or_else(|| self.active_content_type());
        if let Some(override_entry) = self.manual_override_for(&result.original) {
            let override_title = override_entry
                .title
                .as_deref()
                .map(str::trim)
                .filter(|title| !title.is_empty());
            let title_fallback = result
                .title_match
                .as_ref()
                .map(|title| title.name.as_str())
                .or_else(|| (!detected.is_empty()).then_some(detected))
                .or_else(|| {
                    result
                        .metadata
                        .as_ref()
                        .and_then(|metadata| match metadata {
                            matching::MatchMetadata::Episode { series_title, .. } => {
                                series_title.as_deref()
                            }
                            matching::MatchMetadata::Series { title } => title.as_deref(),
                            _ => None,
                        })
                });
            let title = override_title.or(title_fallback);
            let year = result
                .title_match
                .as_ref()
                .and_then(|title| title.year.map(u32::from));
            let episode_hint = result
                .episode_match
                .as_ref()
                .map(|episode| (episode.season, episode.episode))
                .or_else(|| self.episode_hint(result));
            let has_episode = override_entry.season.is_some()
                || override_entry.episode.is_some()
                || episode_hint.is_some();
            if has_episode {
                let (fallback_season, fallback_episode) = episode_hint.unwrap_or((1, 1));
                let season = override_entry.season.unwrap_or(fallback_season);
                let episode = override_entry.episode.unwrap_or(fallback_episode);
                return formatting::format_series_name(
                    SeriesFormatInput {
                        series: title.unwrap_or("Unknown Series"),
                        season,
                        episode,
                        title: result
                            .episode_match
                            .as_ref()
                            .map(|episode| episode.title.as_str()),
                        year,
                    },
                    self.format_options.clone(),
                );
            }
            if let Some(override_title) = override_title {
                if content_type == ContentType::Movie {
                    return formatting::format_movie_name(
                        MovieFormatInput {
                            title: override_title,
                            year,
                        },
                        self.format_options.clone(),
                    );
                }
                return override_title.to_string();
            }
        }
        if let Some(episode_match) = &result.episode_match {
            let series_name = result
                .title_match
                .as_ref()
                .map(|title| title.name.as_str())
                .or_else(|| (!detected.is_empty()).then_some(detected))
                .or_else(|| {
                    result
                        .metadata
                        .as_ref()
                        .and_then(|metadata| match metadata {
                            matching::MatchMetadata::Episode { series_title, .. } => {
                                series_title.as_deref()
                            }
                            _ => None,
                        })
                })
                .unwrap_or("Unknown Series");
            return formatting::format_series_name(
                SeriesFormatInput {
                    series: series_name,
                    season: episode_match.season,
                    episode: episode_match.episode,
                    title: Some(episode_match.title.as_str()),
                    year: result
                        .title_match
                        .as_ref()
                        .and_then(|title| title.year.map(u32::from)),
                },
                self.format_options.clone(),
            );
        }

        if content_type == ContentType::Movie {
            if let Some(title_match) = &result.title_match {
                return formatting::format_movie_name(
                    MovieFormatInput {
                        title: &title_match.name,
                        year: title_match.year.map(u32::from),
                    },
                    self.format_options.clone(),
                );
            }
        }

        if let Some(title_match) = &result.title_match {
            let year = title_match.year.map(|year| year as u32);
            let metadata = result.metadata.as_ref();
            let is_episode = matches!(metadata, Some(matching::MatchMetadata::Episode { .. }));
            if is_episode {
                let (season, episode) = self.episode_hint(result).unwrap_or((1, 1));
                return formatting::format_series_name(
                    SeriesFormatInput {
                        series: &title_match.name,
                        season,
                        episode,
                        title: None,
                        year,
                    },
                    self.format_options.clone(),
                );
            }
            if matches!(metadata, Some(matching::MatchMetadata::Movie { .. })) {
                return formatting::format_movie_name(
                    MovieFormatInput {
                        title: &title_match.name,
                        year,
                    },
                    self.format_options.clone(),
                );
            }
            return title_match.name.clone();
        }

        let Some(metadata) = &result.metadata else {
            return "—".to_string();
        };

        match metadata {
            matching::MatchMetadata::Series { title } => title
                .clone()
                .unwrap_or_else(|| "Unknown Series".to_string()),
            matching::MatchMetadata::Episode {
                series_title,
                season,
                episode,
                episode_title,
            } => {
                let series_name = if detected.is_empty() {
                    series_title
                        .clone()
                        .unwrap_or_else(|| "Unknown Series".to_string())
                } else {
                    detected.to_string()
                };
                let resolved_title = episode_title.as_deref().or_else(|| series_title.as_deref());

                formatting::format_series_name(
                    SeriesFormatInput {
                        series: &series_name,
                        season: *season,
                        episode: *episode,
                        title: resolved_title,
                        year: None,
                    },
                    self.format_options.clone(),
                )
            }
            matching::MatchMetadata::Movie { title, year } => {
                let movie_title = title.as_deref().unwrap_or("Unknown Title");
                formatting::format_movie_name(
                    MovieFormatInput {
                        title: movie_title,
                        year: *year,
                    },
                    self.format_options.clone(),
                )
            }
        }
    }

    fn format_title_match_summary(&self, title: &TitleMatch) -> String {
        let language = title.extras.language.as_deref().unwrap_or("—");
        let year = title
            .year
            .map(|year| year.to_string())
            .unwrap_or_else(|| "—".to_string());
        format!(
            "{} ({} • score {:.2} • lang {} • année {})",
            title.name, title.source, title.global_score, language, year
        )
    }

    fn selected_title_match(&self) -> Option<&TitleMatch> {
        let selected_id = self.selected_title_id.as_deref()?;
        self.title_matches
            .iter()
            .find(|title| title.id == selected_id)
    }

    fn render_title_match_details(&self, ui: &mut egui::Ui, title: &TitleMatch) {
        let palette = self.theme_palette();
        ui.label(format!("Titre: {}", title.name));
        ui.label(format!("Source: {}", title.source));
        ui.label(format!(
            "Score global: {:.2} (source {:.2} • confiance {:.2})",
            title.global_score, title.source_score, title.source_trust
        ));
        ui.label(format!(
            "Langue: {}",
            title.extras.language.as_deref().unwrap_or("—")
        ));
        ui.label(format!(
            "Année: {}",
            title
                .year
                .map(|year| year.to_string())
                .unwrap_or_else(|| "—".to_string())
        ));
        ui.label(format!("ID source: {}", title.id));

        let mut external_ids = Vec::new();
        let ids = &title.extras.external_ids;
        if let Some(id) = ids.imdb.as_deref() {
            external_ids.push(format!("imdb:{id}"));
        }
        if let Some(id) = ids.tmdb.as_deref() {
            external_ids.push(format!("tmdb:{id}"));
        }
        if let Some(id) = ids.tvdb.as_deref() {
            external_ids.push(format!("tvdb:{id}"));
        }
        if let Some(id) = ids.tvmaze.as_deref() {
            external_ids.push(format!("tvmaze:{id}"));
        }
        if let Some(id) = ids.anidb.as_deref() {
            external_ids.push(format!("anidb:{id}"));
        }
        if let Some(id) = ids.omdb.as_deref() {
            external_ids.push(format!("omdb:{id}"));
        }
        for other in &ids.other {
            external_ids.push(format!("{}:{}", other.source, other.id));
        }
        let external_line = if external_ids.is_empty() {
            "IDs externes: —".to_string()
        } else {
            format!("IDs externes: {}", external_ids.join(", "))
        };
        ui.label(external_line);

        if !title.extras.aliases.is_empty() {
            ui.label(format!("Alias: {}", title.extras.aliases.join(", ")));
        }
        if !title.extras.genres.is_empty() {
            ui.label(format!("Genres: {}", title.extras.genres.join(", ")));
        }
        if let Some(synopsis) = title.extras.synopsis.as_deref() {
            ui.add_space(4.0);
            ui.label(RichText::new("Synopsis").strong());
            ui.label(RichText::new(synopsis).size(10.0).color(palette.subtext0));
        }
    }
}

#[derive(Debug, Clone)]
struct ApiConfig {
    tmdb_token: String,
    tvdb_api_key: String,
    omdb_api_key: String,
    anidb_api_key: String,
    tvmaze_user_agent: String,
}

impl ApiConfig {
    fn from_env() -> Self {
        let config = ConfigFile::load();
        let tmdb_token = Self::env_value("KAYABOT_TMDB_BEARER_TOKEN")
            .or_else(|| Self::env_value("KAYABOT_TMDB_API_KEY"))
            .or_else(|| {
                config
                    .as_ref()
                    .and_then(|cfg| cfg.tmdb_bearer_token.clone())
            })
            .or_else(|| config.as_ref().and_then(|cfg| cfg.tmdb_api_key.clone()))
            .unwrap_or_default();
        let tvdb_api_key = Self::env_value("KAYABOT_TVDB_API_KEY")
            .or_else(|| config.as_ref().and_then(|cfg| cfg.tvdb_api_key.clone()))
            .unwrap_or_default();
        let omdb_api_key = Self::env_value("KAYABOT_OMDB_API_KEY")
            .or_else(|| config.as_ref().and_then(|cfg| cfg.omdb_api_key.clone()))
            .unwrap_or_default();
        let anidb_api_key = Self::env_value("KAYABOT_ANIDB_PASSWORD")
            .or_else(|| Self::env_value("KAYABOT_ANIDB_API_KEY"))
            .or_else(|| config.as_ref().and_then(|cfg| cfg.anidb_password.clone()))
            .or_else(|| config.as_ref().and_then(|cfg| cfg.anidb_api_key.clone()))
            .unwrap_or_default();
        let tvmaze_user_agent = Self::env_value("KAYABOT_TVMAZE_USER_AGENT")
            .or_else(|| Self::env_value("KAYABOT_TVMAZE_API_KEY"))
            .or_else(|| {
                config
                    .as_ref()
                    .and_then(|cfg| cfg.tvmaze_user_agent.clone())
            })
            .or_else(|| config.as_ref().and_then(|cfg| cfg.tvmaze_api_key.clone()))
            .unwrap_or_else(|| "KayaBot".to_string());

        Self {
            tmdb_token,
            tvdb_api_key,
            omdb_api_key,
            anidb_api_key,
            tvmaze_user_agent,
        }
    }

    fn env_value(key: &str) -> Option<String> {
        env::var(key).ok().and_then(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        })
    }

    fn config_path() -> Option<PathBuf> {
        config_root().map(|root| root.join("config.toml"))
    }

    fn tmdb_configured(&self) -> bool {
        !self.tmdb_token.trim().is_empty()
    }

    fn tvdb_configured(&self) -> bool {
        !self.tvdb_api_key.trim().is_empty()
    }

    fn omdb_configured(&self) -> bool {
        !self.omdb_api_key.trim().is_empty()
    }

    fn anidb_configured(&self) -> bool {
        !self.anidb_api_key.trim().is_empty()
    }

    fn tvmaze_configured(&self) -> bool {
        !self.tvmaze_user_agent.trim().is_empty()
    }
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct ConfigFile {
    tmdb_bearer_token: Option<String>,
    tmdb_api_key: Option<String>,
    tvdb_api_key: Option<String>,
    omdb_api_key: Option<String>,
    anidb_password: Option<String>,
    anidb_api_key: Option<String>,
    tvmaze_user_agent: Option<String>,
    tvmaze_api_key: Option<String>,
}

impl ConfigFile {
    fn load() -> Option<Self> {
        let path = ApiConfig::config_path()?;
        let contents = fs::read_to_string(path).ok()?;
        toml::from_str(&contents).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum LanguageChoice {
    System,
    English,
    French,
    Spanish,
}

impl LanguageChoice {
    fn label(self) -> &'static str {
        match self {
            LanguageChoice::System => "System default",
            LanguageChoice::English => "English",
            LanguageChoice::French => "Français",
            LanguageChoice::Spanish => "Español",
        }
    }

    fn all() -> [Self; 4] {
        [
            LanguageChoice::System,
            LanguageChoice::English,
            LanguageChoice::French,
            LanguageChoice::Spanish,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum RegionChoice {
    Auto,
    France,
    UnitedStates,
    Japan,
}

impl RegionChoice {
    fn label(self) -> &'static str {
        match self {
            RegionChoice::Auto => "Auto",
            RegionChoice::France => "France",
            RegionChoice::UnitedStates => "United States",
            RegionChoice::Japan => "Japan",
        }
    }

    fn all() -> [Self; 4] {
        [
            RegionChoice::Auto,
            RegionChoice::France,
            RegionChoice::UnitedStates,
            RegionChoice::Japan,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum DateFormat {
    System,
    DdMmYyyy,
    MmDdYyyy,
    YyyyMmDd,
}

impl DateFormat {
    fn label(self) -> &'static str {
        match self {
            DateFormat::System => "System default",
            DateFormat::DdMmYyyy => "DD/MM/YYYY",
            DateFormat::MmDdYyyy => "MM/DD/YYYY",
            DateFormat::YyyyMmDd => "YYYY-MM-DD",
        }
    }

    fn all() -> [Self; 4] {
        [
            DateFormat::System,
            DateFormat::DdMmYyyy,
            DateFormat::MmDdYyyy,
            DateFormat::YyyyMmDd,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum ThemeChoice {
    System,
    Latte,
    Frappe,
    Macchiato,
    Mocha,
}

impl ThemeChoice {
    fn label(self) -> &'static str {
        match self {
            ThemeChoice::System => "System",
            ThemeChoice::Latte => "Latte",
            ThemeChoice::Frappe => "Frappé",
            ThemeChoice::Macchiato => "Macchiato",
            ThemeChoice::Mocha => "Mocha",
        }
    }

    fn all() -> [Self; 5] {
        [
            ThemeChoice::System,
            ThemeChoice::Latte,
            ThemeChoice::Frappe,
            ThemeChoice::Macchiato,
            ThemeChoice::Mocha,
        ]
    }
}

#[derive(Debug, Clone, Copy)]
struct ThemePalette {
    base: Color32,
    mantle: Color32,
    crust: Color32,
    surface0: Color32,
    surface1: Color32,
    surface2: Color32,
    overlay0: Color32,
    text: Color32,
    subtext0: Color32,
    accent: Color32,
    accent_border: Color32,
    success: Color32,
    warning: Color32,
    danger: Color32,
}

impl ThemePalette {
    fn from_theme(theme: ThemeChoice) -> Self {
        match theme {
            ThemeChoice::System => Self::system_default(),
            ThemeChoice::Latte => Self {
                base: Color32::from_rgb(239, 241, 245),
                mantle: Color32::from_rgb(230, 233, 239),
                crust: Color32::from_rgb(220, 224, 232),
                surface0: Color32::from_rgb(204, 208, 218),
                surface1: Color32::from_rgb(188, 192, 204),
                surface2: Color32::from_rgb(172, 176, 190),
                overlay0: Color32::from_rgb(156, 160, 176),
                text: Color32::from_rgb(76, 79, 105),
                subtext0: Color32::from_rgb(108, 111, 133),
                accent: Color32::from_rgb(30, 102, 245),
                accent_border: Color32::from_rgb(30, 102, 245),
                success: Color32::from_rgb(64, 160, 43),
                warning: Color32::from_rgb(223, 142, 29),
                danger: Color32::from_rgb(210, 15, 57),
            },
            ThemeChoice::Frappe => Self {
                base: Color32::from_rgb(48, 52, 70),
                mantle: Color32::from_rgb(41, 44, 60),
                crust: Color32::from_rgb(35, 38, 52),
                surface0: Color32::from_rgb(65, 69, 89),
                surface1: Color32::from_rgb(81, 87, 109),
                surface2: Color32::from_rgb(98, 104, 128),
                overlay0: Color32::from_rgb(115, 121, 148),
                text: Color32::from_rgb(198, 208, 245),
                subtext0: Color32::from_rgb(165, 173, 206),
                accent: Color32::from_rgb(140, 170, 238),
                accent_border: Color32::from_rgb(140, 170, 238),
                success: Color32::from_rgb(166, 209, 137),
                warning: Color32::from_rgb(239, 159, 118),
                danger: Color32::from_rgb(231, 130, 132),
            },
            ThemeChoice::Macchiato => Self {
                base: Color32::from_rgb(36, 39, 58),
                mantle: Color32::from_rgb(30, 32, 48),
                crust: Color32::from_rgb(24, 25, 38),
                surface0: Color32::from_rgb(54, 58, 79),
                surface1: Color32::from_rgb(73, 77, 100),
                surface2: Color32::from_rgb(91, 96, 120),
                overlay0: Color32::from_rgb(110, 115, 141),
                text: Color32::from_rgb(202, 211, 245),
                subtext0: Color32::from_rgb(165, 173, 203),
                accent: Color32::from_rgb(138, 173, 244),
                accent_border: Color32::from_rgb(138, 173, 244),
                success: Color32::from_rgb(166, 218, 149),
                warning: Color32::from_rgb(245, 169, 127),
                danger: Color32::from_rgb(237, 135, 150),
            },
            ThemeChoice::Mocha => Self {
                base: Color32::from_rgb(30, 30, 46),
                mantle: Color32::from_rgb(24, 24, 37),
                crust: Color32::from_rgb(17, 17, 27),
                surface0: Color32::from_rgb(49, 50, 68),
                surface1: Color32::from_rgb(69, 71, 90),
                surface2: Color32::from_rgb(88, 91, 112),
                overlay0: Color32::from_rgb(108, 112, 134),
                text: Color32::from_rgb(205, 214, 244),
                subtext0: Color32::from_rgb(166, 173, 200),
                accent: Color32::from_rgb(137, 180, 250),
                accent_border: Color32::from_rgb(137, 180, 250),
                success: Color32::from_rgb(166, 227, 161),
                warning: Color32::from_rgb(250, 179, 135),
                danger: Color32::from_rgb(243, 139, 168),
            },
        }
    }

    fn system_default() -> Self {
        Self {
            base: Color32::from_gray(250),
            mantle: Color32::from_gray(245),
            crust: Color32::from_gray(230),
            surface0: Color32::from_gray(248),
            surface1: Color32::from_gray(240),
            surface2: Color32::from_gray(230),
            overlay0: Color32::from_gray(200),
            text: Color32::from_gray(20),
            subtext0: Color32::from_gray(120),
            accent: Color32::from_rgb(90, 130, 200),
            accent_border: Color32::from_rgb(90, 130, 200),
            success: Color32::from_rgb(60, 130, 90),
            warning: Color32::from_rgb(150, 110, 30),
            danger: Color32::from_rgb(180, 40, 40),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum DensityChoice {
    Compact,
    Comfortable,
    Spacious,
}

impl DensityChoice {
    fn label(self) -> &'static str {
        match self {
            DensityChoice::Compact => "Compact",
            DensityChoice::Comfortable => "Comfortable",
            DensityChoice::Spacious => "Spacious",
        }
    }

    fn all() -> [Self; 3] {
        [
            DensityChoice::Compact,
            DensityChoice::Comfortable,
            DensityChoice::Spacious,
        ]
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct UserPreferences {
    open_last_session: bool,
    auto_save_queue: bool,
    check_updates_on_launch: bool,
    confirm_before_rename: bool,
    language: LanguageChoice,
    region: RegionChoice,
    date_format: DateFormat,
    theme: ThemeChoice,
    density: DensityChoice,
    show_section_headers: bool,
    animate_transitions: bool,
    show_tips: bool,
    enable_sound_cues: bool,
    show_status_toasts: bool,
    highlight_matches: bool,
    enable_quick_actions: bool,
    confirm_before_clearing: bool,
    copy_results_to_clipboard: bool,
    keep_logs: bool,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            open_last_session: true,
            auto_save_queue: true,
            check_updates_on_launch: true,
            confirm_before_rename: true,
            language: LanguageChoice::System,
            region: RegionChoice::Auto,
            date_format: DateFormat::System,
            theme: ThemeChoice::System,
            density: DensityChoice::Comfortable,
            show_section_headers: true,
            animate_transitions: true,
            show_tips: true,
            enable_sound_cues: false,
            show_status_toasts: true,
            highlight_matches: true,
            enable_quick_actions: true,
            confirm_before_clearing: true,
            copy_results_to_clipboard: false,
            keep_logs: true,
        }
    }
}

impl UserPreferences {
    fn load() -> Option<Self> {
        let path = Self::preferences_path()?;
        let contents = fs::read_to_string(path).ok()?;
        toml::from_str(&contents).ok()
    }

    fn save(&self) {
        let Some(path) = Self::preferences_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!("Failed to create preferences directory: {err}");
                return;
            }
        }
        let Ok(payload) = toml::to_string_pretty(self) else {
            return;
        };
        if let Err(err) = fs::write(path, payload) {
            eprintln!("Failed to save preferences: {err}");
        }
    }

    fn preferences_path() -> Option<PathBuf> {
        config_root().map(|root| root.join("preferences.toml"))
    }
}

fn config_root() -> Option<PathBuf> {
    dirs::config_dir().map(|base| base.join("Colony").join("KayaBot"))
}

trait ListItem {
    fn render(&self, app: &mut RenameApp, ui: &mut egui::Ui);
}

#[derive(Debug, Clone, Default)]
struct ManualOverride {
    title: Option<String>,
    season: Option<u32>,
    episode: Option<u32>,
}

impl ManualOverride {
    fn is_empty(&self) -> bool {
        self.title
            .as_ref()
            .map(|title| title.trim().is_empty())
            .unwrap_or(true)
            && self.season.is_none()
            && self.episode.is_none()
    }
}

struct OriginalFileRow {
    index: usize,
    name: String,
}

impl ListItem for OriginalFileRow {
    fn render(&self, app: &mut RenameApp, ui: &mut egui::Ui) {
        let selected = app.selected_file_index == Some(self.index);
        if ui.selectable_label(selected, &self.name).clicked() {
            app.selected_file_index = Some(self.index);
            app.match_picker_file_index = Some(self.index);
            app.show_match_picker = true;
        }
    }
}

struct NewNameRow {
    name: String,
    status: Option<matching::MatchStatus>,
    original: Option<String>,
    content_type_label: Option<String>,
    confidence: Option<f32>,
    candidate_count: Option<usize>,
    rename_note: Option<(String, Color32)>,
    status_color: Option<Color32>,
    muted_color: Color32,
}

impl NewNameRow {}

impl ListItem for NewNameRow {
    fn render(&self, _app: &mut RenameApp, ui: &mut egui::Ui) {
        if let Some(status) = self.status {
            let (label, color) = match status {
                matching::MatchStatus::Ok => (
                    "ok",
                    self.status_color.unwrap_or(ui.visuals().hyperlink_color),
                ),
                matching::MatchStatus::Ambiguous => (
                    "ambiguous",
                    self.status_color.unwrap_or(ui.visuals().warn_fg_color),
                ),
                matching::MatchStatus::Error => (
                    "error",
                    self.status_color.unwrap_or(ui.visuals().error_fg_color),
                ),
            };
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.name);
                    ui.add_space(6.0);
                    ui.label(RichText::new(label).color(color));
                    if let Some(content_type) = &self.content_type_label {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(content_type)
                                .color(self.muted_color)
                                .size(10.0),
                        );
                    }
                });
                if let Some(original) = &self.original {
                    let candidate_count = self.candidate_count.unwrap_or(1);
                    let detail = if let Some(confidence) = self.confidence {
                        format!(
                            "from {original} • {:.0}% confidence • {candidate_count} candidate(s)",
                            confidence * 100.0
                        )
                    } else {
                        format!("from {original} • {candidate_count} candidate(s)")
                    };
                    ui.label(RichText::new(detail).color(self.muted_color).size(10.0));
                }
                if let Some((note, color)) = &self.rename_note {
                    ui.label(RichText::new(note).color(*color).size(10.0));
                }
            });
        } else {
            ui.label(RichText::new(&self.name).color(self.muted_color));
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(1100.0, 650.0))
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "FileBot - Rename",
        options,
        Box::new(|_cc| Box::new(RenameApp::default())),
    )
}
