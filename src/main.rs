mod formatting;
mod matching;
mod metadata;

use eframe::egui::{
    self, Button, Color32, FontId, Frame, Layout, RichText, ScrollArea, Stroke, Vec2,
};
use formatting::{
    DEFAULT_MOVIE_FORMAT, DEFAULT_SERIES_FORMAT, FormatOptions, MovieFormatInput, SeriesFormatInput,
};
use metadata::aggregate::MetadataPipeline;
use metadata::models::{EpisodeMatch, TitleMatch};
use metadata::provider::{MetadataProvider, MetadataSource};
use metadata::providers::{
    anidb::AniDbClient, omdb::OmdbClient, thetvdb::TheTvDbClient, tmdb::TmdbClient,
    tvmaze::TvMazeClient,
};
use std::{env, fs, path::PathBuf};

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
enum ContentType {
    Movie,
    Series,
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

struct RenameApp {
    active_left_nav: LeftNav,
    original_files: Vec<String>,
    match_results: Vec<matching::MatchResult>,
    content_type: ContentType,
    detected_series_name: String,
    fetch_status: FetchStatus,
    title_matches: Vec<TitleMatch>,
    selected_title_id: Option<String>,
    episode_matches: Vec<EpisodeMatch>,
    show_match_picker: bool,
    rename_feedback: Option<(usize, usize)>,
    format_options: FormatOptions,
    metadata_provider: MetadataPipeline,
    rename_ui_state: RenameUiState,
    active_metadata_source: MetadataSource,
    preferred_movie_source: MetadataSource,
    preferred_series_source: MetadataSource,
    detection_notice: Option<String>,
    force_metadata_source: bool,
    api_config: ApiConfig,
}

impl Default for RenameApp {
    fn default() -> Self {
        let original_files = Vec::new();
        let match_results = Vec::new();
        let rename_ui_state = RenameUiState::Empty;
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

        Self {
            active_left_nav: LeftNav::Rename,
            original_files,
            match_results,
            content_type: ContentType::Series,
            detected_series_name: String::new(),
            fetch_status: FetchStatus::Idle,
            title_matches: Vec::new(),
            selected_title_id: None,
            episode_matches: Vec::new(),
            show_match_picker: false,
            rename_feedback: None,
            format_options: FormatOptions::default(),
            metadata_provider,
            rename_ui_state,
            active_metadata_source: MetadataSource::TheTvDb,
            preferred_movie_source: MetadataSource::TheMovieDb,
            preferred_series_source: MetadataSource::TheTvDb,
            detection_notice: None,
            force_metadata_source: false,
            api_config,
        }
    }
}

impl eframe::App for RenameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_dropped_files(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.left_sidebar(ui);
                ui.add_space(10.0);
                self.main_content(ctx, ui);
            });
        });
    }
}

impl RenameApp {
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped_files = ctx.input(|input| input.raw.dropped_files.clone());
        if dropped_files.is_empty() {
            return;
        }

        let mut added = false;
        for dropped in dropped_files {
            if let Some(path) = dropped.path {
                if !path.is_file() {
                    continue;
                }
                let path_string = path.display().to_string();
                if !self.original_files.contains(&path_string) {
                    self.original_files.push(path_string);
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
            self.match_results = matching::match_files(&self.original_files);
            self.apply_content_detection();
            self.rename_feedback = None;
            self.refresh_rename_ui_state();
        }
    }
    fn left_sidebar(&mut self, ui: &mut egui::Ui) {
        let sidebar_width = 110.0;
        let frame = Frame::none()
            .fill(Color32::from_gray(240))
            .stroke(Stroke::new(1.0, Color32::from_gray(200)))
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
        let is_active = self.active_left_nav == id;
        let fill = if is_active {
            Color32::from_rgb(205, 225, 248)
        } else {
            Color32::from_gray(248)
        };
        let stroke = if is_active {
            Stroke::new(1.5, Color32::from_rgb(90, 130, 200))
        } else {
            Stroke::new(1.0, Color32::from_gray(200))
        };

        let button_frame = Frame::none()
            .fill(fill)
            .stroke(stroke)
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::symmetric(6.0, 6.0));

        let response = button_frame
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(86.0, 78.0));
                ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new(icon).size(26.0));
                    ui.label(RichText::new(label).size(12.0));
                })
                .response
            })
            .response;

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
            |ui| {
                match self.active_left_nav {
                    LeftNav::Rename => self.rename_content(ctx, ui),
                    LeftNav::Settings => self.settings_content(ui),
                    LeftNav::Episodes | LeftNav::Subtitles | LeftNav::Sfv | LeftNav::Filter => {
                        self.placeholder_content(ui)
                    }
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
            let original_files = self.original_files.clone();
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
                |_, ui| {
                    ui.horizontal(|ui| {
                        ui.add_sized(Vec2::new(32.0, 26.0), Button::new("⬇"));
                        ui.add_sized(Vec2::new(32.0, 26.0), Button::new("⬆"));
                        ui.add_sized(Vec2::new(32.0, 26.0), Button::new("❌"));
                        ui.add_sized(Vec2::new(70.0, 26.0), Button::new("📂 Load"));
                        ui.add_sized(Vec2::new(32.0, 26.0), Button::new("🔄"));
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
                            app.show_match_picker = true;
                        }
                    });
                },
            );
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        self.rename_status_message(ui);

        if self.show_match_picker {
            self.match_picker_window(ctx);
        }
    }

    fn settings_content(&mut self, ui: &mut egui::Ui) {
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

        ui.label("Future configuration options will live here.");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("API configuration").strong());
            ui.label(
                RichText::new("Keys are loaded from ~/.kayabot/config.toml or environment variables.")
                    .size(11.0)
                    .color(Color32::from_gray(120)),
            );
            if let Some(path) = ApiConfig::config_path() {
                ui.label(
                    RichText::new(format!("Config path: {}", path.display()))
                        .size(11.0)
                        .color(Color32::from_gray(120)),
                );
            }
            ui.add_space(6.0);
            self.settings_status_row(ui, "TMDB", self.api_config.tmdb_configured());
            self.settings_status_row(ui, "TVDB", self.api_config.tvdb_configured());
            self.settings_status_row(ui, "OMDB", self.api_config.omdb_configured());
            self.settings_status_row(ui, "AniDB", self.api_config.anidb_configured());
            self.settings_status_row(ui, "TVMaze", self.api_config.tvmaze_configured());
        });
    }

    fn settings_status_row(&self, ui: &mut egui::Ui, label: &str, configured: bool) {
        let status = if configured { "Configured" } else { "Missing" };
        let color = if configured {
            Color32::from_rgb(60, 130, 90)
        } else {
            Color32::from_rgb(180, 40, 40)
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
        let panel_frame = Frame::none()
            .fill(Color32::from_gray(245))
            .stroke(Stroke::new(1.0, Color32::from_gray(180)))
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
                            .stroke(Stroke::new(1.0, Color32::from_gray(180)))
                            .fill(Color32::WHITE)
                            .rounding(egui::Rounding::same(2.0))
                            .show(ui, |ui| {
                                ui.set_min_height(list_height);
                                ScrollArea::vertical()
                                    .id_source((panel_id, "list_scroll"))
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for item in items {
                                            item.render(ui);
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
        self.content_type = content_type;
        self.refresh_active_sources();
        self.fetch_status = FetchStatus::Idle;
        self.title_matches.clear();
        self.episode_matches.clear();
        self.selected_title_id = None;
        self.rename_feedback = None;
        self.detection_notice = None;
        match content_type {
            ContentType::Movie => self.preferred_movie_source = source,
            ContentType::Series => self.preferred_series_source = source,
        }
        self.refresh_rename_ui_state();
    }

    fn refresh_active_sources(&mut self) {
        let sources = if self.force_metadata_source {
            vec![self.active_metadata_source]
        } else {
            let mut sources = vec![self.active_metadata_source];
            sources.extend(self.fallback_sources(self.content_type, self.active_metadata_source));
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
        let frame = Frame::none()
            .fill(Color32::from_gray(245))
            .stroke(Stroke::new(1.0, Color32::from_gray(200)))
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
        let response = ui.add_sized(
            Vec2::new(70.0, 70.0),
            Button::new(RichText::new(format!("{icon}\n{label}")).size(14.0))
                .wrap(true)
                .fill(Color32::from_gray(250))
                .stroke(Stroke::new(1.0, Color32::from_gray(160))),
        );
        if response.clicked() {
            println!("Action clicked: {label}");
            if label == "Match" {
                self.match_results = matching::match_files(&self.original_files);
                self.apply_content_detection();
                self.rename_feedback = None;
                self.refresh_rename_ui_state();
            } else if label == "Rename" {
                let matched = self.episode_matches.len().min(self.original_files.len());
                let unmatched = self.original_files.len().saturating_sub(matched);
                self.rename_feedback = Some((matched, unmatched));
            }
        }
    }

    fn fetch_metadata(&mut self) {
        self.fetch_status = FetchStatus::Loading;
        self.rename_feedback = None;
        self.refresh_rename_ui_state();

        let parsed_hint = self.detect_search_hint();
        if self.detected_series_name.trim().is_empty() {
            if let Some(title) = parsed_hint.as_ref().and_then(|hint| hint.title.clone()) {
                self.detected_series_name = title;
            }
        }

        if let Some(hint) = parsed_hint.as_ref() {
            let has_episode = hint.season.is_some() || hint.episode.is_some();
            let has_year = hint.year.is_some();
            if has_episode && self.content_type != ContentType::Series {
                self.switch_metadata_source(self.preferred_series_source, ContentType::Series);
            } else if has_year && !has_episode && self.content_type != ContentType::Movie {
                self.switch_metadata_source(self.preferred_movie_source, ContentType::Movie);
            }
        }

        let mut query = self.detected_series_name.trim().to_string();
        if query.is_empty() {
            if let Some(title) = parsed_hint.as_ref().and_then(|hint| hint.title.clone()) {
                query = title;
            }
        }
        if self.content_type == ContentType::Movie {
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
                if self.content_type == ContentType::Series {
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
        let file = self.original_files.first()?;
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
        ui.group(|ui| {
            ui.label(RichText::new("Fetch Inputs").size(12.0));
            ui.horizontal(|ui| {
                ui.label("Type");
                egui::ComboBox::from_id_source("content_type")
                    .selected_text(match self.content_type {
                        ContentType::Movie => "Movie",
                        ContentType::Series => "Series",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.content_type, ContentType::Movie, "Movie");
                        ui.selectable_value(&mut self.content_type, ContentType::Series, "Series");
                    });
                ui.label("Series");
                ui.add(
                    egui::TextEdit::singleline(&mut self.detected_series_name)
                        .hint_text("Detected name")
                        .desired_width(140.0),
                );
            });
            let force_response = ui.checkbox(&mut self.force_metadata_source, "Forcer la source");
            if force_response.changed() {
                self.refresh_active_sources();
                self.fetch_status = FetchStatus::Idle;
                self.title_matches.clear();
                self.episode_matches.clear();
                self.selected_title_id = None;
            }
            if let Some(message) = &self.detection_notice {
                ui.add_space(4.0);
                ui.label(RichText::new(message).color(Color32::from_rgb(150, 110, 30)));
            }

            match &self.fetch_status {
                FetchStatus::Loading => {
                    ui.label(
                        RichText::new("Loading metadata...").color(Color32::from_rgb(80, 80, 160)),
                    );
                }
                FetchStatus::Error(message) => {
                    ui.label(RichText::new(message).color(Color32::from_rgb(160, 40, 40)));
                }
                _ => {}
            }

            ui.add_space(6.0);
            ui.label(RichText::new("Formatting").size(12.0));
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.format_options.include_episode_title,
                    "Include episode title",
                );
                ui.checkbox(&mut self.format_options.include_year, "Include year");
            });
            let format_label = match self.content_type {
                ContentType::Series => DEFAULT_SERIES_FORMAT,
                ContentType::Movie => DEFAULT_MOVIE_FORMAT,
            };
            ui.label(
                RichText::new(format_label)
                    .size(10.0)
                    .color(Color32::from_gray(120)),
            );
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
                .color(Color32::from_gray(110)),
            );
            if self.content_type == ContentType::Series {
                if self.episode_matches.is_empty() {
                    ui.label(
                        RichText::new("No episodes loaded yet.").color(Color32::from_gray(100)),
                    );
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
                ui.label(RichText::new("No title matches yet.").color(Color32::from_gray(100)));
            } else {
                ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                    for title in &self.title_matches {
                        ui.label(self.format_title_match_summary(title));
                    }
                });
            }

            if let Some((matched, unmatched)) = self.rename_feedback {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("{matched} renamed / {unmatched} unmatched")).strong(),
                );
            }
        });
    }

    fn match_picker_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_match_picker;
        let mut close_window = false;
        egui::Window::new("Adjust Title Match")
            .open(&mut open)
            .show(ctx, |ui| {
                if self.title_matches.is_empty() {
                    ui.label("Fetch data first to see matches.");
                    return;
                }

                let mut selected_id = None;
                let mut should_fetch = false;
                let mut should_close = false;
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        for title in &self.title_matches {
                            let label = self.format_title_match_summary(title);
                            let selected = self.selected_title_id.as_deref() == Some(&title.id);
                            if ui.selectable_label(selected, label).clicked() {
                                selected_id = Some(title.id.clone());
                                should_fetch = self.content_type == ContentType::Series;
                                should_close = true;
                            }
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Source détaillée").strong());
                        ui.add_space(4.0);
                        if let Some(title) = self.selected_title_match() {
                            self.render_title_match_details(ui, title);
                        } else {
                            ui.label(
                                RichText::new("Sélectionnez un match.")
                                    .color(Color32::from_gray(110)),
                            );
                        }
                    });
                });
                if let Some(selected_id) = selected_id {
                    self.selected_title_id = Some(selected_id);
                    if should_fetch {
                        self.fetch_episodes();
                    }
                }
                if should_close {
                    close_window = true;
                }
            });
        if close_window {
            open = false;
        }
        self.show_match_picker = open;
    }

    fn new_names_rows(&self) -> Vec<NewNameRow> {
        match &self.rename_ui_state {
            RenameUiState::Loading => vec![NewNameRow::status("Chargement des résultats…")],
            RenameUiState::Error(message) => {
                vec![NewNameRow::status(format!("Erreur: {message}"))]
            }
            RenameUiState::Empty => vec![NewNameRow::status("Aucun résultat disponible.")],
            RenameUiState::Success(_) => self
                .match_results
                .iter()
                .map(|result| NewNameRow {
                    name: self.format_preview(result),
                    status: Some(result.status),
                    original: Some(result.original.clone()),
                    confidence: Some(result.confidence),
                    candidate_count: Some(result.candidates.len()),
                })
                .collect(),
        }
    }

    fn rename_status_message(&self, ui: &mut egui::Ui) {
        let (message, color) = match &self.rename_ui_state {
            RenameUiState::Loading => (
                "Statut: chargement…".to_string(),
                Color32::from_rgb(80, 80, 160),
            ),
            RenameUiState::Success(count) => (
                format!("Statut: succès — {count} résultat(s) prêts."),
                Color32::from_rgb(60, 130, 90),
            ),
            RenameUiState::Error(message) => (
                format!("Statut: erreur — {message}"),
                Color32::from_rgb(180, 40, 40),
            ),
            RenameUiState::Empty => (
                "Statut: aucun résultat pour le moment.".to_string(),
                Color32::from_gray(110),
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
        match matching::guess_content_type(&self.match_results) {
            ContentGuess::Series => {
                let target_source = self.preferred_series_source;
                if self.content_type != ContentType::Series
                    || self.active_metadata_source != target_source
                {
                    self.switch_metadata_source(target_source, ContentType::Series);
                }
            }
            ContentGuess::Movie => {
                let target_source = self.preferred_movie_source;
                if self.content_type != ContentType::Movie
                    || self.active_metadata_source != target_source
                {
                    self.switch_metadata_source(target_source, ContentType::Movie);
                }
            }
            ContentGuess::Ambiguous => {
                self.detection_notice = Some(
                    "Type ambigu détecté. Choisissez Film ou Série, puis la source.".to_string(),
                );
            }
            ContentGuess::Unknown => {
                self.detection_notice =
                    Some("Aucun indice clair sur le type. Choisissez Film ou Série.".to_string());
            }
        }
    }

    fn format_preview(&self, result: &matching::MatchResult) -> String {
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
                let series_name = if self.detected_series_name.trim().is_empty() {
                    series_title
                        .clone()
                        .unwrap_or_else(|| "Unknown Series".to_string())
                } else {
                    self.detected_series_name.clone()
                };
                let resolved_title = self
                    .episode_matches
                    .iter()
                    .find(|episode_match| {
                        episode_match.season == *season && episode_match.episode == *episode
                    })
                    .map(|episode_match| episode_match.title.as_str())
                    .or_else(|| episode_title.as_deref())
                    .or_else(|| series_title.as_deref());

                formatting::format_series_name(
                    SeriesFormatInput {
                        series: &series_name,
                        season: *season,
                        episode: *episode,
                        title: resolved_title,
                        year: None,
                    },
                    self.format_options,
                )
            }
            matching::MatchMetadata::Movie { title, year } => {
                let movie_title = title.as_deref().unwrap_or("Unknown Title");
                formatting::format_movie_name(
                    MovieFormatInput {
                        title: movie_title,
                        year: *year,
                    },
                    self.format_options,
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
        self.title_matches.iter().find(|title| title.id == selected_id)
    }

    fn render_title_match_details(&self, ui: &mut egui::Ui, title: &TitleMatch) {
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
            ui.label(RichText::new(synopsis).size(10.0).color(Color32::from_gray(120)));
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
            .or_else(|| config.as_ref().and_then(|cfg| cfg.tmdb_bearer_token.clone()))
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
            .or_else(|| config.as_ref().and_then(|cfg| cfg.tvmaze_user_agent.clone()))
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
        env::var(key)
            .ok()
            .and_then(|value| if value.trim().is_empty() { None } else { Some(value) })
    }

    fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".kayabot").join("config.toml"))
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

trait ListItem {
    fn render(&self, ui: &mut egui::Ui);
}

impl ListItem for String {
    fn render(&self, ui: &mut egui::Ui) {
        ui.label(self);
    }
}

struct NewNameRow {
    name: String,
    status: Option<matching::MatchStatus>,
    original: Option<String>,
    confidence: Option<f32>,
    candidate_count: Option<usize>,
}

impl NewNameRow {
    fn status(message: impl Into<String>) -> Self {
        Self {
            name: message.into(),
            status: None,
            original: None,
            confidence: None,
            candidate_count: None,
        }
    }
}

impl ListItem for NewNameRow {
    fn render(&self, ui: &mut egui::Ui) {
        if let Some(status) = self.status {
            let (label, color) = match status {
                matching::MatchStatus::Ok => ("ok", Color32::from_rgb(40, 140, 80)),
                matching::MatchStatus::Ambiguous => ("ambiguous", Color32::from_rgb(180, 130, 30)),
                matching::MatchStatus::Error => ("error", Color32::from_rgb(180, 40, 40)),
            };
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.name);
                    ui.add_space(6.0);
                    ui.label(RichText::new(label).color(color));
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
                    ui.label(
                        RichText::new(detail)
                            .color(Color32::from_gray(120))
                            .size(10.0),
                    );
                }
            });
        } else {
            ui.label(RichText::new(&self.name).color(Color32::from_gray(120)));
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
