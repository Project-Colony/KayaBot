//! The iced interface, built on the Colony design system from `colony-ui`.

mod app;
mod preferences;
mod rename;
mod sidebar;
mod widgets;

pub use app::{App, Message};

use crate::config::Config;

/// Opens the window and runs until it is closed. The preferences are put into
/// effect first, so the very first frame is drawn in the user's theme, font
/// and language.
pub fn run(config: Config) -> iced::Result {
    config.standard.apply();
    crate::i18n::set_locale(config.language.locale());

    colony_ui::fonts::BYTES
        .into_iter()
        .fold(
            iced::application(move || App::new(config.clone()), App::update, App::view),
            |app, font| app.font(font),
        )
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .default_font(colony_ui::ui_font())
        .window_size((1180.0, 760.0))
        .run()
}
