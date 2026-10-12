//! The Preferences page: General, Appearance and Accessibility from the
//! colony-ui kit, then Connections, Renaming and About.

use colony_ui::Typography;
use colony_ui::theme::Palette;
use colony_ui::widgets::{self as kit, ABOUT_CATEGORY, PreferencesView, STANDARD_CATEGORIES};
use iced::widget::{Column, column};
use iced::{Element, Length};

use super::app::{App, KeyField, Message};
use super::widgets;
use crate::api_keys::ApiKeysLoadState;
use crate::config::{Language, MetadataLanguage, MetadataRegion};
use crate::i18n::{t, tf};
use crate::matching::ContentType;
use crate::metadata::provider::MetadataSource;

/// The program's own categories, after the three standard ones.
const OWN_CATEGORIES: [&str; 2] = ["preferences_cat_connections", "preferences_cat_renaming"];

pub fn view<'a>(app: &'a App, typo: &Typography) -> Element<'a, Message> {
    let kit = PreferencesView::new(
        *typo,
        &app.config.standard,
        &app.expanded,
        Message::Preference,
        Message::ToggleSection,
    );
    let mut content: Column<'a, Message> = match app.category {
        0 => kit.general().push(language_section(app, typo)),
        1 => kit.appearance(),
        2 => kit.accessibility(),
        3 => connections(app, typo),
        4 => renaming(app, typo),
        _ => about(typo),
    };
    if let Some(problem) = &app.problem {
        content = content.push(widgets::label(typo, problem.clone(), 12, Palette::ERROR()));
    }

    let labels = STANDARD_CATEGORIES
        .iter()
        .chain(&OWN_CATEGORIES)
        .chain([&ABOUT_CATEGORY])
        .map(|key| t(key));
    kit::preferences_page(
        typo,
        kit::category_list(typo, labels, app.category, Message::Category),
        content,
        Message::TogglePreferences,
    )
}

fn section<'a>(
    app: &App,
    typo: &Typography,
    key: &'static str,
    title: &str,
    content: Column<'a, Message>,
) -> Element<'a, Message> {
    kit::collapsible_section(
        typo,
        title,
        app.expanded.contains(key),
        Message::ToggleSection(key),
        content.into(),
    )
}

fn language_section<'a>(app: &App, typo: &Typography) -> Element<'a, Message> {
    let language_label = |language| match language {
        Language::System => t("preferences_language_system"),
        Language::English => t("preferences_language_english"),
        Language::French => t("preferences_language_french"),
    };
    let metadata_label = |language| match language {
        MetadataLanguage::System => t("preferences_metadata_language_default"),
        MetadataLanguage::English => t("preferences_language_english"),
        MetadataLanguage::French => t("preferences_language_french"),
        MetadataLanguage::Spanish => t("preferences_language_spanish"),
    };
    let region_label = |region| match region {
        MetadataRegion::Auto => t("preferences_region_auto"),
        MetadataRegion::France => t("preferences_region_france"),
        MetadataRegion::UnitedStates => t("preferences_region_united_states"),
        MetadataRegion::Japan => t("preferences_region_japan"),
    };
    section(
        app,
        typo,
        "language",
        t("preferences_section_language"),
        column![
            kit::section_description(typo, t("preferences_language_desc")),
            kit::setting_row(
                typo,
                t("preferences_interface_language"),
                t("preferences_interface_language_desc"),
                kit::dropdown(
                    typo,
                    Language::ALL.map(|language| (language, language_label(language).to_string())),
                    &app.config.language,
                    Message::Language,
                ),
            ),
            kit::setting_row(
                typo,
                t("preferences_metadata_language"),
                t("preferences_metadata_language_desc"),
                kit::dropdown(
                    typo,
                    MetadataLanguage::ALL
                        .map(|language| (language, metadata_label(language).to_string())),
                    &app.config.metadata_language,
                    Message::MetadataLanguage,
                ),
            ),
            kit::setting_row(
                typo,
                t("preferences_metadata_region"),
                t("preferences_metadata_region_desc"),
                kit::dropdown(
                    typo,
                    MetadataRegion::ALL.map(|region| (region, region_label(region).to_string())),
                    &app.config.metadata_region,
                    Message::MetadataRegion,
                ),
            ),
        ],
    )
}

fn connections<'a>(app: &'a App, typo: &Typography) -> Column<'a, Message> {
    let key_row = |field, title: &str, value: &'a str, secure, configured: Option<bool>| {
        let description = match configured {
            Some(true) => t("preferences_key_configured"),
            Some(false) => t("preferences_key_missing"),
            None => t("preferences_tvmaze_agent_desc"),
        };
        kit::setting_row(
            typo,
            title,
            description,
            widgets::field(typo, "", value, move |value| Message::Key(field, value))
                .secure(secure)
                .width(Length::FillPortion(1)),
        )
    };
    let keys = column![
        kit::section_description(typo, t("preferences_keys_desc")),
        key_row(
            KeyField::Tmdb,
            MetadataSource::TheMovieDb.label(),
            &app.keys.tmdb,
            true,
            Some(app.api_config.tmdb_configured()),
        ),
        key_row(
            KeyField::Tvdb,
            MetadataSource::TheTvDb.label(),
            &app.keys.tvdb,
            true,
            Some(app.api_config.tvdb_configured()),
        ),
        key_row(
            KeyField::Omdb,
            MetadataSource::Omdb.label(),
            &app.keys.omdb,
            true,
            Some(app.api_config.omdb_configured()),
        ),
        key_row(
            KeyField::TvmazeUserAgent,
            t("preferences_tvmaze_agent"),
            &app.keys.tvmaze_user_agent,
            false,
            None,
        ),
        widgets::label(typo, t("preferences_keys_env"), 11, Palette::TEXT_DIMMER()),
    ]
    .spacing(4);

    let location = match &app.keys_status.path {
        Some(path) => {
            let key = match app.keys_status.state {
                ApiKeysLoadState::Loaded => "preferences_keys_file",
                ApiKeysLoadState::Missing => "preferences_keys_file_missing",
                ApiKeysLoadState::Unreadable => "preferences_keys_file_unreadable",
            };
            tf(key, &[("path", &path.display())])
        }
        None => t("preferences_no_config_dir").to_string(),
    };
    let storage = column![
        kit::section_description(typo, t("preferences_storage_desc")),
        widgets::label(typo, location, 12, Palette::TEXT_SECONDARY()),
        widgets::text_button(
            typo,
            t("preferences_open_folder"),
            widgets::Tone::Default,
            Some(Message::OpenConfigFolder),
        ),
    ]
    .spacing(8);

    kit::category_heading(
        typo,
        t("preferences_cat_connections"),
        t("preferences_connections_desc"),
    )
    .push(section(
        app,
        typo,
        "keys",
        t("preferences_section_keys"),
        keys,
    ))
    .push(section(
        app,
        typo,
        "storage",
        t("preferences_section_storage"),
        storage,
    ))
}

fn renaming<'a>(app: &'a App, typo: &Typography) -> Column<'a, Message> {
    let config = &app.config;
    let confirmations = column![
        kit::section_description(typo, t("preferences_confirmations_desc")),
        kit::functional_toggle(
            typo,
            t("preferences_confirm_rename"),
            t("preferences_confirm_rename_desc"),
            config.confirm_before_rename,
            Message::ConfirmBeforeRename,
        ),
        kit::functional_toggle(
            typo,
            t("preferences_confirm_clear"),
            t("preferences_confirm_clear_desc"),
            config.confirm_before_clearing,
            Message::ConfirmBeforeClearing,
        ),
    ];
    let template_row = |content_type, title: &str, description: &str| {
        kit::setting_row(
            typo,
            title,
            description,
            widgets::field(
                typo,
                app.rename.template_placeholder(content_type),
                app.rename.template(content_type),
                move |value| Message::Template(content_type, value),
            )
            .width(Length::FillPortion(1)),
        )
    };
    let naming = column![
        kit::section_description(typo, t("preferences_naming_desc")),
        template_row(
            ContentType::Series,
            t("preferences_series_template"),
            t("preferences_series_template_desc"),
        ),
        template_row(
            ContentType::Movie,
            t("preferences_movie_template"),
            t("preferences_movie_template_desc"),
        ),
        kit::functional_toggle(
            typo,
            t("preferences_dry_run"),
            t("preferences_dry_run_desc"),
            config.dry_run_by_default,
            Message::DryRunByDefault,
        ),
    ];

    kit::category_heading(
        typo,
        t("preferences_cat_renaming"),
        t("preferences_renaming_desc"),
    )
    .push(section(
        app,
        typo,
        "confirmations",
        t("preferences_section_confirmations"),
        confirmations,
    ))
    .push(section(
        app,
        typo,
        "naming",
        t("preferences_section_naming"),
        naming,
    ))
}

fn about<'a>(typo: &Typography) -> Column<'a, Message> {
    let line = |key: &str| widgets::label(typo, t(key), 12, Palette::TEXT_SECONDARY());
    kit::category_heading(
        typo,
        t(ABOUT_CATEGORY),
        &tf(
            "preferences_about_version",
            &[
                ("program", &crate::paths::PROGRAM),
                ("version", &env!("CARGO_PKG_VERSION")),
            ],
        ),
    )
    .push(line("preferences_about_description"))
    .push(line("preferences_about_license"))
    .push(widgets::heading(typo, t("preferences_about_sources"), 14))
    .push(line("preferences_about_tmdb"))
    .push(line("preferences_about_tvdb"))
    .push(line("preferences_about_tvmaze"))
    .push(line("preferences_about_omdb"))
    .spacing(8)
}
