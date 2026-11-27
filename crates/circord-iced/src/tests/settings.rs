//! Settings: Appearance offers theme shortcuts, Themes manages installed and
//! custom themes, preferences are kept across a restart, a device is chosen
//! in Voice & video, and push to talk's key is recorded and kept.


use circord_core::voice;
use iced::keyboard::{Key, Location, key};

use super::{change, interact, local, preview, shows, start};
use crate::{
    app::{
        App, Message,
        layers::{Layer, Popup},
    },
    settings::{Event, Section, voice::Event as VoiceEvent},
};

/// Settings on a section, in a window tall enough to show its whole page.
fn open(app: &mut App, section: Section) {
    app.shell.frame.height = 2400.0;
    crate::window_bar::fit_window(app);
    let _ = app.update(crate::app::settings(Event::Open(section)));
}

fn stored(folder: &tempfile::TempDir) -> serde_json::Value {
    serde_json::from_slice(&{
        crate::layout::flush(folder.path());
        std::fs::read(folder.path().join("layout.json")).expect("the layout")
    })
    .expect("JSON")
}

/// Settings open moving, once: their motion starts as they open, stays
/// through what's done in them, and turns to closing as they close, the
/// Settings closed kept, taking nothing, until they've been drawn going.
#[test]
fn settings_open_and_close_as_a_popup() {
    let (mut app, _folder) = preview();
    assert!(app.popups_moving().is_empty());
    open(&mut app, Section::Account);
    let opening = app.popups_moving();
    assert_eq!(opening.len(), 1, "Settings' motion");
    assert!(!opening[0].1, "opening");
    open(&mut app, Section::Appearance);
    assert_eq!(
        app.popups_moving(),
        opening,
        "the same motion, not a new one"
    );
    let _ = app.update(crate::app::settings(Event::Close));
    assert!(app.win.settings.get().is_none(), "closed");
    assert!(app.win.settings.leaving().is_some(), "drawn going");
    assert_eq!(app.popups_moving(), [(opening[0].0, true)], "closing");
    // Escape is the window's again while they go.
    assert!(app.layers().all(|layer| layer != Layer::Settings));
    let _ = app.update(Message::PopupGone(Popup::Layer(Layer::Settings)));
    assert!(app.win.settings.leaving().is_none(), "gone");
    assert!(app.popups_moving().is_empty());
}

/// Settings opened again as they close are the new ones, opening: the
/// ones going go at once.
#[test]
fn settings_opened_again_as_they_close_open_anew() {
    let (mut app, _folder) = preview();
    open(&mut app, Section::Account);
    let _ = app.update(crate::app::settings(Event::Close));
    open(&mut app, Section::Appearance);
    assert!(app.win.settings.leaving().is_none());
    let moving = app.popups_moving();
    assert_eq!(moving.len(), 1);
    assert!(!moving[0].1, "opening");
}

#[test]
fn a_theme_chosen_in_appearance_shows_at_once_and_is_kept() {
    let (mut app, folder) = preview();
    open(&mut app, Section::Account);
    assert!(shows(&app, "My account"));
    interact(&mut app, |ui| {
        ui.click("Appearance").expect("the section");
    });
    assert!(shows(&app, "Catppuccin Latte"), "the themes' tiles");
    interact(&mut app, |ui| {
        ui.click("Nord").expect("its tile");
    });
    assert_eq!(app.theme.id, "nord", "at once");
    assert_eq!(stored(&folder)["theme"], "nord");
    // Escape closes Settings and lets go of what it held.
    let _ = app.update(Message::Key(
        Key::Named(key::Named::Escape),
        Default::default(),
    ));
    assert!(!app.win.settings.is_open());
    assert_eq!(start(&folder, true).theme.id, "nord", "the next start too");
}

fn ctrl_shift_t(app: &mut App) {
    let _ = app.update(Message::Key(
        Key::Character("t".into()),
        iced::keyboard::Modifiers::CTRL | iced::keyboard::Modifiers::SHIFT,
    ));
}

/// A small package installed in `folder`, as `installed:dev.example.small`.
fn install_small_package(folder: &tempfile::TempDir) {
    let fetched = circord_core::themes::install::Fetched::new(
        "https://github.com/ada/small".into(),
        String::new(),
        "main".into(),
        "0123456789abcdef0123456789abcdef01234567".into(),
        circord_core::themes::check::Files {
            manifest: r##"{ "theme_api": 1, "id": "dev.example.small", "name": "Small",
                "version": "1.0.0", "variants": { "dark": { "base": "circord" } } }"##
                .into(),
            ..Default::default()
        },
    )
    .expect("a package");
    circord_core::themes::install::Themes::open(folder.path())
        .install(&fetched)
        .expect("installed");
}

#[test]
fn ctrl_shift_t_turns_a_package_off_even_over_settings() {
    let folder = tempfile::tempdir_in(local()).expect("test folder");
    install_small_package(&folder);
    let mut app = start(&folder, true);
    assert!(app.choose_theme("installed:dev.example.small"));
    assert!(app.theme.packaged());
    open(&mut app, Section::Customize);
    ctrl_shift_t(&mut app);
    assert_eq!(app.theme.id, "bonfire");
    assert!(!app.theme.packaged());
    let kept = stored(&folder);
    assert!(kept["theme"].is_null());
    assert_eq!(kept["previous_theme"], "installed:dev.example.small");
}

/// Only a package is turned off: a palette theme stays, and isn't kept
/// as anything else.
#[test]
fn ctrl_shift_t_leaves_a_palette_theme_as_it_is() {
    let (mut app, folder) = preview();
    assert!(app.choose_theme("sakura"));
    ctrl_shift_t(&mut app);
    assert_eq!(app.theme.id, "sakura");
    let kept = stored(&folder);
    assert_eq!(kept["theme"], "sakura");
}

/// A package on trial ends as Revert ends it, back to the palette theme
/// chosen, which stays kept.
#[test]
fn ctrl_shift_t_ends_a_trial_and_keeps_the_palette_theme() {
    let folder = tempfile::tempdir_in(local()).expect("test folder");
    install_small_package(&folder);
    let mut app = start(&folder, true);
    assert!(app.choose_theme("sakura"));
    app.themes
        .try_installed("installed:dev.example.small")
        .expect("on trial");
    app.layout
        .theme_sounds
        .insert("trial".into(), crate::themes::SoundPrefs::default());
    app.show_wanted_theme();
    assert!(app.theme.packaged());
    ctrl_shift_t(&mut app);
    assert_eq!(app.theme.id, "sakura");
    assert!(app.themes.trial().is_none());
    assert!(!app.layout.theme_sounds.contains_key("trial"));
    assert_eq!(stored(&folder)["theme"], "sakura");
}

/// `--theme-dev`'s theme stops, and the palette theme chosen shows again.
#[test]
fn ctrl_shift_t_stops_the_dev_theme_and_keeps_the_palette_theme() {
    let (mut app, folder) = preview();
    assert!(app.choose_theme("sakura"));
    let dev = tempfile::tempdir_in(local()).expect("dev folder");
    std::fs::write(
        dev.path().join(circord_core::themes::MANIFEST),
        r##"{ "theme_api": 1, "id": "dev.example.dev", "name": "Dev",
            "version": "1.0.0", "variants": { "dark": { "base": "circord" } } }"##,
    )
    .unwrap();
    app.themes
        .develop(dev.path(), &app.layout)
        .expect("developed");
    app.show_wanted_theme();
    assert!(app.theme.packaged());
    ctrl_shift_t(&mut app);
    assert_eq!(app.theme.id, "sakura");
    assert!(!app.theme.packaged());
    assert!(app.themes.dev_folder().is_none(), "no longer watched");
    assert_eq!(stored(&folder)["theme"], "sakura");
}

#[test]
fn customize_is_its_own_settings_section_while_appearance_keeps_the_tiles() {
    let (mut app, _folder) = preview();
    open(&mut app, Section::Appearance);
    assert!(shows(&app, "Catppuccin Latte"), "Appearance's tiles");
    assert!(!shows(&app, "ACCENT"));
    assert!(!shows(&app, "PERFORMANCE"));
    interact(&mut app, |ui| {
        ui.click("Customize").expect("the Customize section");
    });
    assert!(shows(&app, "FONT") && shows(&app, "BACKGROUND"));
    assert!(shows(&app, "ACCENT"), "the accent's group");
    assert!(
        shows(&app, "PERFORMANCE"),
        "moved from the page it replaces"
    );
}

/// A package installed by hand applies through the control command or the
/// engine, and the options its manifest names show in Appearance.
#[test]
fn an_installed_theme_applies_and_turns_off_from_themes() {
    let folder = tempfile::tempdir_in(local()).expect("test folder");
    let source = folder.path().join("plum-source");
    std::fs::create_dir_all(source.join("sounds")).unwrap();
    std::fs::write(
        source.join("sounds/tick.wav"),
        include_bytes!("../../../../themes/template/sounds/tick.wav"),
    )
    .unwrap();
    std::fs::write(
        source.join(circord_core::themes::MANIFEST),
        r##"{
            "theme_api": 1,
            "id": "dev.example.plum",
            "name": "Plum package",
            "version": "1.0.0",
            "authors": ["Ada", "Lin"],
            "description": "A quiet plum theme.",
            "options": [{
                "id": "roomy", "label": "Roomy messages", "kind": "toggle",
                "default": false, "on": { "spacing.message_gap": 12 }
            }],
            "future_sparkle": true,
            "art": {
                "particles": [{ "preset": "sparkles", "on": "theme.applied" }]
            },
            "sounds": {
                "message": "sounds/tick.wav",
                "hover": "sounds/tick.wav"
            },
            "variants": {
                "dark": {
                    "base": "circord",
                    "colours": { "chat": "#2a1a3a" },
                    "layout": {
                        "composer": "top",
                        "messages": "bubbles",
                        "chat_max_width": 720,
                        "chat_avatars": false,
                        "bubbles": { "radius": 18, "padding": [14, 7] },
                        "rail_tiles": {
                            "shape": "ticket", "width": 56, "height": 40,
                            "radius": 6, "gap": 7, "pad": 3,
                            "label": "inside", "label_size": 9,
                            "label_colour": "#f4eaff", "label_ground": "#341e4f",
                            "label_case": "upper", "pill": "unread",
                            "hover": "lift", "hover_amount": 2,
                            "selected": { "offset": [2, 1] },
                            "badge": { "ground": "#ff7eb6", "ink": "#341e4f", "radius": 5 }
                        },
                        "channel_rows": {
                            "height": 41, "gap": 5, "size": 14, "case": "upper", "inset": 10,
                            "reserve": 12,
                            "separator": { "colour": "#5a476e", "width": 1, "inset": [4, 6] },
                            "ink": { "selected": "#ffffff" },
                            "icon": { "size": 18, "gap": 6 },
                            "selected": { "underline": { "colour": "#ff7eb6", "width": 2, "offset": 3 } },
                            "meta": { "voice": "voice · {n}", "size": 10,
                                "badge": { "ground": "#ff7eb6", "ink": "#341e4f", "radius": 5 } }
                        },
                        "member_rows": {
                            "height": 52, "gap": 4, "size": 15, "case": "lower", "reserve": 15,
                            "separator": { "colour": "#5a476e", "width": 1, "inset": [5, 7] },
                            "avatar": { "size": 36, "gap": 9 },
                            "margin": [6, 7], "inset": [10, 11], "activity": "hidden",
                            "status": "text", "sub": "none",
                            "meta": { "online": "here", "playing": "playing {activity}", "size": 10 },
                            "badge": { "text": "P{online}", "ground": "#ff7eb6", "ink": "#341e4f" }
                        }
                    }
                },
                "light": { "base": "sakura" }
            }
        }"##,
    )
    .unwrap();
    let fetched = circord_core::themes::install::Fetched::local(&source).unwrap();
    circord_core::themes::install::Themes::open(folder.path())
        .install(&fetched)
        .unwrap();

    let mut app = start(&folder, true);
    open(&mut app, Section::Appearance);
    // The package's ignored-field note is retained and shown verbatim.
    let ignored = "\"future_sparkle\" isn't something this Circord reads; ignored";
    assert!(
        app.themes.installed[0]
            .notes
            .iter()
            .any(|note| note == ignored)
    );
    let _ = app.update(Message::Control("theme installed:dev.example.plum".into()));
    assert_eq!(app.theme.id, "installed:dev.example.plum");
    assert_eq!(app.theme.chat, crate::style::hex(0x2a1a3a));
    assert!(shows(&app, "Roomy messages"), "the package's own option");
    let particle_events = std::sync::Arc::clone(
        app.themes
            .particles("installed:dev.example.plum")
            .expect("theme applied particles")
            .1,
    );
    particle_events.lock().unwrap().clear();
    app.timeline.heights = vec![1234.0];
    let _ = app.update(crate::settings::appearance_event(
        crate::settings::AppearanceEvent::ThemeOption(
            "installed:dev.example.plum".into(),
            "roomy".into(),
            serde_json::Value::Bool(true),
        ),
    ));
    assert_eq!(app.theme.spacing().message_gap, Some(12.0));
    assert_ne!(
        app.timeline.heights,
        vec![1234.0],
        "a metric-only option remeasures cached timeline rows"
    );
    assert!(
        particle_events.lock().unwrap().is_empty(),
        "an option resolution is not a deliberate theme apply"
    );
    let package_layout = app
        .themes
        .package_layout(&app.theme.id)
        .expect("resolved package layout");
    assert_eq!(
        package_layout.composer,
        Some(circord_core::themes::layout::ComposerSide::Top)
    );
    assert_eq!(
        package_layout.messages,
        Some(circord_core::themes::layout::MessageStyle::Bubbles)
    );
    assert_eq!(package_layout.chat_max_width, Some(720.0));
    assert_eq!(package_layout.chat_avatars, Some(false));
    assert_eq!(package_layout.bubbles.radius, Some(18.0));
    assert_eq!(package_layout.bubbles.padding, Some((14.0, 7.0)));
    let rail = package_layout.rail_tiles.as_ref().expect("rail tiles");
    assert_eq!(rail.width, 56.0);
    assert_eq!(rail.height, 40.0);
    assert_eq!(rail.label_size, 9.0);
    assert_eq!(rail.selected.offset, (2.0, 1.0));
    assert_eq!(rail.badge.radius, Some(5.0));
    assert_eq!(crate::rail::themed_width(Some(package_layout)), 74.0);
    let channels = package_layout.channel_rows.as_ref().expect("channel rows");
    assert_eq!(channels.height, Some(41.0));
    assert_eq!(channels.gap, Some(5.0));
    assert_eq!(channels.reserve, 12.0);
    assert_eq!(channels.icon.size, Some(18.0));
    assert_eq!(channels.icon.gap, Some(6.0));
    assert_eq!(channels.selected.underline.expect("underline").offset, 3.0);
    assert_eq!(channels.meta.voice, "voice · {n}");
    let members = package_layout.member_rows.as_ref().expect("member rows");
    assert_eq!(members.height, Some(52.0));
    assert_eq!(members.gap, Some(4.0));
    assert_eq!(members.reserve, 15.0);
    assert_eq!(members.avatar.size, Some(36.0));
    assert_eq!(members.meta.online.as_deref(), Some("here"));
    assert_eq!(
        members.badge.as_ref().expect("member badge").text,
        "P{online}"
    );
    assert!(!members.activity);
    assert_eq!(stored(&folder)["theme"], "installed:dev.example.plum");
    let _ = app.update(crate::settings::customize_event(
        crate::settings::CustomizeEvent::ThemeTransitions(true),
    ));
    assert_eq!(stored(&folder)["theme_transitions"], true);
}

/// A package whose files changed since it was installed isn't opened:
/// the engine counts it refused with the precise reason.
#[test]
fn a_refused_installed_theme_is_counted_with_the_precise_reason() {
    let folder = tempfile::tempdir_in(local()).expect("test folder");
    let source = folder.path().join("changed-source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join(circord_core::themes::MANIFEST),
        r##"{
            "theme_api": 1,
            "id": "dev.example.changed",
            "name": "Changed",
            "version": "1.0.0",
            "variants": { "dark": { "base": "circord" } }
        }"##,
    )
    .unwrap();
    let fetched = circord_core::themes::install::Fetched::local(&source).unwrap();
    let themes = circord_core::themes::install::Themes::open(folder.path());
    themes.install(&fetched).unwrap();
    let installed = themes.installed();
    let stored_manifest = installed[0].folder.join(circord_core::themes::MANIFEST);
    std::fs::write(stored_manifest, "{}").unwrap();

    let app = start(&folder, true);
    assert!(app.themes.installed.is_empty());
    let refused = app.themes.refused.first().expect("a refused theme");
    assert_eq!(refused.id, "dev.example.changed");
    assert_eq!(
        refused.reason,
        "its theme.json changed since it was installed"
    );
}

/// The person's own themes show as tiles in Appearance, choose, and go on
/// the bin's second press.
#[test]
fn a_custom_theme_chooses_and_deletes_from_its_tile() {
    use crate::settings::{AppearanceEvent, appearance_event};
    let folder = tempfile::tempdir_in(local()).expect("test folder");
    std::fs::write(
        folder.path().join("layout.json"),
        r##"{"theme": "custom:1", "custom_themes": [{"id": "custom:1", "name": "Harbour",
            "base": "circord", "colours": {"chat": "#2a1a3a"}}]}"##,
    )
    .expect("the layout");
    let mut app = start(&folder, true);
    assert_eq!(app.theme.id, "custom:1");
    open(&mut app, Section::Appearance);
    assert!(shows(&app, "Harbour"), "among the tiles");
    // Deleted, asked once more: the chosen theme goes, and Circord shows.
    let _ = app.update(appearance_event(AppearanceEvent::Delete("custom:1".into())));
    assert_eq!(app.theme.id, "custom:1", "armed only");
    let _ = app.update(appearance_event(AppearanceEvent::Delete("custom:1".into())));
    assert_eq!(app.theme.id, "circord");
    assert_eq!(stored(&folder)["custom_themes"], serde_json::json!([]));
}

#[test]
fn preferences_are_kept_across_a_restart() {
    let (mut app, folder) = preview();
    // The newest message sent at a time of its own, as Discord's are.
    change(&mut app, |state| {
        let mut messages = (*state.messages).clone();
        let last = messages.last_mut().expect("a message");
        let mut message = (**last).clone();
        message.timestamp = 1_790_000_000;
        *last = std::sync::Arc::new(message);
        state.messages = std::sync::Arc::new(messages);
    });
    open(&mut app, Section::Messages);
    interact(&mut app, |ui| {
        ui.click("12-hour").expect("the clock");
    });
    interact(&mut app, |ui| {
        ui.click("Show seconds").expect("the switch");
    });
    interact(&mut app, |ui| {
        ui.click("Collapse long messages").expect("the switch");
    });
    interact(&mut app, |ui| {
        ui.click("Large").expect("the text size");
    });
    assert!(app.layout.prefs.twelve_hour_clock && app.layout.prefs.clock_seconds);
    // The timeline's times follow at once: 9:41:05 PM, say.
    let time = &app.timeline.looks.last().expect("a message").time;
    assert!(
        time.matches(':').count() == 2 && (time.ends_with("AM") || time.ends_with("PM")),
        "{time}"
    );
    open(&mut app, Section::Notifications);
    interact(&mut app, |ui| {
        ui.click("Flash the taskbar").expect("the switch");
    });
    open(&mut app, Section::Advanced);
    interact(&mut app, |ui| {
        ui.click("Developer Mode").expect("the switch");
    });
    let again = start(&folder, true);
    let prefs = &again.layout.prefs;
    assert!(prefs.twelve_hour_clock && prefs.clock_seconds);
    assert!(again.layout.collapse_long && again.layout.developer_mode);
    assert_eq!(prefs.text_size, Some(2));
    assert_eq!(prefs.flash_taskbar, Some(false));
    // `layout.json` keeps them under the keys older versions wrote.
    let layout = stored(&folder);
    assert_eq!(layout["twelve_hour_clock"], true);
    assert_eq!(layout["flash_taskbar"], false);
}

#[test]
fn a_device_chosen_in_voice_settings_is_the_one_used() {
    let (mut app, _folder) = preview();
    change(&mut app, |state| {
        let device = |id: &str, name: &str| voice::Device {
            id: id.into(),
            name: name.into(),
        };
        state.voice.microphones = vec![device("mic-a", "Desk"), device("mic-b", "Headset")];
        state.voice.microphone = Some("mic-a".into());
    });
    open(&mut app, Section::Voice);
    assert!(shows(&app, "Desk"), "the one chosen");
    assert!(!shows(&app, "Headset"), "the rest only in the list");
    interact(&mut app, |ui| {
        ui.click("Desk").expect("the dropdown");
    });
    interact(&mut app, |ui| {
        ui.click("Headset").expect("its option");
    });
    assert_eq!(
        app.client.state().voice.microphone.as_deref(),
        Some("mic-b")
    );
    assert!(
        app.win.settings.get().is_some_and(|s| s.open.is_none()),
        "closed"
    );
}

#[test]
fn the_soundboard_volume_moves_with_its_drag() {
    let (mut app, _folder) = preview();
    change(&mut app, |state| state.discord_linked = true);
    open(&mut app, Section::Voice);
    assert!(shows(&app, "Soundboard volume"));
    assert!(!shows(&app, "37%"));
    let voice = |event| crate::app::settings(Event::Voice(event));
    let _ = app.update(voice(VoiceEvent::SoundboardVolume(37, false)));
    assert!(shows(&app, "37%"), "the slider follows the drag");
    // Let go, what the drag left is sent, not what the slider was built
    // with, and the slider shows the core's volume again.
    let _ = app.update(voice(VoiceEvent::SoundboardVolume(80, true)));
    assert!(
        app.win
            .settings
            .get()
            .is_some_and(|settings| settings.voice.soundboard.is_none())
    );
    assert!(!shows(&app, "37%"));
}

#[test]
fn a_recorded_push_to_talk_key_is_named_and_kept() {
    use crate::settings::keybinds::recorded;
    use iced::keyboard::key::{Code, Physical};
    let v = recorded(
        &Key::Character("v".into()),
        Physical::Code(Code::KeyV),
        Location::Standard,
    )
    .expect("a key");
    assert_eq!(v.name, "V");
    if cfg!(target_os = "linux") {
        assert_eq!(v.scancode, 47, "evdev's, as winit gives it");
        assert_eq!(v.trigger.as_deref(), Some("v"));
    }
    let ctrl = recorded(
        &Key::Named(key::Named::Control),
        Physical::Code(Code::ControlRight),
        Location::Right,
    )
    .expect("a key");
    assert_eq!(ctrl.name, "Right Ctrl");
    if cfg!(target_os = "linux") {
        assert_eq!(ctrl.trigger.as_deref(), Some("Control_R"));
    }

    let (mut app, folder) = preview();
    open(&mut app, Section::Voice);
    interact(&mut app, |ui| {
        ui.click("Push to talk").expect("the input mode");
    });
    interact(&mut app, |ui| {
        ui.click("Record keybind").expect("the button");
    });
    assert!(app.recording_keybind());
    assert!(shows(&app, "Press a key..."));
    // The keys pressed together are the keybind once they're all up, and
    // show as they're pressed.
    let heard = |key: &circord_core::keybinds::Key, pressed| {
        crate::app::settings(Event::Heard(crate::settings::keybinds::Heard::Key {
            key: key.clone(),
            pressed,
            escape: false,
        }))
    };
    let _ = app.update(heard(&ctrl, true));
    let _ = app.update(heard(&v, true));
    assert!(shows(&app, "Right Ctrl + V") || shows(&app, "Ctrl + V"));
    let _ = app.update(heard(&v, false));
    assert!(app.recording_keybind(), "a key is still down");
    let _ = app.update(heard(&ctrl, false));
    assert!(!app.recording_keybind());
    assert!(shows(&app, "Edit keybind"));
    let layout = stored(&folder);
    assert_eq!(layout["input_mode"], "push_to_talk");
    assert_eq!(layout["push_to_talk_key"]["name"], "V");
    assert_eq!(layout["push_to_talk_key"]["with"][0]["name"], "Right Ctrl");
    // Escape, pressed first, stops without changing it.
    interact(&mut app, |ui| {
        ui.click("Edit keybind").expect("the button");
    });
    let escape = circord_core::keybinds::Key {
        scancode: 1,
        name: "Escape".into(),
        trigger: None,
    };
    let _ = app.update(crate::app::settings(Event::Heard(
        crate::settings::keybinds::Heard::Key {
            key: escape,
            pressed: true,
            escape: true,
        },
    )));
    assert!(!app.recording_keybind());
    assert_eq!(stored(&folder)["push_to_talk_key"]["name"], "V");
    // Where the key works, as its watcher tells from its own thread,
    // shows without anything else happening; a word from a watcher gone
    // since doesn't.
    let told = app.keybinds.count;
    let status = |reach| {
        crate::keybinds::Told::Status(circord_core::keybinds::Status {
            reach,
            given: Default::default(),
        })
    };
    let everywhere = "Hold the key in any app to talk.";
    let _ = app.update(Message::Keybinds(
        told,
        status(circord_core::keybinds::Reach::Everywhere),
    ));
    assert!(shows(&app, everywhere));
    let focused = circord_core::keybinds::Reach::Focused("Your desktop has no global keys");
    let _ = app.update(Message::Keybinds(told - 1, status(focused.clone())));
    assert!(shows(&app, everywhere));
    // Kept while Settings are closed, for when they open.
    let _ = app.close_settings();
    let _ = app.update(Message::Keybinds(told, status(focused)));
    open(&mut app, Section::Voice);
    assert!(shows(
        &app,
        "Your desktop has no global keys, so push to talk works only while Circord's window is in front."
    ));
    let again = start(&folder, true);
    assert_eq!(
        again
            .layout
            .voice
            .push_to_talk_key
            .as_ref()
            .map(circord_core::keybinds::Shortcut::name),
        Some("Ctrl + V".to_owned())
    );
}

#[test]
fn keybinds_are_added_chosen_recorded_and_removed() {
    let (mut app, folder) = preview();
    open(&mut app, Section::Keybinds);
    assert!(shows(&app, "Add a Keybind"));
    interact(&mut app, |ui| {
        ui.click("Add a Keybind").expect("the button");
    });
    assert!(shows(&app, "No Keybind Set"));
    assert!(shows(
        &app,
        "This action doesn't do anything! Please choose one from the drop down."
    ));
    let keybind = |event| crate::app::settings(Event::Keybind(event));
    use crate::settings::keybinds::{Event as KeybindEvent, Heard, Target};
    let _ = app.update(keybind(KeybindEvent::Action(0, 4)));
    assert!(shows(&app, "Toggle Deafen"));
    let _ = app.update(crate::app::settings(Event::Record(Target::Row(0), true)));
    assert!(app.recording_keybind());
    let d = circord_core::keybinds::Key {
        scancode: 32,
        name: "D".into(),
        trigger: Some("d".into()),
    };
    for pressed in [true, false] {
        let _ = app.update(crate::app::settings(Event::Heard(Heard::Key {
            key: d.clone(),
            pressed,
            escape: false,
        })));
    }
    assert!(!app.recording_keybind());
    let layout = stored(&folder);
    assert_eq!(layout["keybinds"][0]["action"], "toggle_deafen");
    assert_eq!(layout["keybinds"][0]["keys"]["name"], "D");
    assert_eq!(layout["keybinds"][0]["id"], 1);
    // The watcher watches it: a toggle from it deafens.
    assert!(
        app.keybinds
            .watched
            .iter()
            .any(|watched| watched.id == "keybind-1")
    );
    let _ = app.update(keybind(KeybindEvent::Enabled(0, false)));
    assert!(app.keybinds.watched.is_empty());
    assert_eq!(stored(&folder)["keybinds"][0]["enabled"], false);
    assert!(
        app.layout.keybinds[0].keys.is_some(),
        "disabling preserves the keys"
    );
    let _ = app.update(keybind(KeybindEvent::Enabled(0, true)));
    assert_eq!(app.keybinds.watched.len(), 1);
    // As the account card's button: out of a call, the next one starts
    // deafened.
    let deafened =
        |app: &crate::app::App| (app.client.state().voice.deafened, app.layout.deafen_on_join);
    let before = deafened(&app);
    let _ = app.update(Message::Keybinds(
        app.keybinds.count,
        crate::keybinds::Told::Toggled(circord_core::keybinds::Action::ToggleDeafen),
    ));
    assert_ne!(deafened(&app), before);
    let _ = app.update(keybind(KeybindEvent::Remove(0)));
    assert!(!shows(&app, "Toggle Deafen"));
    assert_eq!(stored(&folder)["keybinds"], serde_json::json!([]));
    assert!(app.keybinds.watched.is_empty());
}

#[test]
fn a_new_setting_is_tagged_until_its_page_is_left() {
    use crate::settings::{Event as Settings, Section};
    // A layout from before the tags: everything tagged since is new.
    let folder = tempfile::tempdir_in(super::local()).expect("test folder");
    std::fs::write(folder.path().join("layout.json"), b"{}").expect("layout");
    let mut app = start(&folder, true);
    // Use the same full-page fixture as the other Settings view checks. With
    // Themes restored to the Slint build, the deliberately short
    // preview viewport otherwise clips About's row before this checks its tag.
    open(&mut app, Section::Account);
    assert!(shows(&app, "NEW"), "About's row says so");
    let _ = app.update(crate::app::settings(Settings::Open(Section::About)));
    assert!(shows(&app, "Check for updates") && shows(&app, "NEW"));
    // Leaving About sees them, and the layout keeps it.
    let _ = app.update(crate::app::settings(Settings::Open(Section::Account)));
    assert!(!shows(&app, "NEW"));
    let layout: serde_json::Value = serde_json::from_slice(&{
        crate::layout::flush(folder.path());
        std::fs::read(folder.path().join("layout.json")).expect("the layout")
    })
    .expect("JSON");
    let seen: Vec<&str> = layout["seen_new"]
        .as_array()
        .expect("seen")
        .iter()
        .filter_map(|key| key.as_str())
        .collect();
    assert_eq!(seen, ["check-updates", "update-nightlies"]);
    // A fresh install has met them all.
    let (mut fresh, _folder) = preview();
    let _ = fresh.update(crate::app::settings(Settings::Open(Section::About)));
    assert!(!shows(&fresh, "NEW"));
}

/// Every key the core tags has its tag in Settings' pages.
#[test]
fn every_tagged_setting_has_its_tag() {
    let pages = [
        include_str!("../settings/pages/mod.rs"),
        include_str!("../settings/pages/about.rs"),
        include_str!("../settings/pages/account.rs"),
        include_str!("../settings/pages/advanced.rs"),
        include_str!("../settings/pages/discord.rs"),
        include_str!("../settings/pages/games.rs"),
        include_str!("../settings/pages/messages.rs"),
        include_str!("../settings/pages/notifications.rs"),
        include_str!("../settings/pages/sounds.rs"),
    ]
    .concat();
    for tag in &circord_core::new_tags::NEW {
        assert!(
            pages.contains(&format!("self.fresh(\"{}\")", tag.key)),
            "{}",
            tag.key
        );
    }
}

#[test]
fn a_theme_switch_lays_the_timelines_text_out_in_the_new_inks() {
    use circord_core::chat::{ChatMessage, Notice, NoticeKind};
    let (mut app, _folder) = preview();
    super::change(&mut app, |state| {
        state.messages = std::sync::Arc::new(vec![std::sync::Arc::new(ChatMessage {
            item: "n0".into(),
            author: "Ann".into(),
            time: "12:01".into(),
            notice: Some(Notice {
                kind: NoticeKind::Pinned(Some("m1".into())),
                text: "".into(),
            }),
            ..Default::default()
        })]);
    });
    let ink = |app: &crate::app::App| {
        app.timeline.looks[0].body.parts[0]
            .runs
            .iter()
            .find(|run| run.text.contains("Ann"))
            .and_then(|run| run.color)
    };
    assert_eq!(ink(&app), Some(crate::theme::rgb(app.theme.text)));
    assert!(app.choose_theme("catppuccin-latte"), "a light theme");
    let _ = app.sync();
    assert_eq!(
        ink(&app),
        Some(crate::theme::rgb(app.theme.text)),
        "the author's name in the light theme's ink"
    );
}

#[test]
fn the_theme_command_switches_the_colours_and_keeps_the_choice() {
    let (mut app, folder) = preview();
    assert_eq!(app.theme.id, "bonfire", "the default with the Discord look");
    let _ = app.update(Message::Control("theme sakura".into()));
    assert_eq!(app.theme.id, "sakura");
    assert!(!app.theme.dark);
    assert_eq!(app.theme().palette().background, app.theme.chat);
    let layout: serde_json::Value = serde_json::from_slice(&{
        crate::layout::flush(folder.path());
        std::fs::read(folder.path().join("layout.json")).expect("the layout")
    })
    .expect("JSON");
    assert_eq!(layout["theme"], "sakura");
    // A theme Circord doesn't have changes nothing.
    let _ = app.update(Message::Control("theme gone".into()));
    assert_eq!(app.theme.id, "sakura");
    // The next start shows the one chosen.
    assert_eq!(start(&folder, true).theme.id, "sakura");
}

#[test]
fn theme_performance_reduced_motion_and_readable_text_are_kept_in_the_layout() {
    let (mut app, folder) = preview();
    open(&mut app, Section::Customize);
    let _ = app.update(crate::settings::customize_event(
        crate::settings::CustomizeEvent::Performance(2),
    ));
    let _ = app.update(crate::settings::customize_event(
        crate::settings::CustomizeEvent::ReduceMotion(true),
    ));
    let _ = app.update(crate::settings::appearance_event(
        crate::settings::AppearanceEvent::ReadableText(true),
    ));
    assert_eq!(
        app.layout.theme_performance,
        circord_core::themes::Performance::Light
    );
    assert!(app.layout.reduce_motion);
    assert!(app.layout.readable_text);
    crate::layout::flush(folder.path());
    let layout: serde_json::Value =
        serde_json::from_slice(&std::fs::read(folder.path().join("layout.json")).unwrap()).unwrap();
    assert_eq!(layout["theme_performance"], "light");
    assert_eq!(layout["reduce_motion"], true);
    assert_eq!(layout["readable_text"], true);
}

#[test]
fn a_custom_theme_in_the_layout_shows_from_the_start() {
    let folder = tempfile::tempdir_in(local()).expect("test folder");
    std::fs::write(
        folder.path().join("layout.json"),
        r##"{"theme": "custom:1", "custom_themes": [{"id": "custom:1", "name": "Plum",
            "base": "circord", "colours": {"chat": "#2a1a3a", "accent": "#c04080"}}]}"##,
    )
    .expect("the layout");
    let app = start(&folder, true);
    assert_eq!(app.theme.id, "custom:1");
    assert_eq!(app.theme.chat, crate::style::hex(0x2a1a3a));
    assert_eq!(app.theme.accent, crate::style::hex(0xc04080));
    assert_eq!(app.theme.selection.a, 0.35, "worked out from its accent");
}

#[test]
fn a_layout_key_of_the_wrong_kind_loses_only_itself() {
    let folder = tempfile::tempdir_in(local()).expect("test folder");
    std::fs::write(
        folder.path().join("layout.json"),
        r#"{"theme": "sakura", "collapse_long": "yes", "members_hidden": true}"#,
    )
    .expect("the layout");
    let app = start(&folder, true);
    assert_eq!(app.theme.id, "sakura");
    assert!(app.layout.members_hidden);
    assert!(!app.layout.collapse_long);
}

#[test]
fn edit_profile_from_settings_closes_them_under_the_editor() {
    let (mut app, _folder) = preview();
    change(&mut app, |state| state.discord_user = Some("42".into()));
    open(&mut app, Section::Account);
    let _ = app.update(crate::app::settings(Event::EditProfile));
    assert!(
        app.client.state().profile_editor.is_some(),
        "the editor opens"
    );
    assert!(app.win.settings.get().is_none(), "Settings close under it");
}

/// A call going on in Lounge, joined as the preview joins, unmuted
/// unless `muted`: after the join, which turns the microphone on.
fn in_a_call(app: &mut App, muted: bool) {
    for command in [
        voice::Command::Join("lounge".into()),
        voice::Command::Mute(muted),
    ] {
        app.client
            .dispatch(circord_core::application::Action::Voice(command));
        let _ = app.sync();
    }
    let voice = &app.client.state().voice;
    assert_eq!(
        (voice.active(), voice.muted, voice.deafened),
        (true, muted, false)
    );
}

fn testing(app: &App) -> bool {
    app.win
        .settings
        .get()
        .is_some_and(|settings| settings.voice.testing && settings.voice.gate.is_some())
}

#[test]
fn the_mic_test_deafens_a_call_while_it_plays_back_as_discords_does() {
    let (mut app, _folder) = preview();
    in_a_call(&mut app, false);
    open(&mut app, Section::Voice);
    assert!(shows(
        &app,
        "You'll be muted and deafened in your call while you test."
    ));
    interact(&mut app, |ui| {
        ui.click("Let's check").expect("the mic test's button");
    });
    assert!(testing(&app), "the meter plays the microphone back");
    let call = |app: &App| {
        let voice = &app.client.state().voice;
        (voice.muted, voice.deafened)
    };
    assert_eq!(call(&app), (true, true), "deafened, and so muted");
    assert!(shows(&app, "Stop testing"));
    interact(&mut app, |ui| {
        ui.click("Stop testing").expect("the button");
    });
    assert!(!testing(&app));
    assert_eq!(call(&app), (false, false), "as it was");
    // Undeafened while it plays, the test ends, as Discord's does.
    let _ = app.update(crate::app::settings(Event::Voice(VoiceEvent::MicTest)));
    assert!(testing(&app));
    app.client
        .dispatch(circord_core::application::Action::Voice(
            voice::Command::Deafen(false),
        ));
    let _ = app.sync();
    assert!(!testing(&app));
    assert!(!app.client.state().voice.deafened);
}

#[test]
fn leaving_voice_or_settings_ends_the_mic_test_and_the_call_hears_again() {
    let (mut app, _folder) = preview();
    in_a_call(&mut app, true);
    open(&mut app, Section::Voice);
    let mic_test = crate::app::settings(Event::Voice(VoiceEvent::MicTest));
    let _ = app.update(mic_test.clone());
    assert!(app.client.state().voice.deafened);
    let _ = app.update(crate::app::settings(Event::Open(Section::Advanced)));
    let voice = &app.client.state().voice;
    assert_eq!(
        (voice.muted, voice.deafened),
        (true, false),
        "muted as before"
    );
    open(&mut app, Section::Voice);
    assert!(!testing(&app), "it doesn't start again");
    let _ = app.update(mic_test);
    assert!(app.client.state().voice.deafened);
    let _ = app.update(crate::app::settings(Event::Close));
    assert!(!app.client.state().voice.deafened, "closing ends it too");
}

#[test]
fn the_mic_test_without_a_call_deafens_nothing() {
    let (mut app, _folder) = preview();
    open(&mut app, Section::Voice);
    assert!(!shows(
        &app,
        "You'll be muted and deafened in your call while you test."
    ));
    let call = |app: &App| {
        let voice = &app.client.state().voice;
        (voice.muted, voice.deafened)
    };
    let before = call(&app);
    let _ = app.update(crate::app::settings(Event::Voice(VoiceEvent::MicTest)));
    assert!(testing(&app));
    assert_eq!(call(&app), before);
}

/// Settings → Customize's accent, background and font: each shows at
/// once and is kept, a colour still being typed changes nothing, and
/// emptying the accent takes the theme's own again.
#[test]
fn the_customized_accent_background_and_font_show_at_once_and_are_kept() {
    let (mut app, folder) = preview();
    open(&mut app, Section::Customize);
    let accent_before = app.theme.accent;
    let chat_before = app.theme.chat;
    let event = |event| crate::settings::customize_event(event);
    // A wrong colour still being typed changes nothing.
    let _ = app.update(event(crate::settings::CustomizeEvent::Accent(
        "#notacolor".into(),
    )));
    assert_eq!(app.theme.accent, accent_before);
    assert!(app.layout.customize.accent.is_none());
    // One that parses shows at once and is kept.
    let _ = app.update(event(crate::settings::CustomizeEvent::Accent(
        "#102030".into(),
    )));
    assert_eq!(app.theme.accent, crate::style::hex(0x102030));
    assert_eq!(app.theme.selection.a, 0.35, "worked out from it");
    assert_eq!(app.layout.customize.accent.as_deref(), Some("#102030"));
    // The background's colour stands for the whole ground; the gradient
    // tints the theme's own instead.
    let _ = app.update(event(crate::settings::CustomizeEvent::Background(1)));
    let _ = app.update(event(crate::settings::CustomizeEvent::Colour(
        0,
        "#203040".into(),
    )));
    assert_eq!(app.theme.chat, crate::style::hex(0x203040));
    let _ = app.update(event(crate::settings::CustomizeEvent::Background(2)));
    let _ = app.update(event(crate::settings::CustomizeEvent::Colour(
        1,
        "#000000".into(),
    )));
    let _ = app.update(event(crate::settings::CustomizeEvent::Colour(
        2,
        "#ffffff".into(),
    )));
    let _ = app.update(event(crate::settings::CustomizeEvent::Angle(90)));
    match app.layout.customize.background.as_ref() {
        Some(crate::layout::CustomBackground::Gradient { from, to, angle }) => {
            assert_eq!(from, "#000000");
            assert_eq!(to, "#ffffff");
            assert_eq!(*angle, 90.0);
        }
        other => panic!("a gradient, not {other:?}"),
    }
    assert_eq!(app.theme.chat, chat_before, "the theme's own, tinted");
    // The font's key is kept; the theme's own face is "".
    let _ = app.update(event(crate::settings::CustomizeEvent::Font(1)));
    assert_eq!(app.layout.customize.font, "atkinson");
    // Back to the theme's own.
    let _ = app.update(event(crate::settings::CustomizeEvent::AccentReset));
    let _ = app.update(event(crate::settings::CustomizeEvent::Background(0)));
    assert_eq!(app.theme.accent, accent_before);
    assert_eq!(app.theme.chat, chat_before);
    // And all of it again on the next start.
    let _ = app.update(event(crate::settings::CustomizeEvent::Accent(
        "#102030".into(),
    )));
    let again = start(&folder, true);
    assert_eq!(again.theme.accent, crate::style::hex(0x102030));
    let layout = stored(&folder);
    assert_eq!(layout["customize"]["accent"], "#102030");
    assert_eq!(layout["customize"]["font"], "atkinson");
}
