//! The proposed names, the Match and Rename actions, and the options the
//! lookup and the naming work with.

use colony_ui::theme::Palette;
use colony_ui::{Typography, widgets as kit};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use super::{DetectionNotice, FetchError, FetchState, Message, Rename};
use crate::i18n::{t, tf};
use crate::lookup;
use crate::matching::{ContentType, MatchResult, MatchStatus};
use crate::metadata::error::MetadataError;
use crate::metadata::models::TitleMatch;
use crate::ui::widgets::{self, Tone, icons};

/// Match and Rename, between the two lists.
pub fn actions<'a>(state: &Rename, typo: &Typography) -> Element<'a, Message> {
    let idle = !state.busy();
    let match_label = if state.matching {
        t("action_matching")
    } else {
        t("action_match")
    };
    container(
        column![
            widgets::action(
                typo,
                icons::MATCH,
                match_label,
                Tone::Default,
                (idle && !state.files.is_empty()).then_some(Message::Match),
            ),
            widgets::action(
                typo,
                icons::RENAME,
                t("action_rename"),
                Tone::Primary,
                (idle && !state.results.is_empty()).then_some(Message::Rename),
            ),
        ]
        .spacing(12)
        .align_x(Alignment::Center),
    )
    .center_y(Length::Fill)
    .into()
}

/// The New names list and its toolbar.
pub fn view<'a>(state: &'a Rename, typo: &Typography) -> Element<'a, Message> {
    let list: Element<'a, Message> = if state.matching {
        placeholder(typo, t("names_loading"))
    } else if let FetchState::Failed(err) = &state.fetch {
        widgets::label(typo, fetch_error_text(err), 13, Palette::ERROR()).into()
    } else if state.results.is_empty() {
        placeholder(typo, t("names_empty"))
    } else {
        scrollable(
            column(
                state
                    .results
                    .iter()
                    .enumerate()
                    .map(|(index, result)| name_row(state, typo, index, result)),
            )
            .spacing(2),
        )
        .height(Length::Fill)
        .into()
    };

    let fetch_label = if matches!(state.fetch, FetchState::Loading) {
        t("names_fetching")
    } else {
        t("names_fetch")
    };
    let toolbar = row![
        widgets::action(
            typo,
            icons::FETCH,
            fetch_label,
            Tone::Default,
            (!state.busy()).then_some(Message::Fetch),
        ),
        widgets::action(
            typo,
            icons::ADJUST,
            t("names_adjust"),
            Tone::Default,
            (!state.files.is_empty()).then_some(Message::OpenDetails),
        ),
    ]
    .spacing(6)
    .wrap();

    widgets::card(
        column![
            widgets::heading(typo, t("names_title"), 14),
            widgets::well(list).height(Length::Fill).width(Length::Fill),
            toolbar,
        ]
        .spacing(8),
    )
    .width(Length::FillPortion(1))
    .height(Length::Fill)
    .into()
}

pub(super) fn placeholder<'a>(typo: &Typography, message: &str) -> Element<'a, Message> {
    container(widgets::label(typo, message, 13, Palette::TEXT_MUTED()))
        .padding(8)
        .into()
}

fn name_row<'a>(
    state: &Rename,
    typo: &Typography,
    index: usize,
    result: &MatchResult,
) -> Element<'a, Message> {
    let name = state
        .format_preview(result)
        .unwrap_or_else(|| t("names_no_name").to_string());
    let (status, status_color) = match result.status {
        MatchStatus::Ok => (t("match_ok"), Palette::SUCCESS()),
        MatchStatus::Ambiguous => (t("match_ambiguous"), Palette::WARNING()),
        MatchStatus::Error => (t("match_error"), Palette::ERROR()),
    };
    let mut first_line = row![
        text(name).size(typo.sz(13)).font(typo.regular),
        widgets::label(typo, status, 11, status_color),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if let Some(content_type) = lookup::content_type_of(result) {
        first_line = first_line.push(
            text(content_type_label(content_type))
                .size(typo.sz(10))
                .font(typo.regular),
        );
    }

    let mut details = vec![tf("names_from", &[("file", &file_name(&result.original))])];
    if let Some(source) = result
        .episode_match
        .as_ref()
        .map(|episode| episode.source.as_str())
        .or_else(|| {
            result
                .title_match
                .as_ref()
                .map(|title| title.source.as_str())
        })
    {
        details.push(tf("names_source", &[("source", &source)]));
    }
    let confidence = result.metadata_confidence.unwrap_or(result.confidence);
    details.push(tf(
        "names_confidence",
        &[("percent", &format!("{:.0}", confidence * 100.0))],
    ));
    let candidates = result
        .metadata_candidate_count
        .unwrap_or(result.candidates.len());
    details.push(tf("names_candidates", &[("count", &candidates)]));

    let mut lines = column![
        first_line,
        text(details.join(" \u{2022} "))
            .size(typo.sz(10))
            .font(typo.regular),
    ]
    .spacing(2);
    if let Some(summary) = state
        .summaries
        .iter()
        .find(|summary| summary.original == result.original)
    {
        lines = lines.push(widgets::label(
            typo,
            super::summary::message(summary),
            10,
            super::summary::color(&summary.outcome),
        ));
    }

    button(lines)
        .on_press(Message::Select(index))
        .width(Length::Fill)
        .padding([6, 10])
        .style(kit::selection_style(state.selected == Some(index)))
        .into()
}

pub(super) fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map_or_else(
        || path.to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

pub(super) fn content_type_label(content_type: ContentType) -> &'static str {
    match content_type {
        ContentType::Movie => t("content_movie"),
        ContentType::Series => t("content_series"),
    }
}

/// A lookup failure, in the interface language. The source's own detail is
/// technical and stays as the source reported it.
pub fn fetch_error_text(err: &FetchError) -> String {
    match err {
        FetchError::NoTitle => t("lookup_error_no_title").to_string(),
        FetchError::Metadata(err) => {
            let (key, detail) = match err {
                MetadataError::Network(detail) => ("metadata_error_network", detail),
                MetadataError::NotFound(detail) => ("metadata_error_not_found", detail),
                MetadataError::InvalidResponse(detail) => ("metadata_error_invalid", detail),
                MetadataError::RateLimited(detail) => ("metadata_error_rate_limited", detail),
                MetadataError::Other(detail) => ("metadata_error_other", detail),
            };
            tf(key, &[("detail", detail)])
        }
    }
}

/// The lookup, naming and results cards under the lists.
pub fn options<'a>(state: &'a Rename, typo: &Typography) -> Element<'a, Message> {
    let width = 340.0 * typo.scale;
    let cards = row![
        lookup_card(state, typo).width(width),
        naming_card(state, typo).width(width),
        results_card(state, typo).width(width),
    ]
    .spacing(12)
    .wrap()
    .vertical_spacing(12);
    container(scrollable(cards).height(Length::Shrink))
        .max_height(320.0 * typo.scale)
        .into()
}

fn lookup_card<'a>(state: &'a Rename, typo: &Typography) -> container::Container<'a, Message> {
    let content_type = state.active_content_type();
    let mut card = column![
        widgets::heading(typo, t("lookup_title"), 14),
        labelled(
            typo,
            t("lookup_type"),
            kit::dropdown(
                typo,
                [ContentType::Series, ContentType::Movie]
                    .map(|kind| (kind, content_type_label(kind).to_string())),
                &content_type,
                Message::ContentType,
            ),
        ),
        labelled(
            typo,
            t("lookup_name"),
            widgets::field(
                typo,
                t("lookup_name_placeholder"),
                &state.detected_name,
                Message::DetectedName
            )
            .on_submit(Message::Fetch),
        ),
        labelled(
            typo,
            t("lookup_source"),
            kit::dropdown(
                typo,
                lookup::sources_for(content_type)
                    .iter()
                    .map(|source| (*source, source.label().to_string())),
                &state.plan.primary(content_type),
                Message::Source,
            ),
        ),
        widgets::check(
            typo,
            t("lookup_force"),
            state.plan.force,
            Message::ForceSource
        ),
    ]
    .spacing(8);
    if let Some(notice) = state.notice {
        let key = match notice {
            DetectionNotice::Ambiguous => "lookup_notice_ambiguous",
            DetectionNotice::Unknown => "lookup_notice_unknown",
        };
        card = card.push(
            row![
                widgets::glyph(typo, icons::WARNING, 12, Palette::WARNING()),
                widgets::label(typo, t(key), 12, Palette::WARNING()),
            ]
            .spacing(6),
        );
    }
    if matches!(state.fetch, FetchState::Loading) {
        card = card.push(widgets::label(
            typo,
            t("lookup_loading"),
            12,
            Palette::ACCENT(),
        ));
    }
    widgets::card(card)
}

fn naming_card<'a>(state: &'a Rename, typo: &Typography) -> container::Container<'a, Message> {
    let content_type = state.active_content_type();
    widgets::card(
        column![
            widgets::heading(typo, t("naming_title"), 14),
            widgets::check(
                typo,
                t("naming_episode_title"),
                state.format.include_episode_title,
                Message::IncludeEpisodeTitle,
            ),
            widgets::check(
                typo,
                t("naming_year"),
                state.format.include_year,
                Message::IncludeYear
            ),
            labelled(
                typo,
                t("naming_template"),
                widgets::field(
                    typo,
                    state.template_placeholder(content_type),
                    state.template(content_type),
                    Message::Template,
                ),
            ),
            row![
                widgets::text_button(
                    typo,
                    t("naming_preset"),
                    Tone::Default,
                    Some(Message::TemplatePreset)
                ),
                widgets::text_button(
                    typo,
                    t("naming_reset"),
                    Tone::Default,
                    Some(Message::TemplateReset)
                ),
            ]
            .spacing(6)
            .wrap(),
            widgets::label(
                typo,
                tf(
                    "naming_tokens",
                    &[("template", &state.template_placeholder(content_type))]
                ),
                10,
                Palette::TEXT_DIMMER(),
            ),
            widgets::check(typo, t("naming_dry_run"), state.dry_run, Message::DryRun),
        ]
        .spacing(8),
    )
}

fn results_card<'a>(state: &'a Rename, typo: &Typography) -> container::Container<'a, Message> {
    let title = match state.answered_by {
        Some(source) => tf("results_from", &[("source", &source.label())]),
        None => t("results_title").to_string(),
    };
    let lines: Vec<String> = if state.active_content_type() == ContentType::Series {
        state
            .episodes
            .iter()
            .map(|episode| {
                tf(
                    "results_episode",
                    &[
                        ("season", &format!("{:02}", episode.season)),
                        ("episode", &format!("{:02}", episode.episode)),
                        ("title", &episode.title),
                    ],
                )
            })
            .collect()
    } else {
        state.titles.iter().map(title_summary).collect()
    };
    let body: Element<'a, Message> = if lines.is_empty() {
        let key = if state.active_content_type() == ContentType::Series {
            "results_no_episodes"
        } else {
            "results_no_titles"
        };
        widgets::label(typo, t(key), 12, Palette::TEXT_MUTED()).into()
    } else {
        scrollable(
            column(
                lines
                    .into_iter()
                    .map(|line| widgets::label(typo, line, 12, Palette::TEXT_SECONDARY()).into()),
            )
            .spacing(2),
        )
        .height(Length::Shrink)
        .into()
    };
    widgets::card(column![widgets::heading(typo, title, 14), body].spacing(8))
}

/// One line about a title: name, source, score, language and year.
pub(super) fn title_summary(title: &TitleMatch) -> String {
    tf(
        "results_title_line",
        &[
            ("name", &title.name),
            ("source", &title.source),
            ("score", &format!("{:.2}", title.global_score)),
            (
                "language",
                &title
                    .extras
                    .language
                    .as_deref()
                    .unwrap_or(t("value_unknown")),
            ),
            (
                "year",
                &title
                    .year
                    .map_or_else(|| t("value_unknown").to_string(), |year| year.to_string()),
            ),
        ],
    )
}

/// A control with its name above it.
fn labelled<'a>(
    typo: &Typography,
    name: &str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![
        widgets::label(typo, name, 12, Palette::TEXT_MUTED()),
        control.into(),
    ]
    .spacing(4)
    .into()
}
