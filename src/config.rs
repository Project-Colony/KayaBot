//! The user's preferences, saved as `preferences.json` in the config directory.
//!
//! The Appearance and Accessibility part is colony-ui's
//! [`StandardPreferences`]; the rest is KayaBot's own. Loading never fails: a
//! value this build does not understand falls back to its default, for that
//! field only. The `preferences.toml` of older versions is read once and
//! rewritten in the new format.

use std::path::{Path, PathBuf};
use std::{fs, io};

use colony_ui::i18n::Locale;
use colony_ui::preferences::StandardPreferences;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::metadata::locale::MetadataLocale;
use crate::paths;

const FILE_NAME: &str = "preferences.json";
const LEGACY_FILE_NAME: &str = "preferences.toml";

/// The language of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// Whatever the environment asks for.
    #[default]
    System,
    English,
    French,
}

impl Language {
    pub const ALL: [Self; 3] = [Self::System, Self::English, Self::French];

    pub fn locale(self) -> Locale {
        match self {
            Self::System => crate::i18n::system_locale(),
            Self::English => Locale::En,
            Self::French => Locale::Fr,
        }
    }
}

/// The language titles and episode names are fetched in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataLanguage {
    /// Each source's own default.
    #[default]
    System,
    English,
    French,
    Spanish,
}

impl MetadataLanguage {
    pub const ALL: [Self; 4] = [Self::System, Self::English, Self::French, Self::Spanish];

    fn code(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::English => Some("en"),
            Self::French => Some("fr"),
            Self::Spanish => Some("es"),
        }
    }
}

/// The country release dates and titles are localized for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataRegion {
    #[default]
    Auto,
    France,
    UnitedStates,
    Japan,
}

impl MetadataRegion {
    pub const ALL: [Self; 4] = [Self::Auto, Self::France, Self::UnitedStates, Self::Japan];

    fn code(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::France => Some("FR"),
            Self::UnitedStates => Some("US"),
            Self::Japan => Some("JP"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Config {
    #[serde(flatten)]
    pub standard: StandardPreferences,
    pub language: Language,
    pub metadata_language: MetadataLanguage,
    pub metadata_region: MetadataRegion,
    /// Ask before files are renamed for real.
    pub confirm_before_rename: bool,
    /// Ask before the file list is cleared.
    pub confirm_before_clearing: bool,
    /// Start every session in dry-run mode.
    pub dry_run_by_default: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            standard: StandardPreferences::default(),
            language: Language::System,
            metadata_language: MetadataLanguage::System,
            metadata_region: MetadataRegion::Auto,
            confirm_before_rename: true,
            confirm_before_clearing: true,
            dry_run_by_default: false,
        }
    }
}

impl Config {
    /// Reads the preferences from the config directory: `preferences.json`,
    /// or once, the `preferences.toml` of an older version, which is then
    /// saved in the new format. Defaults when neither exists.
    pub fn load() -> Self {
        let Some(dir) = paths::config_dir() else {
            return Self::default();
        };
        Self::load_from(&dir)
    }

    fn load_from(dir: &Path) -> Self {
        if let Ok(text) = fs::read_to_string(dir.join(FILE_NAME)) {
            return Self::from_json(&text);
        }
        let Ok(text) = fs::read_to_string(dir.join(LEGACY_FILE_NAME)) else {
            return Self::default();
        };
        let config = Self::from_legacy_toml(&text);
        if let Err(err) = config.save_to(&dir.join(FILE_NAME)) {
            eprintln!("Failed to save the migrated preferences: {err}");
        }
        config
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else {
            return;
        };
        if let Err(err) = self.save_to(&path) {
            eprintln!("Failed to save preferences to {}: {err}", path.display());
        }
    }

    fn save_to(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, json)
    }

    pub fn path() -> Option<PathBuf> {
        paths::config_dir().map(|dir| dir.join(FILE_NAME))
    }

    /// Parses `preferences.json`. A field with a value of the wrong type or an
    /// unknown variant keeps its default; the other fields are kept.
    pub fn from_json(text: &str) -> Self {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return Self::default();
        };
        let defaults = Self::default();
        let field = |key: &str| value.get(key);
        Self {
            standard: StandardPreferences::deserialize(&value).unwrap_or_default(),
            language: parse_or(field("language"), defaults.language),
            metadata_language: parse_or(field("metadata_language"), defaults.metadata_language),
            metadata_region: parse_or(field("metadata_region"), defaults.metadata_region),
            confirm_before_rename: parse_or(
                field("confirm_before_rename"),
                defaults.confirm_before_rename,
            ),
            confirm_before_clearing: parse_or(
                field("confirm_before_clearing"),
                defaults.confirm_before_clearing,
            ),
            dry_run_by_default: parse_or(field("dry_run_by_default"), defaults.dry_run_by_default),
        }
    }

    /// Parses the `preferences.toml` of older versions, in either of its two
    /// shapes: flat, or in `[Connections]`, `[Language]`, `[Appearance]`,
    /// `[Experience]` and `[Utilities]` sections. Settings that were never
    /// implemented are dropped.
    pub fn from_legacy_toml(text: &str) -> Self {
        let table: toml::Table = toml::from_str(text).unwrap_or_default();
        let lookup = |section: &str, key: &str| {
            table
                .get(section)
                .and_then(|section| section.get(key))
                .or_else(|| table.get(key))
                .cloned()
        };
        let defaults = Self::default();
        let mut config = Self {
            metadata_language: parse_toml_or(
                lookup("Language", "language"),
                defaults.metadata_language,
            ),
            metadata_region: parse_toml_or(lookup("Language", "region"), defaults.metadata_region),
            confirm_before_rename: parse_toml_or(
                lookup("Connections", "confirm_before_rename"),
                defaults.confirm_before_rename,
            ),
            confirm_before_clearing: parse_toml_or(
                lookup("Utilities", "confirm_before_clearing"),
                defaults.confirm_before_clearing,
            ),
            ..defaults
        };
        config.standard.animations = parse_toml_or(
            lookup("Appearance", "animate_transitions"),
            config.standard.animations,
        );
        // The old themes were the four Catppuccin flavours, plus "system".
        if let Some(flavour) = lookup("Appearance", "theme")
            .and_then(|value| value.as_str().map(str::to_string))
            .filter(|flavour| ["latte", "frappe", "macchiato", "mocha"].contains(&flavour.as_str()))
        {
            config.standard.selected_theme = "catppuccin".to_string();
            config.standard.selected_variant = flavour;
        }
        config
    }

    /// The locale to ask the metadata sources for, if any.
    pub fn metadata_locale(&self) -> Option<MetadataLocale> {
        let locale = MetadataLocale {
            language: self.metadata_language.code().map(str::to_string),
            region: self.metadata_region.code().map(str::to_string),
        };
        (!locale.is_empty()).then_some(locale)
    }
}

fn parse_or<T: DeserializeOwned>(value: Option<&serde_json::Value>, default: T) -> T {
    value
        .and_then(|value| T::deserialize(value).ok())
        .unwrap_or(default)
}

fn parse_toml_or<T: DeserializeOwned>(value: Option<toml::Value>, default: T) -> T {
    value
        .and_then(|value| value.try_into().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn a_bad_value_only_resets_its_own_field() {
        let config = Config::from_json(
            r#"{
                "selected_theme": "catppuccin",
                "selected_variant": "mocha",
                "font_size": "enormous",
                "language": "klingon",
                "metadata_language": "spanish",
                "confirm_before_rename": "yes",
                "confirm_before_clearing": false,
                "unknown_field": 3
            }"#,
        );
        assert_eq!(config.standard.selected_theme, "catppuccin");
        assert_eq!(config.standard.selected_variant, "mocha");
        assert_eq!(config.standard.font_size, colony_ui::FontSize::Default);
        assert_eq!(config.language, Language::System);
        assert_eq!(config.metadata_language, MetadataLanguage::Spanish);
        assert!(config.confirm_before_rename);
        assert!(!config.confirm_before_clearing);
    }

    #[test]
    fn garbage_is_the_defaults() {
        assert_eq!(Config::from_json("not json"), Config::default());
        assert_eq!(Config::from_json("[1, 2]"), Config::default());
    }

    #[test]
    fn json_round_trips() {
        let config = Config {
            language: Language::French,
            metadata_region: MetadataRegion::Japan,
            dry_run_by_default: true,
            ..Config::default()
        };
        let json = serde_json::to_string(&config).expect("serialize");
        assert_eq!(Config::from_json(&json), config);
    }

    #[test]
    fn the_sectioned_legacy_format_is_read() {
        let config = Config::from_legacy_toml(
            "[Connections]\n\
             open_last_session = true\n\
             confirm_before_rename = false\n\
             [Language]\n\
             language = \"french\"\n\
             region = \"france\"\n\
             date_format = \"dd_mm_yyyy\"\n\
             [Appearance]\n\
             theme = \"latte\"\n\
             animate_transitions = false\n\
             [Utilities]\n\
             confirm_before_clearing = \"sometimes\"\n",
        );
        assert!(!config.confirm_before_rename);
        assert!(
            config.confirm_before_clearing,
            "a bad value keeps the default"
        );
        assert_eq!(config.metadata_language, MetadataLanguage::French);
        assert_eq!(config.metadata_region, MetadataRegion::France);
        assert_eq!(config.standard.selected_theme, "catppuccin");
        assert_eq!(config.standard.selected_variant, "latte");
        assert!(!config.standard.animations);
        assert_eq!(config.language, Language::System);
    }

    #[test]
    fn the_flat_legacy_format_is_read() {
        let config = Config::from_legacy_toml(
            "theme = \"system\"\nlanguage = \"spanish\"\nregion = \"united_states\"\n\
             confirm_before_clearing = false\n",
        );
        assert_eq!(config.standard, StandardPreferences::default());
        assert_eq!(config.metadata_language, MetadataLanguage::Spanish);
        assert_eq!(config.metadata_region, MetadataRegion::UnitedStates);
        assert!(!config.confirm_before_clearing);
        assert_eq!(
            config.metadata_locale(),
            Some(MetadataLocale {
                language: Some("es".to_string()),
                region: Some("US".to_string()),
            })
        );
    }

    #[test]
    fn the_legacy_file_is_migrated_once() {
        let dir = TempDir::new("config-migrate");
        fs::write(
            dir.join(LEGACY_FILE_NAME),
            "[Utilities]\nconfirm_before_clearing = false\n",
        )
        .expect("legacy file");

        let config = Config::load_from(&dir);
        assert!(!config.confirm_before_clearing);
        assert!(dir.join(FILE_NAME).exists());

        // From now on preferences.json is the one read.
        fs::write(
            dir.join(LEGACY_FILE_NAME),
            "[Utilities]\nconfirm_before_clearing = true\n",
        )
        .expect("rewrite legacy");
        assert!(!Config::load_from(&dir).confirm_before_clearing);
    }

    #[test]
    fn no_file_is_the_defaults() {
        let dir = TempDir::new("config-empty");
        assert_eq!(Config::load_from(&dir), Config::default());
        assert!(!dir.join(FILE_NAME).exists());
    }
}
