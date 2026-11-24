//! Appearance: the themes to choose as tiles, the chosen theme's own
//! options, readable text, the title bar and the profile switches. What
//! the person sets of the window's own look — its font, background and
//! accent — is Settings → Customize. See docs/themes.md.

use circord_core::{
    application::Action,
    theme::{self as palette},
    themes::options,
};
use iced::{
    Alignment, Border, Color, Element, Length,
    widget::{Space, button, column, container, row, stack, text},
};

use super::{Event as SettingsEvent, Page, Settings};
use crate::{
    app::Message,
    controls::{self, lights, placed, slider},
    ctx::{Cx, Pref as Shown, Request},
    fonts::{BOLD, MEDIUM, REGULAR},
    form::{self, switch},
    pictures::Icon,
    themes::{Installed, color},
};

/// A theme's tile: at least this wide, this tall, this far apart.
const TILE: f32 = 150.0;
const TILE_HEIGHT: f32 = 124.0;
const TILE_GAP: f32 = 12.0;

#[derive(Clone, Debug)]
pub enum Event {
    Choose(String),
    /// The chosen package's own option, as the theme's author named it.
    ThemeOption(String, String, serde_json::Value),
    ReadableText(bool),
    /// A person's theme's bin: the first press arms it, the second deletes.
    Delete(String),
    Profile(Cosmetic, bool),
}

/// Appearance's profile switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cosmetic {
    Effects,
    Decorations,
    Nameplates,
    Moving,
    NameStyles,
    ServerTags,
    Frames,
}

pub(super) fn theme_message(event: Event) -> Message {
    crate::app::settings(SettingsEvent::Appearance(event))
}

impl Settings {
    pub(super) fn appearance_event(
        &mut self,
        event: Event,
        cx: &mut Cx<'_>,
    ) -> iced::Task<Message> {
        // Anything but a bin press takes the arm back off.
        if !matches!(event, Event::Delete(_)) {
            self.delete_armed = None;
        }
        match event {
            Event::Choose(id) => choose(cx, &id),
            Event::ThemeOption(id, option, value) => {
                let Some(choices) = cx.themes.set_option(
                    &id,
                    &option,
                    value,
                    cx.layout.theme_performance,
                    cx.layout.reduce_motion,
                ) else {
                    return iced::Task::none();
                };
                if choices.is_empty() {
                    cx.layout.theme_options.remove(&id);
                } else {
                    cx.layout.theme_options.insert(id, choices);
                }
                cx.keep("theme_options", &cx.layout.theme_options);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::ReadableText(on) => {
                cx.layout.readable_text = on;
                cx.keep("readable_text", on);
                cx.themes.readable_changed(on);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::Delete(id) => {
                if self.delete_armed.as_deref() != Some(id.as_str()) {
                    self.delete_armed = Some(id);
                    return iced::Task::none();
                }
                self.delete_armed = None;
                if cx.themes.forget_custom(cx.client.root(), &id) {
                    cx.layout.custom_themes.clone_from(&cx.themes.custom);
                    cx.keep("custom_themes", &cx.themes.custom);
                    cx.request(Request::Changed(Shown::Theme));
                }
            }
            Event::Profile(cosmetic, on) => self::cosmetic(cx, cosmetic, on),
        }
        iced::Task::none()
    }
}

/// Chooses the theme `id` and keeps the choice, if there's such a theme:
/// the window shows it.
fn choose(cx: &mut Cx<'_>, id: &str) {
    let Some(theme) = cx.themes.find(id) else {
        return;
    };
    cx.themes.choose(cx.client.root(), &theme.id);
    cx.request(Request::ThemeApplied);
}

/// A profile switch: kept, and the core told what may now show.
fn cosmetic(cx: &mut Cx<'_>, cosmetic: Cosmetic, on: bool) {
    let prefs = &mut cx.layout.prefs;
    let (slot, key) = match cosmetic {
        Cosmetic::Effects => (&mut prefs.profile_effects, "profile_effects"),
        Cosmetic::Decorations => (&mut prefs.decorations, "decorations"),
        Cosmetic::Nameplates => (&mut prefs.nameplates, "nameplates"),
        Cosmetic::Moving => (&mut prefs.moving_cosmetics, "moving_cosmetics"),
        Cosmetic::NameStyles => (&mut prefs.name_styles, "name_styles"),
        Cosmetic::ServerTags => (&mut prefs.server_tags, "server_tags"),
        Cosmetic::Frames => (&mut prefs.profile_frames, "profile_frames"),
    };
    *slot = Some(on);
    cx.keep(key, on);
    if cosmetic == Cosmetic::Nameplates {
        cx.dispatch(Action::HideNameplates(!on));
    } else {
        let cosmetics = cx.layout.prefs.cosmetics();
        cx.dispatch(Action::SetCosmetics(cosmetics));
    }
    cx.request(Request::Changed(Shown::Roster));
}

impl<'a> Page<'a> {
    pub(super) fn appearance_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let tiles = self.theme_tiles(true);
        let prefs = &self.cx.layout.prefs;
        let cosmetics = prefs.cosmetics();
        let profile = |label, description, on: bool, which| {
            switch(label, on, Some(theme_message(Event::Profile(which, !on))))
                .description(description)
                .view(self.cx.look)
        };
        let theme_options = self
            .cx
            .themes
            .installed
            .iter()
            .find(|installed| self.cx.themes.chosen_id() == installed.id)
            .and_then(|installed| self.theme_options(installed))
            .unwrap_or_else(|| Space::new().height(0).into());
        form::page([
            form::title(theme, "Appearance"),
            form::group([form::heading(theme, "THEME"), tiles]),
            theme_options,
            switch(
                "Readable text",
                self.cx.layout.readable_text,
                Some(theme_message(Event::ReadableText(
                    !self.cx.layout.readable_text,
                ))),
            )
            .description(
                "Messages, names, channels and what you type use Circord's own font in every theme. The theme's art, colours and layout stay.",
            )
            .view(self.cx.look),
            self.title_bar_switch(),
            column![
                form::heading(theme, "PROFILES"),
                profile(
                    "Play profile effects",
                    "The animations some people put over their profile cards.",
                    cosmetics.effects,
                    Cosmetic::Effects,
                ),
                profile(
                    "Show avatar decorations",
                    "The pictures around people's avatars.",
                    cosmetics.decorations,
                    Cosmetic::Decorations,
                ),
                profile(
                    "Show nameplates",
                    "The pictures Discord members equip behind their rows in the member list and on your account card.",
                    prefs.nameplates.unwrap_or(true),
                    Cosmetic::Nameplates,
                ),
                profile(
                    "Animate decorations and nameplates",
                    "They play when pointed at, as in Discord. Off, they hold still.",
                    cosmetics.moving,
                    Cosmetic::Moving,
                ),
                profile(
                    "Show name styles",
                    "The fonts and colours people give their display names.",
                    cosmetics.name_styles,
                    Cosmetic::NameStyles,
                ),
                profile(
                    "Show server tags",
                    "The tag and its badge beside people's names.",
                    cosmetics.server_tags,
                    Cosmetic::ServerTags,
                ),
                profile(
                    "Show profile frames",
                    "The frames some people put around their profile cards.",
                    cosmetics.frames,
                    Cosmetic::Frames,
                ),
            ]
            .spacing(12)
            .into(),
        ])
    }

    /// The themes as tiles, as many 150 px or wider across as fit.
    fn theme_tiles(self, include_built_in: bool) -> Element<'a, Message> {
        iced::widget::responsive(move |size| {
            let columns = ((size.width + TILE_GAP) / (TILE + TILE_GAP))
                .floor()
                .max(1.0) as usize;
            let width = (size.width - (columns - 1) as f32 * TILE_GAP) / columns as f32;
            let built_in = if include_built_in {
                palette::BUILT_IN.as_slice()
            } else {
                &[]
            };
            let cards: Vec<(String, String, bool, palette::Theme)> =
                built_in
                    .iter()
                    .map(|built_in| {
                        (
                            built_in.id.to_owned(),
                            built_in.name.to_owned(),
                            false,
                            built_in.theme,
                        )
                    })
                    .chain(
                        self.cx.themes.custom.iter().map(|custom| {
                            (custom.id.clone(), custom.name.clone(), true, custom.theme)
                        }),
                    )
                    .collect();
            let mut rows = column![].spacing(TILE_GAP);
            for chunk in cards.chunks(columns) {
                let mut line = row![].spacing(TILE_GAP);
                for (id, name, custom, palette) in chunk {
                    line = line.push(self.theme_tile(id, name, *custom, palette, width));
                }
                rows = rows.push(line);
            }
            rows.into()
        })
        .height(Length::Shrink)
        .into()
    }

    /// A theme to choose: a small window in its colours, its name under it,
    /// an accent ring and a tick on the one shown, and a bin on the
    /// person's own.
    fn theme_tile(
        self,
        id: &str,
        name: &str,
        custom: bool,
        palette: &palette::Theme,
        width: f32,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let current = self.cx.themes.chosen_id() == id;
        let preview = preview(palette, width - 16.0);
        let mut label = row![
            text(name.to_owned())
                .size(14)
                .font(if current { BOLD } else { MEDIUM })
                .color(if current { theme.text } else { theme.secondary })
                .wrapping(text::Wrapping::None)
                .width(Length::Fill)
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .height(24);
        if current {
            label = label.push(self.cx.look.glyph(Icon::Check, 16, theme.accent));
        }
        let (accent, faint, line, hover) = (theme.accent, theme.faint, theme.line, theme.hover);
        let face = button(
            column![preview, container(label).padding([0, 2])]
                .spacing(6)
                .padding(8),
        )
        .padding(0)
        .width(width)
        .height(TILE_HEIGHT)
        .on_press(theme_message(Event::Choose(id.to_owned())))
        .style(move |_, status| {
            let lit = lights(status);
            button::Style {
                background: lit.then(|| hover.into()),
                border: Border {
                    color: if current {
                        accent
                    } else if lit {
                        faint
                    } else {
                        line
                    },
                    width: if current { 2.0 } else { 1.0 },
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        });
        if !custom {
            return face.into();
        }
        // The bin arms on the first press and deletes on the second, as the
        // menu the editor's Delete was.
        let armed = self.s.delete_armed.as_deref() == Some(id);
        let bin = self
            .cx
            .look
            .icon_button(
                Icon::Trash,
                if armed { "Delete it" } else { "Delete" },
                theme_message(Event::Delete(id.to_owned())),
            )
            .size(24.0, 14);
        stack![face, placed(bin, width - 8.0 - 24.0, 8.0 + 78.0 + 6.0)].into()
    }

    /// THEME OPTIONS: the applied package's switches, choices and sliders.
    fn theme_options(self, installed: &'a Installed) -> Option<Element<'a, Message>> {
        let theme = self.cx.look.theme;
        let mut rows = column![form::heading(theme, "THEME OPTIONS")].spacing(8);
        let mut any = false;
        for option in &installed.options {
            if !option.active(&installed.options, &installed.choices) {
                continue;
            }
            any = true;
            let value = option.value(&installed.choices);
            let id = installed.id.clone();
            let option_id = option.id.clone();
            let message = move |value| {
                theme_message(Event::ThemeOption(id.clone(), option_id.clone(), value))
            };
            let note = (!option.description.is_empty())
                .then(|| form::note(theme, option.description.clone()));
            let control: Element<'a, Message> = match &option.kind {
                options::Kind::Toggle { .. } => {
                    let on = value.as_bool().unwrap_or(false);
                    switch(
                        option.label.clone(),
                        on,
                        Some(message(serde_json::Value::Bool(!on))),
                    )
                    .description(option.description.clone())
                    .view(self.cx.look)
                }
                options::Kind::Choice { choices, .. } => {
                    let current = value
                        .as_str()
                        .and_then(|value| choices.iter().position(|choice| choice.value == value))
                        .unwrap_or(usize::MAX);
                    let labels: Vec<&'a str> =
                        choices.iter().map(|choice| choice.label.as_str()).collect();
                    let values: Vec<String> =
                        choices.iter().map(|choice| choice.value.clone()).collect();
                    let widest = (labels.len() as f32 * 120.0).max(360.0);
                    let mut parts = column![
                        text(option.label.clone())
                            .size(15)
                            .font(REGULAR)
                            .color(theme.text),
                        controls::choices_within(theme, &labels, current, widest, move |index| {
                            message(serde_json::Value::String(values[index].clone()))
                        }),
                    ]
                    .spacing(6);
                    if let Some(note) = note {
                        parts = parts.push(note);
                    }
                    parts.into()
                }
                options::Kind::Slider { min, max, step, .. } => {
                    option_slider(theme, option, &value, *min, *max, *step, message, note)
                }
                options::Kind::Hue { .. } => {
                    option_slider(theme, option, &value, 0.0, 360.0, 5.0, message, note)
                }
            };
            rows = rows.push(control);
        }
        any.then(|| rows.into())
    }

    /// Appearance's "Use the system title bar", where Circord can draw its
    /// own: the desktop's way until it's chosen.
    pub(super) fn title_bar_switch(self) -> Element<'a, Message> {
        if !crate::window_bar::offered() {
            return Space::new().into();
        }
        let system = !self.cx.own_title_bar;
        switch(
            "Use the system title bar",
            system,
            Some(crate::app::settings(SettingsEvent::Pref(
                super::pages::Pref::SystemTitleBar(!system),
            ))),
        )
        .description("Your desktop draws the window's title bar and buttons, as on its other windows. Off, Circord draws its own.")
        .view(self.cx.look)
    }
}

/// An option's number as its value.
fn number(value: f64) -> serde_json::Value {
    serde_json::Number::from_f64(value)
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
}

/// A package option's slider: its label, value and steps, as the themes
/// page drew it.
#[allow(
    clippy::too_many_arguments,
    reason = "each part it draws is its own input"
)]
fn option_slider<'a>(
    theme: &crate::theme::Theme,
    option: &options::ThemeOption,
    value: &serde_json::Value,
    min: f64,
    max: f64,
    step: f64,
    message: impl Fn(serde_json::Value) -> Message + 'a,
    note: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let steps = ((max - min) / step).round().max(1.0) as u32;
    let chosen = value.as_f64().unwrap_or(min).clamp(min, max);
    let at = ((chosen - min) / step).round().clamp(0.0, f64::from(steps)) as u32;
    let mut control = column![
        slider(option.label.clone(), at, steps, move |at| {
            message(number(min + f64::from(at) * step))
        })
        .step(1)
        .shown(option.describe(value))
        .mark(false)
        .view(theme)
    ]
    .spacing(6);
    if let Some(note) = note {
        control = control.push(note);
    }
    control.into()
}

/// A theme's small window, 78 px tall: the rail with three servers, the
/// sidebar's channels, two messages and the composer, in its colours.
fn preview<'a>(palette: &palette::Theme, width: f32) -> Element<'a, Message> {
    let c = |colour| color(colour);
    let (rail, sidebar, chat, raised, accent, ink, muted) = (
        c(palette.rail),
        c(palette.sidebar),
        c(palette.chat),
        c(palette.raised),
        c(palette.accent),
        c(palette.text),
        c(palette.muted),
    );
    let blob = |w: f32, h: f32, colour: Color, radius: f32| -> Element<'a, Message> {
        container(Space::new())
            .width(w)
            .height(h)
            .style(move |_| crate::style::rounded(colour, radius))
            .into()
    };
    let mut layers = stack![blob(width, 78.0, chat, 6.0)];
    layers = layers.push(blob(16.0, 78.0, rail, 0.0));
    for index in 0..3 {
        let (colour, radius) = if index == 0 {
            (accent, 3.0)
        } else {
            (raised, 5.0)
        };
        layers = layers.push(placed(
            blob(10.0, 10.0, colour, radius),
            3.0,
            6.0 + index as f32 * 13.0,
        ));
    }
    layers = layers.push(placed(blob(38.0, 78.0, sidebar, 0.0), 16.0, 0.0));
    for index in 0..4 {
        let w = if index == 1 {
            24.0
        } else {
            18.0 + (index % 2) as f32 * 4.0
        };
        let colour = if index == 1 { ink } else { muted };
        layers = layers.push(placed(
            blob(w, 4.0, colour, 2.0),
            22.0,
            8.0 + index as f32 * 11.0,
        ));
    }
    let room = width - 66.0;
    for index in 0..2 {
        let y = 8.0 + index as f32 * 22.0;
        let first = index == 0;
        layers = layers
            .push(placed(
                blob(10.0, 10.0, if first { accent } else { muted }, 5.0),
                60.0,
                y,
            ))
            .push(placed(
                blob(if first { 22.0 } else { 16.0 }, 4.0, ink, 2.0),
                74.0,
                y + 1.0,
            ))
            .push(placed(
                blob(
                    (room - 14.0).min(if first { 52.0 } else { 40.0 }),
                    3.0,
                    muted,
                    1.5,
                ),
                74.0,
                y + 8.0,
            ));
    }
    layers = layers
        .push(placed(blob(room, 12.0, raised, 4.0), 60.0, 78.0 - 18.0))
        .push(placed(
            blob(6.0, 6.0, accent, 3.0),
            60.0 + room - 10.0,
            78.0 - 18.0,
        ));
    container(layers.width(width).height(78))
        .clip(true)
        .style(|_| iced::widget::container::Style {
            border: crate::style::round(6),
            ..Default::default()
        })
        .into()
}
