mod formatting;
mod matching;
mod metadata;

use eframe::egui::{
    self, Button, Color32, FontId, Frame, Layout, RichText, ScrollArea, Stroke, Vec2,
};
use formatting::{DEFAULT_SERIES_FORMAT, FormatOptions, MovieFormatInput, SeriesFormatInput};
use metadata::filebot_like::FileBotLikeProvider;
use metadata::models::{EpisodeMatch, TitleMatch};
use metadata::provider::{MetadataProvider, MetadataSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftNav {
    Rename,
    Episodes,
    Subtitles,
    Sfv,
    Filter,
    List,
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
    metadata_provider: FileBotLikeProvider,
    rename_ui_state: RenameUiState,
    active_metadata_source: MetadataSource,
}

impl Default for RenameApp {
    fn default() -> Self {
        let original_files = Vec::new();
        let match_results = Vec::new();
        let rename_ui_state = RenameUiState::Empty;

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
            metadata_provider: FileBotLikeProvider::new(),
            rename_ui_state,
            active_metadata_source: MetadataSource::TheMovieDb,
        }
    }
}

impl eframe::App for RenameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                    self.left_nav_button(ui, LeftNav::List, "📋", "List");
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
        self.refresh_rename_ui_state();
        let available_width = ui.available_width();
        ui.allocate_ui_with_layout(
            Vec2::new(available_width, ui.available_height()),
            Layout::top_down(egui::Align::Min),
            |ui| {
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
            },
        );
    }

    fn list_panel<C, F, T>(
        app: &mut RenameApp,
        ui: &mut egui::Ui,
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
                ui.push_id(title, |ui| {
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
                                    .id_source("list_scroll")
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
            app.fetch_match_context_menu(ui);
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
        self.metadata_provider.set_source(source);
        self.fetch_status = FetchStatus::Idle;
        self.title_matches.clear();
        self.episode_matches.clear();
        self.selected_title_id = None;
        self.rename_feedback = None;
        self.refresh_rename_ui_state();
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
            if label == "Rename" {
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

        if self.detected_series_name.trim().is_empty() {
            if let Some(detected) = self.detect_series_name() {
                self.detected_series_name = detected;
            }
        }

        let query = self.detected_series_name.trim();
        match self.metadata_provider.search_title(query) {
            Ok(matches) => {
                self.title_matches = matches.clone();
                if let Some(best) = matches.iter().max_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
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
            }
            Err(err) => {
                self.episode_matches.clear();
                self.fetch_status = FetchStatus::Error(err.to_string());
            }
        }
        self.refresh_rename_ui_state();
    }

    fn detect_series_name(&self) -> Option<String> {
        let file = self.original_files.first()?;
        let mut parts = file
            .split(|c: char| !c.is_alphanumeric())
            .filter(|part| !part.is_empty());
        parts.next().map(|value| value.to_string())
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
            ui.label(
                RichText::new(DEFAULT_SERIES_FORMAT)
                    .size(10.0)
                    .color(Color32::from_gray(120)),
            );
            ui.add_space(6.0);
            ui.label(RichText::new("Proposed Results").size(12.0));
            if self.content_type == ContentType::Series {
                if self.episode_matches.is_empty() {
                    ui.label(
                        RichText::new("No episodes loaded yet.").color(Color32::from_gray(100)),
                    );
                } else {
                    ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                        for episode in &self.episode_matches {
                            ui.label(format!(
                                "S{:02}E{:02} - {}",
                                episode.season, episode.episode, episode.title
                            ));
                        }
                    });
                }
            } else if self.title_matches.is_empty() {
                ui.label(RichText::new("No title matches yet.").color(Color32::from_gray(100)));
            } else {
                ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                    for title in &self.title_matches {
                        ui.label(format!("{} ({})", title.name, title.source));
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
                for title in &self.title_matches {
                    let label = format!(
                        "{}{} (score {:.2})",
                        title.name,
                        title
                            .year
                            .map(|year| format!(" {year}"))
                            .unwrap_or_default(),
                        title.score
                    );
                    let selected = self.selected_title_id.as_deref() == Some(&title.id);
                    ui.push_id(&title.id, |ui| {
                        if ui.selectable_label(selected, label).clicked() {
                            selected_id = Some(title.id.clone());
                            should_fetch = self.content_type == ContentType::Series;
                            should_close = true;
                        }
                    });
                }
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

    fn format_preview(&self, result: &matching::MatchResult) -> String {
        let Some(metadata) = &result.metadata else {
            return "—".to_string();
        };

        match metadata {
            matching::MatchMetadata::Series {
                title,
                season,
                episode,
            } => {
                let series_name = if self.detected_series_name.trim().is_empty() {
                    title
                        .clone()
                        .unwrap_or_else(|| "Unknown Series".to_string())
                } else {
                    self.detected_series_name.clone()
                };
                let episode_title = self
                    .episode_matches
                    .iter()
                    .find(|episode_match| {
                        episode_match.season == *season && episode_match.episode == *episode
                    })
                    .map(|episode_match| episode_match.title.as_str())
                    .or_else(|| title.as_deref());

                formatting::format_series_name(
                    SeriesFormatInput {
                        series: &series_name,
                        season: *season,
                        episode: *episode,
                        title: episode_title,
                    },
                    self.format_options,
                )
            }
            matching::MatchMetadata::Movie { title, year } => {
                let movie_title = title.as_deref().unwrap_or("Unknown Title");
                formatting::format_movie_name(
                    MovieFormatInput {
                        title: movie_title,
                        year: Some(*year),
                    },
                    self.format_options,
                )
            }
        }
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
}

impl NewNameRow {
    fn status(message: impl Into<String>) -> Self {
        Self {
            name: message.into(),
            status: None,
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
            ui.horizontal(|ui| {
                ui.label(&self.name);
                ui.add_space(6.0);
                ui.label(RichText::new(label).color(color));
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
