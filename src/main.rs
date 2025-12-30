mod metadata;

use eframe::egui::{self, Button, Color32, FontId, Frame, Layout, RichText, ScrollArea, Stroke, Vec2};
use metadata::filebot_like::FileBotLikeProvider;
use metadata::models::{EpisodeMatch, TitleMatch};
use metadata::provider::MetadataProvider;

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

struct RenameApp {
    active_left_nav: LeftNav,
    original_files: Vec<String>,
    new_names: Vec<String>,
    content_type: ContentType,
    detected_series_name: String,
    fetch_status: FetchStatus,
    title_matches: Vec<TitleMatch>,
    selected_title_id: Option<String>,
    episode_matches: Vec<EpisodeMatch>,
    show_match_picker: bool,
    rename_feedback: Option<(usize, usize)>,
    metadata_provider: FileBotLikeProvider,
}

impl Default for RenameApp {
    fn default() -> Self {
        let original_files = vec![
            "alias.116",
            "alias.117",
            "alias.118",
            "alias.119",
            "alias.120",
            "alias.121",
            "alias.122",
            "alias.201",
            "alias.202",
            "alias.203",
            "alias.204",
            "alias.205",
            "alias.206",
            "alias.207",
            "alias.208",
            "alias.209",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let new_names = vec![
            "~/Media/TV Shows/Alias/Season 01/Alias - S01E16 - The Prophecy",
            "~/Media/TV Shows/Alias/Season 01/Alias - S01E17 - Q & A",
            "~/Media/TV Shows/Alias/Season 01/Alias - S01E18 - Masquerade",
            "~/Media/TV Shows/Alias/Season 01/Alias - S01E19 - Snowman",
            "~/Media/TV Shows/Alias/Season 01/Alias - S01E20 - The Solution",
            "~/Media/TV Shows/Alias/Season 01/Alias - S01E21 - Rendezvous",
            "~/Media/TV Shows/Alias/Season 01/Alias - S01E22 - Almost Thirty Years",
            "~/Media/TV Shows/Alias/Season 02/Alias - S02E01 - The Enemy Walks In",
            "~/Media/TV Shows/Alias/Season 02/Alias - S02E02 - Trust Me",
            "~/Media/TV Shows/Alias/Season 02/Alias - S02E03 - Cipher",
            "~/Media/TV Shows/Alias/Season 02/Alias - S02E04 - Dead Drop",
            "~/Media/TV Shows/Alias/Season 02/Alias - S02E05 - The Indicator",
            "~/Media/TV Shows/Alias/Season 02/Alias - S02E06 - Salvation",
            "~/Media/TV Shows/Alias/Season 02/Alias - S02E07 - The Counteragent",
            "~/Media/TV Shows/Alias/Season 02/Alias - S02E08 - Passage",
            "~/Media/TV Shows/Alias/Season 02/Alias - S02E09 - Passage",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        Self {
            active_left_nav: LeftNav::Rename,
            original_files,
            new_names,
            content_type: ContentType::Series,
            detected_series_name: "Alias".to_string(),
            fetch_status: FetchStatus::Idle,
            title_matches: Vec::new(),
            selected_title_id: None,
            episode_matches: Vec::new(),
            show_match_picker: false,
            rename_feedback: None,
            metadata_provider: FileBotLikeProvider::new(),
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
                    let new_names = self.new_names.clone();

                    Self::list_panel(self, ui, "Original Files", left_width, &original_files, |_, ui| {
                        ui.add_space(6.0);
                    }, |_, ui| {
                        ui.horizontal(|ui| {
                            ui.add_sized(Vec2::new(32.0, 26.0), Button::new("⬇"));
                            ui.add_sized(Vec2::new(32.0, 26.0), Button::new("⬆"));
                            ui.add_sized(Vec2::new(32.0, 26.0), Button::new("❌"));
                            ui.add_sized(Vec2::new(70.0, 26.0), Button::new("📂 Load"));
                            ui.add_sized(Vec2::new(32.0, 26.0), Button::new("🔄"));
                        });
                    });

                    ui.add_space(10.0);
                    self.center_buttons(ui);
                    ui.add_space(10.0);

                    Self::list_panel(self, ui, "New Names", right_width, &new_names, |app, ui| {
                        ui.add_space(6.0);
                        app.fetch_data_panel(ui);
                    }, |app, ui| {
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
                            let adjust_clicked = ui.add_sized(Vec2::new(32.0, 26.0), Button::new("🔧")).clicked();
                            if adjust_clicked {
                                app.show_match_picker = true;
                            }
                        });
                    });
                });

                if self.show_match_picker {
                    self.match_picker_window(ctx);
                }
            },
        );
    }

    fn list_panel<C, F>(
        app: &mut RenameApp,
        ui: &mut egui::Ui,
        title: &str,
        width: f32,
        items: &[String],
        content: C,
        toolbar: F,
    )
    where
        C: FnOnce(&mut RenameApp, &mut egui::Ui),
        F: FnOnce(&mut RenameApp, &mut egui::Ui),
    {
        let panel_frame = Frame::none()
            .fill(Color32::from_gray(245))
            .stroke(Stroke::new(1.0, Color32::from_gray(180)))
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(8.0, 8.0));

        ui.allocate_ui_with_layout(
            Vec2::new(width, ui.available_height()),
            Layout::top_down(egui::Align::Min),
            |ui| {
                ui.push_id(title, |ui| {
                    panel_frame.show(ui, |ui| {
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
                                            ui.label(item);
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
                let matched = self
                    .episode_matches
                    .len()
                    .min(self.original_files.len());
                let unmatched = self.original_files.len().saturating_sub(matched);
                self.rename_feedback = Some((matched, unmatched));
            }
        }
    }

    fn fetch_metadata(&mut self) {
        self.fetch_status = FetchStatus::Loading;
        self.rename_feedback = None;

        if self.detected_series_name.trim().is_empty() {
            if let Some(detected) = self.detect_series_name() {
                self.detected_series_name = detected;
            }
        }

        let query = self.detected_series_name.trim();
        match self.metadata_provider.search_title(query) {
            Ok(matches) => {
                self.title_matches = matches.clone();
                if let Some(best) = matches
                    .iter()
                    .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
                {
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
                    ui.label(RichText::new("Loading metadata...").color(Color32::from_rgb(80, 80, 160)));
                }
                FetchStatus::Error(message) => {
                    ui.label(RichText::new(message).color(Color32::from_rgb(160, 40, 40)));
                }
                _ => {}
            }

            ui.add_space(6.0);
            ui.label(RichText::new("Proposed Results").size(12.0));
            if self.content_type == ContentType::Series {
                if self.episode_matches.is_empty() {
                    ui.label(RichText::new("No episodes loaded yet.").color(Color32::from_gray(100)));
                } else {
                    ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
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
                ui.label(RichText::new(format!("{matched} renamed / {unmatched} unmatched")).strong());
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
                        title.year.map(|year| format!(" {year}")).unwrap_or_default(),
                        title.score
                    );
                    let selected = self.selected_title_id.as_deref() == Some(&title.id);
                    if ui.selectable_label(selected, label).clicked() {
                        selected_id = Some(title.id.clone());
                        should_fetch = self.content_type == ContentType::Series;
                        should_close = true;
                    }
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
