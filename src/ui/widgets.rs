//! KayaBot's own building blocks, styled from the active palette and sized
//! through the user's typography. The shared ones are in `colony_ui::widgets`.

use colony_ui::theme::Palette;
use colony_ui::{Typography, contrast_on, fonts};
use iced::widget::{
    button, center, column, container, mouse_area, opaque, row, stack, text, tooltip,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

/// Nerd Font glyphs, written as codepoints and drawn in
/// `fonts::glyph_font()`. A test checks each one exists in the font.
pub mod icons {
    pub const ADD: &str = "\u{f067}";
    pub const FOLDER: &str = "\u{f07c}";
    pub const UP: &str = "\u{f062}";
    pub const DOWN: &str = "\u{f063}";
    pub const REMOVE: &str = "\u{f00d}";
    pub const RESCAN: &str = "\u{f021}";
    pub const CLEAR: &str = "\u{f1f8}";
    pub const MATCH: &str = "\u{f0ec}";
    pub const RENAME: &str = "\u{f061}";
    pub const FETCH: &str = "\u{f002}";
    pub const ADJUST: &str = "\u{f0ad}";
    pub const EXPORT: &str = "\u{f0c7}";
    pub const WARNING: &str = "\u{f071}";

    #[cfg(test)]
    pub const ALL: [&str; 13] = [
        ADD, FOLDER, UP, DOWN, REMOVE, RESCAN, CLEAR, MATCH, RENAME, FETCH, ADJUST, EXPORT, WARNING,
    ];
}

/// Text at `sz(size)` in the interface font, in `color`.
pub fn label<'a>(
    typo: &Typography,
    content: impl Into<String>,
    size: u16,
    color: Color,
) -> text::Text<'a> {
    text(content.into())
        .size(typo.sz(size))
        .font(typo.regular)
        .color(color)
}

/// A heading: `sz(size)`, bold, `text_primary`.
pub fn heading<'a>(typo: &Typography, content: impl Into<String>, size: u16) -> text::Text<'a> {
    text(content.into())
        .size(typo.sz(size))
        .font(typo.bold)
        .color(Palette::TEXT_PRIMARY())
}

/// A glyph from [`icons`], in the Nerd Font whatever the dyslexia toggle says.
pub fn glyph<'a>(typo: &Typography, icon: &'static str, size: u16, color: Color) -> text::Text<'a> {
    text(icon)
        .size(typo.sz(size))
        .font(fonts::glyph_font())
        .color(color)
}

/// A card: `bg_card`, a subtle border, radius 8.
pub fn card<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> container::Container<'a, M> {
    container(content)
        .padding(12)
        .style(|_theme| container::Style {
            background: Some(Palette::BG_CARD().into()),
            border: Border::default()
                .rounded(8)
                .width(1)
                .color(Palette::BORDER_SUBTLE()),
            ..Default::default()
        })
}

/// The sunken area a list scrolls in, inside a card.
pub fn well<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> container::Container<'a, M> {
    container(content)
        .padding(4)
        .style(|_theme| container::Style {
            background: Some(Palette::BG_INPUT().into()),
            border: Border::default()
                .rounded(6)
                .width(1)
                .color(Palette::BORDER_SUBTLE()),
            ..Default::default()
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// The everyday action.
    Default,
    /// The one action a screen is for.
    Primary,
    /// An action that throws something away.
    Danger,
}

fn button_style(tone: Tone) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (background, text_color) = match (tone, status) {
            (_, button::Status::Disabled) => (Palette::BTN_DEFAULT(), Palette::TEXT_DIMMEST()),
            (Tone::Default, button::Status::Hovered) => {
                (Palette::BTN_HOVER(), Palette::TEXT_PRIMARY())
            }
            (Tone::Default, button::Status::Pressed) => {
                (Palette::BTN_PRESSED(), Palette::TEXT_PRIMARY())
            }
            (Tone::Default, button::Status::Active) => {
                (Palette::BTN_DEFAULT(), Palette::TEXT_PRIMARY())
            }
            (Tone::Primary, status) => {
                let accent = Palette::ACCENT();
                let background = if status == button::Status::Active {
                    accent
                } else {
                    Color { a: 0.85, ..accent }
                };
                (background, contrast_on(accent))
            }
            (Tone::Danger, button::Status::Active) => (Palette::BTN_DANGER_BG(), Palette::ERROR()),
            (Tone::Danger, _) => (Palette::BTN_DANGER_HOVER(), Palette::ERROR()),
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border::default().rounded(6),
            ..Default::default()
        }
    }
}

/// A button with a glyph and a label, sized to its content. `None` disables it.
pub fn action<'a, M: Clone + 'a>(
    typo: &Typography,
    icon: &'static str,
    caption: &str,
    tone: Tone,
    on_press: Option<M>,
) -> Element<'a, M> {
    // The glyph inherits the button's text colour, so it dims with it.
    let content = row![
        text(icon).size(typo.sz(13)).font(fonts::glyph_font()),
        text(caption.to_string())
            .size(typo.sz(13))
            .font(typo.regular),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    button(content)
        .padding([6, 12])
        .style(button_style(tone))
        .on_press_maybe(on_press)
        .into()
}

/// A button with a label only.
pub fn text_button<'a, M: Clone + 'a>(
    typo: &Typography,
    caption: &str,
    tone: Tone,
    on_press: Option<M>,
) -> Element<'a, M> {
    button(
        text(caption.to_string())
            .size(typo.sz(13))
            .font(typo.regular),
    )
    .padding([6, 12])
    .style(button_style(tone))
    .on_press_maybe(on_press)
    .into()
}

/// A glyph-only button; `hint` names it in a tooltip.
pub fn icon_button<'a, M: Clone + 'a>(
    typo: &Typography,
    icon: &'static str,
    hint: &str,
    on_press: Option<M>,
) -> Element<'a, M> {
    let control = button(text(icon).size(typo.sz(13)).font(fonts::glyph_font()))
        .padding([6, 10])
        .style(button_style(Tone::Default))
        .on_press_maybe(on_press);
    tooltip(control, tip(typo, hint), tooltip::Position::Top)
        .gap(4)
        .into()
}

fn tip<'a, M: 'a>(typo: &Typography, hint: &str) -> Element<'a, M> {
    container(label(typo, hint, 12, Palette::TEXT_PRIMARY()))
        .padding([4, 8])
        .style(|_theme| container::Style {
            background: Some(Palette::BG_MODAL_SECTION().into()),
            border: Border::default()
                .rounded(6)
                .width(1)
                .color(Palette::BORDER_SUBTLE()),
            ..Default::default()
        })
        .into()
}

/// A checkbox whose label follows the typography.
pub fn check<'a, M: Clone + 'a>(
    typo: &Typography,
    caption: &str,
    checked: bool,
    on_toggle: impl Fn(bool) -> M + 'a,
) -> Element<'a, M> {
    iced::widget::checkbox(checked)
        .label(caption.to_string())
        .size(typo.sz(14))
        .text_size(typo.sz(13))
        .font(typo.regular)
        .on_toggle(on_toggle)
        .into()
}

/// A single-line text field whose text follows the typography.
pub fn field<'a, M: Clone + 'a>(
    typo: &Typography,
    placeholder: &str,
    value: &str,
    on_input: impl Fn(String) -> M + 'a,
) -> iced::widget::TextInput<'a, M> {
    iced::widget::text_input(placeholder, value)
        .on_input(on_input)
        .size(typo.sz(13))
        .font(typo.regular)
        .padding([6, 10])
}

/// `dialog` centred over `base`, on a dimmed backdrop that sends
/// `on_dismiss` when clicked.
pub fn modal<'a, M: Clone + 'a>(
    base: impl Into<Element<'a, M>>,
    dialog: impl Into<Element<'a, M>>,
    on_dismiss: M,
) -> Element<'a, M> {
    let backdrop = Color {
        a: 0.75,
        ..Palette::BG_SIDEBAR()
    };
    stack![
        base.into(),
        opaque(
            mouse_area(
                center(opaque(dialog)).style(move |_theme| container::Style {
                    background: Some(backdrop.into()),
                    ..Default::default()
                })
            )
            .on_press(on_dismiss)
        )
    ]
    .into()
}

/// The frame of a dialog: a card at most `max_width` wide, scaled with the
/// text so French and large sizes still fit.
pub fn dialog<'a, M: 'a>(
    typo: &Typography,
    title: &str,
    body: impl Into<Element<'a, M>>,
    max_width: f32,
) -> Element<'a, M> {
    container(column![heading(typo, title, 18), body.into()].spacing(16))
        .padding(20)
        .max_width(max_width * typo.scale)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Palette::BG_PRIMARY().into()),
            border: Border::default()
                .rounded(10)
                .width(1)
                .color(Palette::BORDER_SUBTLE()),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::icons;

    /// A glyph missing from JetBrainsMono Nerd Font renders as tofu.
    #[test]
    fn every_icon_is_in_the_nerd_font() {
        let face = ttf_parser::Face::parse(colony_ui::fonts::BYTES[0], 0).expect("font parses");
        for icon in icons::ALL {
            let ch = icon.chars().next().expect("one character");
            assert!(
                face.glyph_index(ch).is_some(),
                "U+{:04X} is not in the Nerd Font",
                ch as u32
            );
        }
    }
}
