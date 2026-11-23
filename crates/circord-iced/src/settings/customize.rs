//! Settings → Customize: the window's font, its background and the accent
//! colour, over whatever theme shows, kept and shown at once. The choices
//! live as `layout.json`'s "customize"; how they draw is `crate::customize`,
//! and a theme's own performance and motion settings sit beside them.

use circord_core::{theme as palette, themes::Performance};
use iced::{
    Border, Color, Element,
    widget::{Space, button, column, row, text},
};

use super::{Drop, Event as SettingsEvent, Page, Settings};
use crate::{
    app::Message,
    controls::{self, Kind, action, lights},
    ctx::{Cx, Pref as Shown, Request},
    form::{self, switch},
    layout::{CustomBackground, Customize},
    themes::color,
};

/// The fonts offered: their labels, and the keys `customize.font` keeps,
/// as [`crate::fonts::custom_family`] reads them.
const FONTS: &[(&str, &str)] = &[
    ("Circord's own", ""),
    ("Atkinson Hyperlegible", "atkinson"),
    ("System font", "system"),
    ("Monospace", "mono"),
    ("8-Bit", "display:8Bit"),
    ("Headbang", "display:Headbang"),
    ("Jellybean", "display:Jellybean"),
    ("Journal", "display:Journal"),
    ("Mainframe", "display:Mainframe"),
    ("Medieval", "display:Néo-castel"),
    ("Modern", "display:Modern"),
    ("Monkey Bars", "display:Monkey Bars"),
    ("Sakura", "display:Sakura"),
    ("Tempo", "display:Tempo"),
    ("Vampyre", "display:Sinistre"),
    ("A font file…", "file"),
];

/// The colours a tap sets, for the accent and the background.
const SWATCHES: &[&str] = &[
    "#ff9046", "#5865f2", "#57f287", "#eb459e", "#f2bb13", "#f26522", "#e3e5e8", "#1e1f22",
];

/// The Customize page's field texts while it has the window: a colour the
/// person is still typing keeps its letters, wrong or not, without
/// changing what's kept. The page fills it from `layout.json`'s choices
/// as it opens.
#[derive(Clone, Default)]
pub struct Draft {
    /// The accent colour's text.
    pub accent: String,
    /// The solid background's colour text.
    pub solid: String,
    /// The gradient's two colour texts.
    pub from: String,
    pub to: String,
    /// The gradient's angle, in degrees.
    pub angle: u32,
    /// The font file's path, while "A font file…" is the font.
    pub font_path: String,
}

impl Draft {
    /// The fields as the kept choices start them: unset ones take the
    /// theme showing's colours, so its own colours are the first a change
    /// moves from.
    pub fn of(customize: &Customize, theme: &crate::theme::Theme) -> Self {
        let (mut solid, mut from, mut to, mut angle) = (
            hex_of(theme.chat),
            hex_of(theme.chat),
            hex_of(theme.accent),
            135,
        );
        match customize.background.as_ref() {
            Some(CustomBackground::Solid { colour }) => solid.clone_from(colour),
            Some(CustomBackground::Gradient {
                from: first,
                to: second,
                angle: degrees,
            }) => {
                from.clone_from(first);
                to.clone_from(second);
                angle = (*degrees as u32).min(360);
            }
            None => {}
        }
        Self {
            accent: customize
                .accent
                .clone()
                .unwrap_or_else(|| hex_of(theme.accent)),
            solid,
            from,
            to,
            angle,
            font_path: customize
                .font
                .strip_prefix("file:")
                .unwrap_or_default()
                .to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    /// A font from the list, by its row in [`FONTS`].
    Font(usize),
    /// The font file's path, while "A font file…" is the font.
    FontPath(String),
    /// The background's kind: 0 the theme's, 1 a colour, 2 a gradient.
    Background(usize),
    /// A background colour's text: 0 the solid's, 1 and 2 the gradient's.
    Colour(usize, String),
    /// The gradient's angle, in degrees.
    Angle(u32),
    /// The accent colour's text; emptied, the theme's own accent.
    Accent(String),
    /// The accent goes back to the theme's.
    AccentReset,
    /// How much of an installed theme runs.
    Performance(i32),
    ReduceMotion(bool),
    ThemeTransitions(bool),
}

pub(super) fn customize_message(event: Event) -> Message {
    crate::app::settings(SettingsEvent::Customize(event))
}

impl Settings {
    /// One Customize change: the field's text goes in the draft, and what
    /// it makes — only a colour that parses — is kept and shown at once.
    pub(super) fn customize_event(&mut self, event: Event, cx: &mut Cx<'_>) -> iced::Task<Message> {
        let draft = self
            .customize_draft
            .get_or_insert_with(|| Draft::of(&cx.layout.customize, cx.theme));
        match event {
            Event::Font(index) => {
                let key = FONTS.get(index).map(|(_, key)| *key).unwrap_or_default();
                cx.layout.customize.font = match key {
                    "file" => format!("file:{}", draft.font_path.trim()),
                    _ => key.to_owned(),
                };
                applied(cx);
            }
            Event::FontPath(path) => {
                draft.font_path = path;
                if cx.layout.customize.font.starts_with("file:") {
                    cx.layout.customize.font = format!("file:{}", draft.font_path.trim());
                    applied(cx);
                }
            }
            Event::Background(index) => {
                cx.layout.customize.background = match index {
                    1 => Some(CustomBackground::Solid {
                        colour: colour_or(&draft.solid, cx.theme.chat),
                    }),
                    2 => Some(CustomBackground::Gradient {
                        from: colour_or(&draft.from, cx.theme.chat),
                        to: colour_or(&draft.to, cx.theme.accent),
                        angle: draft.angle as f32,
                    }),
                    _ => None,
                };
                applied(cx);
            }
            Event::Colour(which, text) => {
                let slot = match which {
                    1 => &mut draft.from,
                    2 => &mut draft.to,
                    _ => &mut draft.solid,
                };
                slot.clone_from(&text);
                if palette::parse(&text).is_none() {
                    return iced::Task::none();
                }
                match &mut cx.layout.customize.background {
                    Some(CustomBackground::Solid { colour }) if which == 0 => *colour = text,
                    Some(CustomBackground::Gradient { from, .. }) if which == 1 => *from = text,
                    Some(CustomBackground::Gradient { to, .. }) if which == 2 => *to = text,
                    _ => return iced::Task::none(),
                }
                applied(cx);
            }
            Event::Angle(angle) => {
                draft.angle = angle;
                if let Some(CustomBackground::Gradient { angle: kept, .. }) =
                    &mut cx.layout.customize.background
                {
                    *kept = angle as f32;
                    applied(cx);
                }
            }
            Event::Accent(text) => {
                draft.accent.clone_from(&text);
                if text.trim().is_empty() {
                    cx.layout.customize.accent = None;
                    applied(cx);
                } else if palette::parse(&text).is_some() {
                    cx.layout.customize.accent = Some(text);
                    applied(cx);
                }
            }
            Event::AccentReset => {
                draft.accent = hex_of(cx.theme.accent);
                cx.layout.customize.accent = None;
                applied(cx);
            }
            Event::Performance(index) => {
                let performance = Performance::from_index(index);
                cx.layout.theme_performance = performance;
                cx.keep("theme_performance", performance);
                cx.themes
                    .motion_changed(performance, cx.layout.reduce_motion);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::ReduceMotion(on) => {
                cx.layout.reduce_motion = on;
                cx.keep("reduce_motion", on);
                cx.themes.motion_changed(cx.layout.theme_performance, on);
                cx.request(Request::Changed(Shown::Theme));
            }
            Event::ThemeTransitions(on) => {
                cx.layout.theme_transitions = on;
                cx.keep("theme_transitions", on);
            }
        }
        iced::Task::none()
    }
}

/// Keeps Customize's choices and shows them again, over the theme.
fn applied(cx: &mut Cx<'_>) {
    cx.keep("customize", &cx.layout.customize);
    cx.request(Request::Changed(Shown::Theme));
}

/// `text` as a colour for keeping, else `fallback`'s: the text the page
/// starts a choice with is the theme's own colour.
fn colour_or(text: &str, fallback: Color) -> String {
    if palette::parse(text).is_some() {
        text.to_owned()
    } else {
        hex_of(fallback)
    }
}

/// `colour` as `#rrggbb`, for a field to start from the theme's.
fn hex_of(colour: Color) -> String {
    palette::hex(palette::Colour::rgba(
        ((colour.r * 255.0).round() as u32) << 24
            | ((colour.g * 255.0).round() as u32) << 16
            | ((colour.b * 255.0).round() as u32) << 8
            | ((colour.a * 255.0).round() as u32),
    ))
}

impl<'a> Page<'a> {
    /// The page: font, background, accent, then a theme's performance and
    /// motion, moved here from the page Customize replaces.
    pub(super) fn customize_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let customize = &self.cx.layout.customize;
        let draft = self
            .s
            .customize_draft
            .clone()
            .unwrap_or_else(|| Draft::of(customize, theme));
        form::page([
            form::title(theme, "Customize"),
            form::note(
                theme,
                "Your font, your colours: these sit over the theme you choose in Appearance, and are kept at once.",
            ),
            self.font_group(&customize.font, &draft),
            self.background_group(customize.background.as_ref(), &draft),
            self.accent_group(customize.accent.is_some(), &draft.accent),
            self.performance_group(),
        ])
    }

    /// FONT: the list of faces, and the file's path while "A font file…"
    /// is chosen.
    fn font_group(self, font: &str, draft: &Draft) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let chosen = FONTS
            .iter()
            .position(|(_, key)| *key == font)
            .or_else(|| font.starts_with("file:").then_some(FONTS.len() - 1));
        let names: Vec<String> = FONTS.iter().map(|(name, _)| (*name).to_owned()).collect();
        let mut group = column![
            form::heading(theme, "FONT"),
            form::dropdown(
                names,
                chosen,
                self.s.open == Some(Drop::Font),
                crate::app::settings(SettingsEvent::Drop(Drop::Font)),
                |index| customize_message(Event::Font(index)),
            )
            .view(self.cx.look),
            form::note(
                theme,
                "Everything Circord draws uses it, from the channel list to what you type. Display fonts are made for short texts.",
            ),
        ]
        .spacing(12);
        if font.starts_with("file:") {
            group = group.push(
                controls::field(
                    theme,
                    "customize-font-path",
                    "The font file's path (.ttf or .otf)",
                    &draft.font_path,
                    false,
                    true,
                )
                .size(14)
                .on_input(|path| customize_message(Event::FontPath(path))),
            );
        }
        group.into()
    }

    /// BACKGROUND: the theme's own, one colour, or a two-colour gradient
    /// with its angle, each colour as a field under a row of readier ones.
    fn background_group(
        self,
        background: Option<&CustomBackground>,
        draft: &Draft,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let mode = match background {
            Some(CustomBackground::Solid { .. }) => 1,
            Some(CustomBackground::Gradient { .. }) => 2,
            None => 0,
        };
        let mut group = column![
            form::heading(theme, "BACKGROUND"),
            controls::choices(
                theme,
                &["The theme's", "A colour", "A gradient"],
                mode,
                |index| customize_message(Event::Background(index)),
            ),
        ]
        .spacing(12);
        match mode {
            1 => {
                group = group.push(self.colour_part(
                    "Colour",
                    "customize-solid",
                    "#1e1f22",
                    &draft.solid,
                    |text| customize_message(Event::Colour(0, text)),
                ));
            }
            2 => {
                group = group
                    .push(self.colour_part(
                        "From",
                        "customize-from",
                        "#1e1f22",
                        &draft.from,
                        |text| customize_message(Event::Colour(1, text)),
                    ))
                    .push(
                        self.colour_part("To", "customize-to", "#ff9046", &draft.to, |text| {
                            customize_message(Event::Colour(2, text))
                        }),
                    )
                    .push(
                        controls::slider("Angle", draft.angle, 360, move |angle| {
                            customize_message(Event::Angle(angle))
                        })
                        .step(15)
                        .shown(format!("{}°", draft.angle))
                        .mark(false)
                        .view(theme),
                    );
            }
            _ => {}
        }
        group.into()
    }

    /// ACCENT: its colour as a field under the readier ones, and back to
    /// the theme's own.
    fn accent_group(self, over_theme: bool, accent: &str) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let reset: Element<'a, Message> = if over_theme {
            action(
                theme,
                "The theme's own",
                Kind::Plain,
                Some(customize_message(Event::AccentReset)),
            )
            .small()
            .into()
        } else {
            Space::new().height(0).into()
        };
        column![
            column![
                form::beside(form::heading(theme, "ACCENT"), reset),
                form::note(
                    theme,
                    "Buttons, links and highlights take it; what's written on it stays readable.",
                ),
            ]
            .spacing(8),
            self.colour_part("Colour", "customize-accent", "#ff9046", accent, |text| {
                customize_message(Event::Accent(text))
            }),
        ]
        .spacing(12)
        .into()
    }

    /// PERFORMANCE and MOTION, as the themes page had them.
    fn performance_group(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let performance = self.cx.layout.theme_performance.index() as usize;
        let said = match performance {
            0 => "Everything an installed theme has: every animation, at the rate it asks for.",
            1 => {
                "Heavy animations stay still, the rest run at up to 30 frames a second and pause while Circord isn't focused."
            }
            _ => "Colours, fonts, shapes and spacing only: no theme animation, effect or motion.",
        };
        column![
            form::rule(theme),
            column![
                form::heading(theme, "PERFORMANCE"),
                controls::choices(
                    theme,
                    &["Full", "Balanced", "Light"],
                    performance,
                    |index| customize_message(Event::Performance(index as i32)),
                ),
                form::note(theme, said),
            ]
            .spacing(12),
            column![
                form::heading(theme, "MOTION"),
                switch(
                    "Reduce motion",
                    self.cx.layout.reduce_motion,
                    Some(customize_message(Event::ReduceMotion(
                        !self.cx.layout.reduce_motion,
                    ))),
                )
                .description(
                    "Themes' animations, text effects and hover motion stay still, whatever Performance is set to.",
                )
                .view(self.cx.look),
                switch(
                    "Transitions between channels",
                    self.cx.layout.theme_transitions,
                    Some(customize_message(Event::ThemeTransitions(
                        !self.cx.layout.theme_transitions,
                    ))),
                )
                .description(
                    "A theme's fade or dissolve when you change channel or server, at Full. Off, a channel shows at once.",
                )
                .view(self.cx.look),
            ]
            .spacing(12),
        ]
        .spacing(16)
        .into()
    }

    /// A colour's label, field and readier colours: the field shows what
    /// the person typed, ringed in danger while it isn't a colour.
    fn colour_part(
        self,
        label: &'a str,
        id: &'static str,
        placeholder: &'static str,
        value: &str,
        on_input: impl Fn(String) -> Message + Copy + 'a,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let wrong = !value.is_empty() && palette::parse(value).is_none();
        column![
            column![
                text(label).size(15).color(theme.text),
                colour_field(theme, id, placeholder, value, wrong).on_input(on_input),
            ]
            .spacing(6),
            swatches(theme, move |hex| on_input(hex.to_owned())),
        ]
        .spacing(8)
        .into()
    }
}

/// A colour's field, ringed in danger while `wrong`.
fn colour_field<'a>(
    theme: &'a crate::theme::Theme,
    id: &'static str,
    placeholder: &'static str,
    value: &str,
    wrong: bool,
) -> iced::widget::TextInput<'a, Message> {
    let normal = controls::field_style(theme, theme.sunken, true);
    let danger = theme.danger;
    controls::field(theme, id, placeholder, value, false, true)
        .size(14)
        .style(move |t, status| {
            let mut style = normal(t, status);
            if wrong {
                style.border.color = danger;
                style.border.width = 1.0;
            }
            style
        })
}

/// The readier colours, as chips a tap puts in the field.
fn swatches<'a>(
    theme: &'a crate::theme::Theme,
    pick: impl Fn(&'static str) -> Message + 'a,
) -> Element<'a, Message> {
    let mut row = row![].spacing(6);
    for hex in SWATCHES {
        let Some(colour) = palette::parse(hex).map(color) else {
            continue;
        };
        let (line, ink) = (theme.line, theme.text);
        row = row.push(
            button(Space::new())
                .width(22)
                .height(22)
                .on_press(pick(hex))
                .style(move |_, status| {
                    let lit = lights(status);
                    button::Style {
                        background: Some(colour.into()),
                        border: Border {
                            color: if lit { ink } else { line },
                            width: if lit { 2.0 } else { 1.0 },
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    }
                }),
        );
    }
    row.into()
}
