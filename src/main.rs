use eframe::egui::{self, Button, Color32, FontId, Frame, Layout, RichText, ScrollArea, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftNav {
    Rename,
    Episodes,
    Subtitles,
    Sfv,
    Filter,
    List,
}

struct RenameApp {
    active_left_nav: LeftNav,
    original_files: Vec<String>,
    new_names: Vec<String>,
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
        }
    }
}

impl eframe::App for RenameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.left_sidebar(ui);
                ui.add_space(10.0);
                self.main_content(ui);
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

    fn main_content(&mut self, ui: &mut egui::Ui) {
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

                    self.list_panel(ui, "Original Files", left_width, &self.original_files, |ui| {
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

                    self.list_panel(ui, "New Names", right_width, &self.new_names, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_sized(Vec2::new(32.0, 26.0), Button::new("⬇"));
                            ui.add_sized(Vec2::new(32.0, 26.0), Button::new("⬆"));
                            ui.add_sized(Vec2::new(70.0, 26.0), Button::new("📂 Load"));
                            ui.add_sized(Vec2::new(92.0, 26.0), Button::new("Fetch Data"));
                            ui.add_sized(Vec2::new(32.0, 26.0), Button::new("🔧"));
                        });
                    });
                });
            },
        );
    }

    fn list_panel<F>(&self, ui: &mut egui::Ui, title: &str, width: f32, items: &[String], toolbar: F)
    where
        F: FnOnce(&mut egui::Ui),
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

                        ui.add_space(8.0);
                        ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.set_min_height(toolbar_height);
                            toolbar(ui);
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
