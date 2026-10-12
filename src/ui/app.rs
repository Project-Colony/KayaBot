//! The application: its state, every message it handles, and the frame the
//! views sit in.

use std::collections::HashSet;

use colony_ui::Typography;
use colony_ui::preferences::Change;
use colony_ui::theme::Palette;
use iced::event::{self, Event};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{container, row, stack};
use iced::{Element, Length, Subscription, Task, Theme, window};

use super::rename::{self, Rename};
use super::{preferences, sidebar};
use crate::api_keys::{ApiConfig, ApiKeysFile, ApiKeysLoadStatus};
use crate::config::{Config, Language, MetadataLanguage, MetadataRegion};
use crate::formatting::FormatOptions;
use crate::i18n;
use crate::lookup;
use crate::matching::ContentType;

/// The API key fields of Preferences as typed. They are saved trimmed, and an
/// empty field removes its key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyFields {
    pub tmdb: String,
    pub tvdb: String,
    pub omdb: String,
    pub tvmaze_user_agent: String,
}

impl KeyFields {
    fn from_file(keys: Option<ApiKeysFile>) -> Self {
        let keys = keys.unwrap_or_default();
        Self {
            tmdb: keys
                .tmdb_bearer_token
                .or(keys.tmdb_api_key)
                .unwrap_or_default(),
            tvdb: keys.tvdb_api_key.unwrap_or_default(),
            omdb: keys.omdb_api_key.unwrap_or_default(),
            tvmaze_user_agent: keys.tvmaze_user_agent.unwrap_or_default(),
        }
    }

    fn to_file(&self) -> ApiKeysFile {
        let value = |text: &str| {
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_string())
        };
        ApiKeysFile {
            // TMDB takes a v3 key or a v4 token in the same field.
            tmdb_bearer_token: value(&self.tmdb),
            tmdb_api_key: None,
            tvdb_api_key: value(&self.tvdb),
            omdb_api_key: value(&self.omdb),
            tvmaze_user_agent: value(&self.tvmaze_user_agent),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyField {
    Tmdb,
    Tvdb,
    Omdb,
    TvmazeUserAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    AddFiles,
    RemoveSelected,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Sent by the KayaBot button and by the Preferences page's Close button.
    TogglePreferences,
    ShowRename,
    Rename(rename::Message),
    Category(usize),
    ToggleSection(&'static str),
    Preference(Change),
    DismissToast,
    Language(Language),
    MetadataLanguage(MetadataLanguage),
    MetadataRegion(MetadataRegion),
    ConfirmBeforeRename,
    ConfirmBeforeClearing,
    DryRunByDefault,
    Template(ContentType, String),
    Key(KeyField, String),
    OpenConfigFolder,
    Shortcut(Shortcut),
}

pub struct App {
    pub(super) config: Config,
    pub(super) rename: Rename,
    pub(super) preferences_open: bool,
    pub(super) category: usize,
    pub(super) expanded: HashSet<String>,
    pub(super) toast: bool,
    pub(super) api_config: ApiConfig,
    pub(super) keys_status: ApiKeysLoadStatus,
    pub(super) keys: KeyFields,
    /// Why the last change could not be saved or opened, shown in place.
    pub(super) problem: Option<String>,
}

impl App {
    /// Builds the application from a config already applied by the caller.
    pub fn new(config: Config) -> Self {
        let (keys, keys_status) = ApiKeysFile::load();
        let api_config = ApiConfig::resolve(keys.as_ref());
        let pipeline = lookup::pipeline(&api_config, config.metadata_locale());
        let mut rename = Rename::new(
            pipeline,
            FormatOptions::load().unwrap_or_default(),
            config.dry_run_by_default,
        );
        rename.confirm_before_rename = config.confirm_before_rename;
        rename.confirm_before_clearing = config.confirm_before_clearing;
        Self {
            config,
            rename,
            preferences_open: false,
            category: 0,
            expanded: HashSet::new(),
            toast: false,
            api_config,
            keys_status,
            keys: KeyFields::from_file(keys),
            problem: None,
        }
    }

    pub fn title(&self) -> String {
        crate::paths::PROGRAM.to_string()
    }

    /// Built-in widgets (text fields, check boxes, scroll bars) draw from the
    /// iced theme, so it carries the active Colony palette.
    pub fn theme(&self) -> Theme {
        Theme::custom(
            crate::paths::PROGRAM,
            iced::theme::Palette {
                background: Palette::BG_PRIMARY(),
                text: Palette::TEXT_PRIMARY(),
                primary: Palette::ACCENT(),
                success: Palette::SUCCESS(),
                warning: Palette::WARNING(),
                danger: Palette::ERROR(),
            },
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        event::listen_with(on_event)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TogglePreferences => {
                self.preferences_open = !self.preferences_open;
                self.category = 0;
                self.problem = None;
            }
            Message::ShowRename => self.preferences_open = false,
            Message::Rename(message) => {
                return self.rename.update(message).map(Message::Rename);
            }
            Message::Category(index) => self.category = index,
            Message::ToggleSection(key) => {
                if !self.expanded.remove(key) {
                    self.expanded.insert(key.to_string());
                }
            }
            Message::Preference(change) => {
                self.config.standard.update(change);
                if matches!(change, Change::Theme { .. }) {
                    self.toast = true;
                }
                self.config.save();
            }
            Message::DismissToast => self.toast = false,
            Message::Language(language) => {
                self.config.language = language;
                i18n::set_locale(language.locale());
                self.config.save();
            }
            Message::MetadataLanguage(language) => {
                self.config.metadata_language = language;
                self.rebuild_pipeline();
                self.config.save();
            }
            Message::MetadataRegion(region) => {
                self.config.metadata_region = region;
                self.rebuild_pipeline();
                self.config.save();
            }
            Message::ConfirmBeforeRename => {
                self.config.confirm_before_rename = !self.config.confirm_before_rename;
                self.rename.confirm_before_rename = self.config.confirm_before_rename;
                self.config.save();
            }
            Message::ConfirmBeforeClearing => {
                self.config.confirm_before_clearing = !self.config.confirm_before_clearing;
                self.rename.confirm_before_clearing = self.config.confirm_before_clearing;
                self.config.save();
            }
            Message::DryRunByDefault => {
                self.config.dry_run_by_default = !self.config.dry_run_by_default;
                self.config.save();
            }
            Message::Template(content_type, template) => {
                self.rename.set_template(content_type, template);
            }
            Message::Key(field, value) => {
                let slot = match field {
                    KeyField::Tmdb => &mut self.keys.tmdb,
                    KeyField::Tvdb => &mut self.keys.tvdb,
                    KeyField::Omdb => &mut self.keys.omdb,
                    KeyField::TvmazeUserAgent => &mut self.keys.tvmaze_user_agent,
                };
                *slot = value;
                self.save_keys();
            }
            Message::OpenConfigFolder => {
                self.problem = match crate::paths::config_dir() {
                    Some(dir) => open::that(&dir)
                        .err()
                        .map(|err| i18n::tf("preferences_open_folder_failed", &[("error", &err)])),
                    None => Some(i18n::t("preferences_no_config_dir").to_string()),
                };
            }
            Message::Shortcut(shortcut) => {
                if self.preferences_open || self.rename.dialog_open() {
                    return Task::none();
                }
                let message = match shortcut {
                    Shortcut::AddFiles => rename::Message::AddFiles,
                    Shortcut::RemoveSelected => rename::Message::Remove,
                };
                return self.rename.update(message).map(Message::Rename);
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let typo = Typography::current();
        let content = if self.preferences_open {
            preferences::view(self, &typo)
        } else {
            self.rename.view(&typo).map(Message::Rename)
        };
        let content: Element<'_, Message> = if self.toast {
            stack![
                content,
                container(colony_ui::widgets::toast(
                    &typo,
                    i18n::t("theme_applied"),
                    Message::DismissToast,
                ))
                .padding(16)
                .align_bottom(Length::Fill),
            ]
            .into()
        } else {
            content
        };

        container(row![sidebar::view(self, &typo), content])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(Palette::BG_PRIMARY().into()),
                ..Default::default()
            })
            .into()
    }

    /// Saves the API key fields to api_keys.toml and puts them into effect.
    fn save_keys(&mut self) {
        let keys = self.keys.to_file();
        match keys.save() {
            Ok(path) => {
                self.problem = None;
                self.keys_status = ApiKeysLoadStatus {
                    path: Some(path),
                    state: crate::api_keys::ApiKeysLoadState::Loaded,
                };
            }
            Err(err) => {
                self.problem = Some(i18n::tf("preferences_keys_save_failed", &[("error", &err)]));
            }
        }
        self.api_config = ApiConfig::resolve(Some(&keys));
        self.rebuild_pipeline();
    }

    fn rebuild_pipeline(&mut self) {
        self.rename.set_pipeline(lookup::pipeline(
            &self.api_config,
            self.config.metadata_locale(),
        ));
    }
}

/// Files dropped on the window, and the keyboard shortcuts. A key a focused
/// text field used is not a shortcut.
fn on_event(event: Event, status: event::Status, _window: window::Id) -> Option<Message> {
    match event {
        Event::Window(window::Event::FileDropped(path)) => {
            Some(Message::Rename(rename::Message::Dropped(path)))
        }
        Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
            if status == event::Status::Ignored =>
        {
            match key.as_ref() {
                Key::Character(c) if modifiers.command() && c.eq_ignore_ascii_case("o") => {
                    Some(Message::Shortcut(Shortcut::AddFiles))
                }
                Key::Named(Named::Delete) => Some(Message::Shortcut(Shortcut::RemoveSelected)),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(Config::default())
    }

    #[test]
    fn the_name_opens_preferences_and_the_same_message_closes_them() {
        let mut app = app();
        let _ = app.update(Message::Category(3));
        let _ = app.update(Message::TogglePreferences);
        assert!(app.preferences_open);
        assert_eq!(app.category, 0, "Preferences open on General");
        let _ = app.update(Message::Category(4));
        let _ = app.update(Message::TogglePreferences);
        assert!(!app.preferences_open);
        assert_eq!(app.category, 0);
    }

    #[test]
    fn the_rename_entry_leaves_preferences() {
        let mut app = app();
        let _ = app.update(Message::TogglePreferences);
        let _ = app.update(Message::ShowRename);
        assert!(!app.preferences_open);
    }

    #[test]
    fn sections_toggle_open_and_closed() {
        let mut app = app();
        let _ = app.update(Message::ToggleSection("keys"));
        assert!(app.expanded.contains("keys"));
        let _ = app.update(Message::ToggleSection("keys"));
        assert!(!app.expanded.contains("keys"));
    }

    #[test]
    fn a_theme_change_raises_the_toast() {
        let mut app = app();
        let _ = app.update(Message::Preference(Change::HighContrast(false)));
        assert!(!app.toast);
        let _ = app.update(Message::Preference(Change::Theme {
            family: "gruvbox",
            variant: "dark",
        }));
        assert!(app.toast);
        let _ = app.update(Message::DismissToast);
        assert!(!app.toast);
    }

    #[test]
    fn confirmations_reach_the_rename_view() {
        let mut app = app();
        assert!(app.rename.confirm_before_rename && app.rename.confirm_before_clearing);
        let _ = app.update(Message::ConfirmBeforeRename);
        let _ = app.update(Message::ConfirmBeforeClearing);
        assert!(!app.config.confirm_before_rename && !app.rename.confirm_before_rename);
        assert!(!app.config.confirm_before_clearing && !app.rename.confirm_before_clearing);
    }

    #[test]
    fn shortcuts_wait_while_preferences_are_open() {
        let mut app = app();
        let _ = app.update(Message::Rename(rename::Message::Picked(vec![
            "/media/a.mkv".into(),
            "/media/b.mkv".into(),
        ])));
        let _ = app.update(Message::Rename(rename::Message::Select(0)));
        let _ = app.update(Message::TogglePreferences);
        let _ = app.update(Message::Shortcut(Shortcut::RemoveSelected));
        assert_eq!(app.rename.file_count(), 2);
        let _ = app.update(Message::TogglePreferences);
        let _ = app.update(Message::Shortcut(Shortcut::RemoveSelected));
        assert_eq!(app.rename.file_count(), 1);
    }

    #[test]
    fn key_fields_are_saved_trimmed_and_empty_ones_dropped() {
        let fields = KeyFields {
            tmdb: "  token ".to_string(),
            tvdb: "   ".to_string(),
            omdb: String::new(),
            tvmaze_user_agent: "KayaBot (me@example.org)".to_string(),
        };
        let file = fields.to_file();
        assert_eq!(file.tmdb_bearer_token.as_deref(), Some("token"));
        assert_eq!(file.tvdb_api_key, None);
        assert_eq!(file.omdb_api_key, None);
        assert_eq!(
            file.tvmaze_user_agent.as_deref(),
            Some("KayaBot (me@example.org)")
        );
        assert_eq!(KeyFields::from_file(Some(file)).tmdb, "token");
    }
}
