//! The Original files list and its toolbar.

use colony_ui::{Typography, widgets as kit};
use iced::widget::{button, column, row, scrollable, text};
use iced::{Element, Length};

use super::preview::{file_name, placeholder};
use super::{Message, Rename};
use crate::i18n::{t, tf};
use crate::ui::widgets::{self, Tone, icons};

pub fn view<'a>(state: &'a Rename, typo: &Typography) -> Element<'a, Message> {
    let list: Element<'a, Message> =
        if state.files.is_empty() {
            placeholder(typo, t("files_empty"))
        } else {
            scrollable(
                column(state.files.iter().enumerate().map(|(index, path)| {
                    file_row(typo, index, path, state.selected == Some(index))
                }))
                .spacing(2),
            )
            .height(Length::Fill)
            .into()
        };

    let selected = state.selected.filter(|&index| index < state.files.len());
    let has_files = !state.files.is_empty();
    let toolbar = row![
        widgets::action(
            typo,
            icons::ADD,
            t("files_add"),
            Tone::Default,
            Some(Message::AddFiles)
        ),
        widgets::action(
            typo,
            icons::FOLDER,
            t("files_add_folder"),
            Tone::Default,
            Some(Message::AddFolder),
        ),
        widgets::icon_button(
            typo,
            icons::UP,
            t("files_move_up"),
            selected.filter(|&index| index > 0).map(|_| Message::MoveUp),
        ),
        widgets::icon_button(
            typo,
            icons::DOWN,
            t("files_move_down"),
            selected
                .filter(|&index| index + 1 < state.files.len())
                .map(|_| Message::MoveDown),
        ),
        widgets::icon_button(
            typo,
            icons::REMOVE,
            t("files_remove"),
            selected.map(|_| Message::Remove)
        ),
        widgets::icon_button(
            typo,
            icons::RESCAN,
            t("files_rescan"),
            has_files.then_some(Message::Rescan)
        ),
        widgets::icon_button(
            typo,
            icons::CLEAR,
            t("files_clear"),
            has_files.then_some(Message::Clear)
        ),
    ]
    .spacing(6)
    .wrap()
    .vertical_spacing(6);

    let title = if has_files {
        tf("files_title_count", &[("count", &state.files.len())])
    } else {
        t("files_title").to_string()
    };

    widgets::card(
        column![
            widgets::heading(typo, title, 14),
            widgets::well(list).height(Length::Fill).width(Length::Fill),
            toolbar,
        ]
        .spacing(8),
    )
    .width(Length::FillPortion(1))
    .height(Length::Fill)
    .into()
}

/// A file: its name, then the folder it is in, smaller.
fn file_row<'a>(
    typo: &Typography,
    index: usize,
    path: &str,
    selected: bool,
) -> Element<'a, Message> {
    let folder = std::path::Path::new(path)
        .parent()
        .map(|parent| parent.display().to_string())
        .unwrap_or_default();
    button(
        column![
            text(file_name(path)).size(typo.sz(13)).font(typo.regular),
            text(folder).size(typo.sz(10)).font(typo.regular),
        ]
        .spacing(2),
    )
    .on_press(Message::Select(index))
    .width(Length::Fill)
    .padding([6, 10])
    .style(kit::selection_style(selected))
    .into()
}
