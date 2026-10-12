//! The sidebar: KayaBot's name, which opens Preferences, then the sections.

use colony_ui::theme::Palette;
use colony_ui::{Typography, widgets as kit};
use iced::widget::{column, container, space};
use iced::{Element, Length};

use super::app::{App, Message};
use crate::i18n::{t, tf};

pub fn view<'a>(app: &App, typo: &Typography) -> Element<'a, Message> {
    let modifier = if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    };
    let sidebar = column![
        kit::identity_button(
            typo,
            crate::paths::PROGRAM,
            30,
            app.preferences_open,
            Message::TogglePreferences,
        ),
        space().height(24),
        kit::nav_label(typo, t("nav_label")),
        kit::nav_item(
            typo,
            t("nav_rename"),
            !app.preferences_open,
            Message::ShowRename
        ),
        space::vertical(),
        kit::nav_hint(typo, &tf("nav_hint", &[("modifier", &modifier)])),
    ]
    .spacing(10)
    .padding(16)
    // Grows with the text, so the name still fits at the largest sizes.
    .width(220.0 * typo.scale);

    container(sidebar)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Palette::BG_SIDEBAR().into()),
            ..Default::default()
        })
        .into()
}
