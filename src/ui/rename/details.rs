//! The match dialog of one file: every title the lookup found, the selected
//! title's details and episodes, and a manual override of the name parts.

use colony_ui::theme::Palette;
use colony_ui::{Typography, widgets as kit};
use iced::widget::{button, column, row, scrollable, space, text};
use iced::{Element, Length};

use super::preview::{file_name, title_summary};
use super::{Message, Rename};
use crate::i18n::{t, tf};
use crate::matching::ContentType;
use crate::metadata::models::TitleMatch;
use crate::ui::widgets::{self, Tone};

pub fn view<'a>(state: &'a Rename, typo: &Typography, index: usize) -> Element<'a, Message> {
    let Some(file) = state.files.get(index) else {
        return widgets::dialog(
            typo,
            t("details_title"),
            widgets::label(typo, t("details_missing"), 13, Palette::TEXT_MUTED()),
            860.0,
        );
    };
    let content_type = state
        .content_type_for_file(file)
        .unwrap_or_else(|| state.active_content_type());

    let body = column![
        widgets::label(
            typo,
            tf("details_file", &[("file", &file_name(file))]),
            13,
            Palette::TEXT_SECONDARY(),
        ),
        row![
            titles(state, typo).width(Length::FillPortion(1)),
            selected_title(state, typo, content_type).width(Length::FillPortion(1)),
        ]
        .spacing(16),
        manual_override(state, typo),
    ]
    .spacing(16);

    widgets::dialog(
        typo,
        t("details_title"),
        scrollable(body).height(Length::Shrink),
        860.0,
    )
}

fn titles<'a>(state: &'a Rename, typo: &Typography) -> iced::widget::Column<'a, Message> {
    let heading = widgets::heading(typo, t("details_matches"), 14);
    if state.titles.is_empty() {
        return column![
            heading,
            widgets::label(typo, t("details_fetch_first"), 12, Palette::TEXT_MUTED()),
        ]
        .spacing(8);
    }
    let list = state.titles.iter().map(|title| {
        let selected = state.selected_title.as_deref() == Some(title.id.as_str());
        button(
            text(title_summary(title))
                .size(typo.sz(12))
                .font(typo.regular),
        )
        .on_press(Message::PickTitle(title.id.clone()))
        .width(Length::Fill)
        .padding([6, 10])
        .style(kit::selection_style(selected))
        .into()
    });
    column![
        heading,
        widgets::well(scrollable(column(list).spacing(2)).height(Length::Shrink))
            .max_height(260.0 * typo.scale),
    ]
    .spacing(8)
}

fn selected_title<'a>(
    state: &'a Rename,
    typo: &Typography,
    content_type: ContentType,
) -> iced::widget::Column<'a, Message> {
    let mut column = column![widgets::heading(typo, t("details_source"), 14)].spacing(4);
    match state.selected_title_match() {
        Some(title) => {
            for line in title_lines(title) {
                column = column.push(widgets::label(typo, line, 12, Palette::TEXT_SECONDARY()));
            }
            if let Some(synopsis) = title.extras.synopsis.as_deref() {
                column = column
                    .push(space().height(4))
                    .push(widgets::label(
                        typo,
                        t("details_synopsis"),
                        12,
                        Palette::TEXT_PRIMARY(),
                    ))
                    .push(widgets::label(typo, synopsis, 11, Palette::TEXT_DIMMER()));
            }
        }
        None => {
            column = column.push(widgets::label(
                typo,
                t("details_select"),
                12,
                Palette::TEXT_MUTED(),
            ));
        }
    }
    if content_type == ContentType::Series {
        column = column
            .push(space().height(8))
            .push(widgets::heading(typo, t("details_episodes"), 14))
            .push(episodes(state, typo));
    }
    column
}

fn title_lines(title: &TitleMatch) -> Vec<String> {
    let unknown = t("value_unknown");
    let mut lines = vec![
        tf("details_name", &[("name", &title.name)]),
        tf("details_source_line", &[("source", &title.source)]),
        tf(
            "details_score",
            &[
                ("score", &format!("{:.2}", title.global_score)),
                ("source_score", &format!("{:.2}", title.source_score)),
                ("trust", &format!("{:.2}", title.source_trust)),
            ],
        ),
        tf(
            "details_language",
            &[(
                "language",
                &title.extras.language.as_deref().unwrap_or(unknown),
            )],
        ),
        tf(
            "details_year",
            &[(
                "year",
                &title
                    .year
                    .map_or_else(|| unknown.to_string(), |year| year.to_string()),
            )],
        ),
        tf("details_id", &[("id", &title.id)]),
    ];
    let ids = &title.extras.external_ids;
    let mut external: Vec<String> = [
        ("imdb", &ids.imdb),
        ("tmdb", &ids.tmdb),
        ("tvdb", &ids.tvdb),
        ("tvmaze", &ids.tvmaze),
        ("omdb", &ids.omdb),
    ]
    .iter()
    .filter_map(|(name, id)| id.as_deref().map(|id| format!("{name}:{id}")))
    .collect();
    external.extend(
        ids.other
            .iter()
            .map(|other| format!("{}:{}", other.source, other.id)),
    );
    let external = if external.is_empty() {
        unknown.to_string()
    } else {
        external.join(", ")
    };
    lines.push(tf("details_external_ids", &[("ids", &external)]));
    if !title.extras.aliases.is_empty() {
        lines.push(tf(
            "details_aliases",
            &[("aliases", &title.extras.aliases.join(", "))],
        ));
    }
    if !title.extras.genres.is_empty() {
        lines.push(tf(
            "details_genres",
            &[("genres", &title.extras.genres.join(", "))],
        ));
    }
    lines
}

fn episodes<'a>(state: &'a Rename, typo: &Typography) -> Element<'a, Message> {
    if state.loading_episodes {
        return widgets::label(typo, t("details_loading_episodes"), 12, Palette::ACCENT()).into();
    }
    if state.episodes.is_empty() {
        return widgets::label(typo, t("details_no_episodes"), 12, Palette::TEXT_MUTED()).into();
    }
    let current = state.details_override().cloned().unwrap_or_default();
    let list = state.episodes.iter().map(|episode| {
        let selected =
            current.season == Some(episode.season) && current.episode == Some(episode.episode);
        let label = tf(
            "results_episode",
            &[
                ("season", &format!("{:02}", episode.season)),
                ("episode", &format!("{:02}", episode.episode)),
                ("title", &episode.title),
            ],
        );
        button(text(label).size(typo.sz(12)).font(typo.regular))
            .on_press(Message::PickEpisode(episode.season, episode.episode))
            .width(Length::Fill)
            .padding([4, 10])
            .style(kit::selection_style(selected))
            .into()
    });
    widgets::well(scrollable(column(list).spacing(2)).height(Length::Shrink))
        .max_height(200.0 * typo.scale)
        .into()
}

fn manual_override<'a>(state: &'a Rename, typo: &Typography) -> Element<'a, Message> {
    let draft = &state.draft;
    let fields = row![
        field(
            typo,
            t("details_override_title"),
            &draft.title,
            Message::OverrideTitle,
            3
        ),
        field(
            typo,
            t("details_override_season"),
            &draft.season,
            Message::OverrideSeason,
            1
        ),
        field(
            typo,
            t("details_override_episode"),
            &draft.episode,
            Message::OverrideEpisode,
            1
        ),
    ]
    .spacing(8);
    let mut section = column![
        widgets::heading(typo, t("details_override"), 14),
        widgets::label(typo, t("details_override_hint"), 11, Palette::TEXT_DIMMER()),
        fields,
    ]
    .spacing(8);
    if draft.parse().is_none() {
        section = section.push(widgets::label(
            typo,
            t("details_invalid_number"),
            11,
            Palette::WARNING(),
        ));
    }
    section
        .push(
            row![
                widgets::text_button(
                    typo,
                    t("details_clear_override"),
                    Tone::Default,
                    Some(Message::ClearOverride),
                ),
                space::horizontal(),
                widgets::text_button(
                    typo,
                    t("details_close"),
                    Tone::Primary,
                    Some(Message::CloseDetails)
                ),
            ]
            .spacing(8),
        )
        .into()
}

fn field<'a>(
    typo: &Typography,
    name: &str,
    value: &str,
    on_input: fn(String) -> Message,
    portion: u16,
) -> Element<'a, Message> {
    column![
        widgets::label(typo, name, 12, Palette::TEXT_MUTED()),
        widgets::field(typo, "", value, on_input),
    ]
    .spacing(4)
    .width(Length::FillPortion(portion))
    .into()
}
