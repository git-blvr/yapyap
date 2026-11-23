//! Settings → Customize's choices, applied over the theme shown: the
//! window's font, its background and the accent colour, kept as
//! `layout.json`'s "customize" and drawn over whatever theme shows, a
//! package's or a palette's. The font's family resolves through the kit's
//! font loaders; the accent derives its hover, soft and on-accent the way
//! the built-in themes write theirs.

use circord_core::theme as palette;
use iced::Color;

use crate::{
    layout::{CustomBackground, Customize},
    theme::{self, Theme},
};

/// `theme` with Customize's colour choices over it: the accent, and the
/// background's base colour as `chat`, so derivations drawn against the
/// ground (danger text, the window's clear colour) keep reading right.
pub fn applied(theme: Theme, customize: &Customize) -> Theme {
    let mut theme = theme;
    let mut changed = false;
    if let Some(accent) = customize.accent.as_deref().and_then(palette::parse) {
        let accent = crate::themes::color(accent);
        theme.accent = accent;
        theme.accent_hover = theme::mix(accent, Color::WHITE, 0.85);
        theme.accent_soft = Color { a: 0.15, ..accent };
        theme.on_accent = on_accent(accent);
        changed = true;
    }
    if let Some(base) = base_colour(customize.background.as_ref()) {
        theme.chat = base;
        changed = true;
    }
    if changed { theme.worked_out() } else { theme }
}

/// The ink readable on `accent`: the nearer of white or black, lifted to
/// 4.5:1 as the themes' `on_accent` is.
fn on_accent(accent: Color) -> Color {
    let start = if theme::contrast(Color::WHITE, accent) >= theme::contrast(Color::BLACK, accent) {
        Color::WHITE
    } else {
        Color::BLACK
    };
    theme::readable_on(start, accent, 4.5)
}

/// The colour `chat` shows as: a solid's, a gradient's middle.
fn base_colour(background: Option<&CustomBackground>) -> Option<Color> {
    match background? {
        CustomBackground::Solid { colour } => palette::parse(colour).map(crate::themes::color),
        CustomBackground::Gradient { from, to, .. } => {
            let from = palette::parse(from).map(crate::themes::color);
            let to = palette::parse(to).map(crate::themes::color);
            match (from, to) {
                (Some(from), Some(to)) => Some(theme::mix(from, to, 0.5)),
                (one, other) => one.or(other),
            }
        }
    }
}

/// What a ground that would fill with `theme.chat` draws while Customize
/// sets a background: its paint, else `theme.chat` itself.
pub fn chat_fill(customize: &Customize, theme: &Theme) -> iced::Background {
    background(customize).unwrap_or_else(|| theme.chat.into())
}

/// The window's whole ground under everything, as Customize set it: a
/// colour or the gradient, drawn under the app's surfaces instead of
/// `chat`'s clear colour.
pub fn background(customize: &Customize) -> Option<iced::Background> {
    match customize.background.as_ref()? {
        CustomBackground::Solid { colour } => {
            palette::parse(colour).map(|colour| crate::themes::color(colour).into())
        }
        CustomBackground::Gradient { from, to, angle } => {
            let (from, to) = (
                palette::parse(from).map(crate::themes::color)?,
                palette::parse(to).map(crate::themes::color)?,
            );
            Some(
                iced::gradient::Linear::new(iced::Radians(angle.to_radians()))
                    .add_stop(0.0, from)
                    .add_stop(1.0, to)
                    .into(),
            )
        }
    }
}

/// The family the Customize page's font `choice` names, loaded; "" and
/// unknown choices keep the app's own.
pub fn font_family(choice: &str) -> Option<&'static str> {
    crate::fonts::custom_family(choice)
}
