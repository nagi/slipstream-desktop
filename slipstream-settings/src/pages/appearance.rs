//! Appearance: the theme Slipstream is drawn in, its three colours, motion and night light.

use super::*;
use crate::{harmony, picker::ColourButton};

/// What the row that follows Omarchy's theme says: which theme the desktop is drawn in, or why
/// none is.
fn followed_text(settings: &Settings) -> String {
    if !settings.appearance.follow_omarchy_theme {
        return "Slipstream is drawn in its own colours, whatever theme Omarchy is showing."
            .to_string();
    }
    match settings.appearance.theme() {
        Some(theme) => format!(
            "Slipstream is drawn in the theme, a {} one: its bar, panels, rings, streams and \
             wallpapers, this window, and the colours apps are asked to take. It changes as \
             soon as the theme does.",
            if theme.light { "light" } else { "dark" }
        ),
        None => "Omarchy has no theme set whose colours Slipstream can read, so it is drawn in \
                 its own."
            .to_string(),
    }
}

/// Omarchy's themes by the names it shows them under, from its own list. Empty where there is
/// no Omarchy.
fn omarchy_themes() -> Vec<String> {
    std::process::Command::new("omarchy-theme-list")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|listed| listed.status.success())
        .map(|listed| {
            String::from_utf8_lossy(&listed.stdout)
                .lines()
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Has Omarchy set the theme called `name`, as its own menu would. Omarchy recolours the
/// terminal, the editor and the rest of what it looks after, and Slipstream follows the
/// result. Its work goes on after this returns.
fn set_omarchy_theme(name: &str) {
    let started = std::process::Command::new("omarchy-theme-set")
        .arg(name)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    match started {
        // Waited for off the main thread, so it doesn't linger once it has finished.
        Ok(mut setting) => {
            std::thread::spawn(move || {
                let _ = setting.wait();
            });
        }
        Err(err) => eprintln!("slipstream-settings: couldn't run omarchy-theme-set: {err}"),
    }
}

/// The page is built again whenever what it offers changes: the colours to choose from are the
/// followed theme's own.
pub(super) fn build(store: &Store) -> gtk::Widget {
    let holder = gtk::Box::new(gtk::Orientation::Vertical, 0);
    holder.append(&appearance_page(store));
    let offered = |settings: &Settings| {
        (
            settings.appearance.follow_omarchy_theme,
            settings.appearance.theme(),
        )
    };
    let shown = Rc::new(RefCell::new(offered(&store.get())));
    crate::when_restyled({
        let (store, holder) = (store.clone(), holder.downgrade());
        move || {
            let settings = slipstream_config::read(store.path()).unwrap_or_else(|_| store.get());
            let now = offered(&settings);
            if *shown.borrow() == now {
                return;
            }
            shown.replace(now);
            // Not from inside the control whose change brought this about.
            let (store, holder) = (store.clone(), holder.clone());
            gtk::glib::idle_add_local_once(move || {
                let Some(holder) = holder.upgrade() else {
                    return;
                };
                store.reload();
                while let Some(old) = holder.first_child() {
                    holder.remove(&old);
                }
                holder.append(&appearance_page(&store));
            });
        }
    });
    holder.upcast()
}

fn appearance_page(store: &Store) -> gtk::Box {
    let page = page(
        "Appearance",
        "The theme Slipstream is drawn in, what it marks things with, and how much it moves",
    );
    let settings = store.get();
    let theme = settings.appearance.theme();
    // Only where there is an Omarchy to follow.
    if slipstream_config::theme::current_dir().is_dir() {
        let omarchy = group(&page, "Theme");
        let themes = omarchy_themes();
        if let (false, Some(theme)) = (themes.is_empty(), &theme) {
            let names: Vec<&str> = themes.iter().map(String::as_str).collect();
            let list = gtk::DropDown::from_strings(&names);
            let current = theme.title();
            let at = themes
                .iter()
                .position(|name| name.eq_ignore_ascii_case(&current));
            list.set_selected(at.map_or(gtk::INVALID_LIST_POSITION, |at| at as u32));
            list.connect_selected_notify(move |list| {
                if let Some(name) = themes.get(list.selected() as usize)
                    && !name.eq_ignore_ascii_case(&current)
                {
                    set_omarchy_theme(name);
                }
            });
            row(
                &omarchy,
                "Omarchy theme",
                Some(
                    "Sets the theme through Omarchy, as its own menu does, so the terminal, the \
                     editor and everything else Omarchy colours change with the desktop.",
                ),
                &list,
            );
        }
        let follow = gtk::Switch::new();
        follow.set_active(settings.appearance.follow_omarchy_theme);
        let follow_store = store.clone();
        follow.connect_active_notify(move |follow| {
            let on = follow.is_active();
            follow_store.change(|settings| settings.appearance.follow_omarchy_theme = on);
            crate::retheme();
        });
        row(
            &omarchy,
            "Follow Omarchy's theme",
            Some(&followed_text(&settings)),
            &follow,
        );
    }

    let colours = group(&page, "Colours");
    match &theme {
        // Under a theme the three are the theme's own colours, chosen by name so the choice goes
        // with the next theme too.
        Some(theme) => {
            let choices = colour_choices(theme);
            let borders = &settings.borders;
            colour_row(
                &colours,
                store,
                "Primary colour",
                "Where the keyboard is: the ring around the focused window and in the panels, \
                 and the workspace you're on. Also a battery that's charging, plugins that are \
                 on, and what apps take for their accent.",
                choices.clone(),
                (&borders.selected_tile, borders.ring(Some(theme))),
                |settings, chosen| settings.borders.selected_tile = chosen,
            );
            colour_row(
                &colours,
                store,
                "Secondary colour",
                "Bullet time, and whatever wants your attention: the notification count, Caps \
                 Lock, a low battery, and apps and plugins asking to be looked at.",
                choices.clone(),
                (&borders.bullet_time, borders.secondary(Some(theme))),
                |settings, chosen| settings.borders.bullet_time = chosen,
            );
            colour_row(
                &colours,
                store,
                "Tertiary colour",
                "What's on or chosen: switches, tiles and ticks, sliders, today in the calendar, \
                 the text cursor, toasts and the button Enter presses.",
                choices,
                (&borders.accent, borders.highlight_colour(Some(theme))),
                |settings, chosen| settings.borders.accent = chosen,
            );
        }
        // A followed theme says whether it is dark or light. Without one it is chosen here, with
        // Slipstream's own sets of three colours.
        None => {
            let schemes = slipstream_config::ColourScheme::ALL;
            let names: Vec<&str> = schemes.iter().map(|scheme| scheme.label()).collect();
            let scheme = gtk::DropDown::from_strings(&names);
            let chosen = settings.appearance.colour_scheme;
            scheme
                .set_selected(schemes.iter().position(|held| *held == chosen).unwrap_or(0) as u32);
            let scheme_store = store.clone();
            scheme.connect_selected_notify(move |scheme| {
                if let Some(chosen) = schemes.get(scheme.selected() as usize).copied() {
                    scheme_store.change(move |settings| settings.appearance.colour_scheme = chosen);
                }
            });
            row(
                &colours,
                "Apps",
                Some(
                    "Which one apps are asked for, and they change without restarting. \
                     Slipstream's own bar, panels and windows are dark either way, and an app \
                     that never asks the desktop keeps whatever colours it was set to.",
                ),
                &scheme,
            );
            self::colours(&colours, store);
        }
    }
    let motion = group(&page, "Motion");
    // The same name quick settings gives this switch.
    let reduced = gtk::Switch::new();
    reduced.set_active(store.get().motion.reduced);
    let reduced_store = store.clone();
    reduced.connect_active_notify(move |reduced| {
        let on = reduced.is_active();
        reduced_store.change(|settings| settings.motion.reduced = on);
    });
    row(
        &motion,
        "Reduced motion",
        Some("Windows jump into place and effects become short fades"),
        &reduced,
    );
    let styles = [
        (slipstream_config::Effects::Slipstream, "Slipstream"),
        (slipstream_config::Effects::Matrix, "Matrix rain"),
    ];
    let names: Vec<&str> = styles.iter().map(|(_, name)| *name).collect();
    let effects = gtk::DropDown::from_strings(&names);
    let current = store.get().motion.effects;
    effects.set_selected(
        styles
            .iter()
            .position(|(which, _)| *which == current)
            .unwrap_or(0) as u32,
    );
    let effects_store = store.clone();
    effects.connect_selected_notify(move |effects| {
        if let Some((which, _)) = styles.get(effects.selected() as usize) {
            let which = *which;
            effects_store.change(|settings| settings.motion.effects = which);
        }
    });
    row(
        &motion,
        "Effects",
        Some("What minimised apps turn into: falling code, or streaks of light"),
        &effects,
    );
    let rain = gtk::Switch::new();
    rain.set_active(store.get().motion.close_into_rain);
    let rain_store = store.clone();
    rain.connect_active_notify(move |rain| {
        let on = rain.is_active();
        rain_store.change(|settings| settings.motion.close_into_rain = on);
    });
    row(
        &motion,
        "Close into the rain",
        Some("A closed window falls away in the same effects instead of fading"),
        &rain,
    );

    night_light(&page, store);
    page
}

/// Times offered for a night light schedule: every half hour.
fn half_hours() -> Vec<String> {
    (0..48)
        .map(|n| slipstream_config::sun::format_time(n as f64 * 30.0))
        .collect()
}

/// Tonight's sunset and tomorrow's sunrise where the time zone's city is, as the schedule's row
/// says them.
fn sunset_note() -> String {
    let now = gtk::glib::DateTime::now_local();
    let location = slipstream_config::sun::location();
    let times = now.ok().zip(location).and_then(|(now, (lat, lon))| {
        let date = (
            now.year() as i64,
            now.month() as u32,
            now.day_of_month() as u32,
        );
        let offset = now.utc_offset().as_minutes() as f64;
        slipstream_config::sun::sun_times(lat, lon, date, offset)
    });
    match times {
        Some((sunrise, sunset)) => format!(
            "Warms up over half an hour from sunset, about {} today, and cools again from \
             sunrise, about {}, going by your time zone. Turning it by hand in quick settings \
             lasts until the next change.",
            slipstream_config::sun::format_time(sunset),
            slipstream_config::sun::format_time(sunrise)
        ),
        None => "Your time zone doesn't say where the sun sets, so the hours below are used \
                 instead."
            .to_string(),
    }
}

fn night_light(page: &gtk::Box, store: &Store) {
    use slipstream_config::NightSchedule;
    let night = group(page, "Night light");
    let schedules = [
        (NightSchedule::Off, "Only by hand"),
        (NightSchedule::Sunset, "Sunset to sunrise"),
        (NightSchedule::Custom, "Set hours"),
    ];
    let names: Vec<&str> = schedules.iter().map(|(_, name)| *name).collect();
    let schedule = gtk::DropDown::from_strings(&names);
    let current = store.get().display.night_light_schedule;
    schedule.set_selected(
        schedules
            .iter()
            .position(|(which, _)| *which == current)
            .unwrap_or(0) as u32,
    );
    let sub = row(
        &night,
        "Schedule",
        Some("Warmer colours in the evening. Quick settings turns it on and off by hand."),
        &schedule,
    );

    let times = half_hours();
    let names: Vec<&str> = times.iter().map(String::as_str).collect();
    let pick = |value: &str| {
        let minutes = slipstream_config::sun::parse_time(value).unwrap_or(0.0);
        ((minutes / 30.0).round() as u32).min(47)
    };
    let from = gtk::DropDown::from_strings(&names);
    from.set_selected(pick(&store.get().display.night_light_from));
    let to = gtk::DropDown::from_strings(&names);
    to.set_selected(pick(&store.get().display.night_light_to));
    row(&night, "From", None, &from);
    row(
        &night,
        "Until",
        Some("The next morning, if earlier than the start."),
        &to,
    );

    let follow = {
        let (from, to, sub) = (from.clone(), to.clone(), sub.clone());
        move |which: NightSchedule| {
            // The hours count for a set schedule, and for sunset where the sun's times can't be
            // worked out.
            let custom = which == NightSchedule::Custom
                || (which == NightSchedule::Sunset && slipstream_config::sun::location().is_none());
            from.set_sensitive(custom);
            to.set_sensitive(custom);
            if let Some(sub) = &sub {
                sub.set_text(&match which {
                    NightSchedule::Off => "Warmer colours in the evening. Quick settings \
                                           turns it on and off by hand."
                        .to_string(),
                    NightSchedule::Sunset => sunset_note(),
                    NightSchedule::Custom => "Warms up over half an hour from the start, and \
                                              cools over half an hour from the end."
                        .to_string(),
                });
            }
        }
    };
    follow(current);
    let schedule_store = store.clone();
    schedule.connect_selected_notify(move |schedule| {
        if let Some((which, _)) = schedules.get(schedule.selected() as usize) {
            let which = *which;
            follow(which);
            schedule_store.change(move |settings| settings.display.night_light_schedule = which);
        }
    });
    for (dropdown, is_from) in [(from, true), (to, false)] {
        let times = times.clone();
        let store = store.clone();
        dropdown.connect_selected_notify(move |dropdown| {
            if let Some(time) = times.get(dropdown.selected() as usize).cloned() {
                store.change(move |settings| {
                    if is_from {
                        settings.display.night_light_from = time;
                    } else {
                        settings.display.night_light_to = time;
                    }
                });
            }
        });
    }
}

/// What the theme list says when the colours are none of its sets.
const YOUR_OWN: &str = "Your own";

/// The focus ring, bullet time's colour and the accent, each channel 0–1.
type Three = [[f32; 3]; 3];

/// The colour rows, kept in step with each other: whatever changes one shows the result on all.
struct Colours {
    store: Store,
    theme: gtk::DropDown,
    names: gtk::StringList,
    shown: Cell<Three>,
    wheel: gtk::Scale,
    focus: ColourButton,
    bullet: ColourButton,
    accent: ColourButton,
    /// Set while the rows are being changed to match, so their own handlers stay quiet.
    syncing: Cell<bool>,
}

impl Colours {
    fn current_of(settings: &Settings) -> Three {
        let borders = &settings.borders;
        [
            borders.selected_tile_rgb(None),
            borders.bullet_time_rgb(None),
            borders.accent_rgb(None),
        ]
    }

    /// Saves `three` and shows it on every row.
    fn choose(&self, three: Three) {
        let [focus, bullet, accent] = three.map(harmony::hex);
        self.store.change(|settings| {
            settings.borders.selected_tile = focus;
            settings.borders.bullet_time = bullet;
            settings.borders.accent = accent;
        });
        crate::restyle(three);
        self.show(three);
    }

    /// A focus ring of `focus`, with the two colours that go with it.
    fn choose_focus(&self, focus: [f32; 3]) {
        let (bullet, accent) = harmony::partners(focus);
        self.choose([focus, bullet, accent]);
    }

    fn show(&self, three: Three) {
        self.syncing.set(true);
        self.shown.set(three);
        let sets = harmony::THEMES.len() as u32;
        match matching_theme(three) {
            Some(index) => {
                self.theme.set_selected(index as u32);
                if self.names.n_items() > sets {
                    self.names.remove(sets);
                }
            }
            None => {
                if self.names.n_items() == sets {
                    self.names.append(YOUR_OWN);
                }
                self.theme.set_selected(sets);
            }
        }
        // The rainbow moves to the focus ring's hue, unless it's already within a step of it.
        let hue = harmony::hue(three[0]);
        if (self.wheel.value() - hue).abs() >= 1.0 {
            self.wheel.set_value(hue.round());
        }
        for (button, rgb) in [&self.focus, &self.bullet, &self.accent].iter().zip(three) {
            button.set(rgb);
        }
        self.syncing.set(false);
    }
}

/// Which of the sets `three` is, if it's one of them.
fn matching_theme(three: Three) -> Option<usize> {
    harmony::THEMES.iter().position(|theme| {
        [theme.focus, theme.bullet, theme.accent]
            .map(slipstream_config::rgb)
            .iter()
            .zip(three)
            .all(|(held, rgb)| *held == Some(rgb))
    })
}

/// The theme list's entries: a set's three colours as dots, then its name.
fn theme_entries() -> gtk::SignalListItemFactory {
    const DOT: i32 = 14;
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let dots = gtk::DrawingArea::new();
        dots.set_content_width(3 * DOT + 2 * 4);
        dots.set_content_height(DOT);
        dots.set_valign(gtk::Align::Center);
        let entry = gtk::Box::builder().spacing(10).build();
        entry.append(&dots);
        entry.append(&gtk::Label::new(None));
        item.set_child(Some(&entry));
    });
    factory.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let name = item
            .item()
            .and_downcast::<gtk::StringObject>()
            .map(|name| name.string().to_string())
            .unwrap_or_default();
        let Some(entry) = item.child() else {
            return;
        };
        let dots = entry.first_child().and_downcast::<gtk::DrawingArea>();
        let label = entry.last_child().and_downcast::<gtk::Label>();
        let (Some(dots), Some(label)) = (dots, label) else {
            return;
        };
        label.set_label(&name);
        let set = harmony::THEMES.iter().find(|set| set.name == name);
        dots.set_visible(set.is_some());
        let three = set.map(|set| [set.focus, set.bullet, set.accent].map(slipstream_config::rgb));
        dots.set_draw_func(move |_, cairo, _, height| {
            let radius = height as f64 / 2.0 - 1.0;
            for (n, rgb) in three.iter().flatten().enumerate() {
                let Some([r, g, b]) = rgb else { continue };
                let x = height as f64 / 2.0 + n as f64 * (height as f64 + 4.0);
                cairo.arc(x, height as f64 / 2.0, radius, 0.0, std::f64::consts::TAU);
                cairo.set_source_rgb(*r as f64, *g as f64, *b as f64);
                let _ = cairo.fill();
            }
        });
    });
    factory
}

/// The theme, the focus ring on a rainbow with the other two following it, and each colour on
/// its own for a choice of one's own.
fn colours(card: &gtk::Box, store: &Store) {
    let names: Vec<&str> = harmony::THEMES.iter().map(|theme| theme.name).collect();
    let names = gtk::StringList::new(&names);
    let theme = gtk::DropDown::new(Some(names.clone()), gtk::Expression::NONE);
    theme.set_factory(Some(&theme_entries()));
    let wheel = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 360.0, 1.0);
    wheel.set_round_digits(0);
    wheel.set_draw_value(false);
    wheel.set_width_request(200);
    wheel.add_css_class("hue");
    // The handlers hold it; GTK lets them go with the widgets when the window closes.
    let colours = Rc::new(Colours {
        store: store.clone(),
        theme,
        names,
        shown: Cell::new(Colours::current_of(&store.get())),
        wheel,
        focus: ColourButton::new("Primary colour"),
        bullet: ColourButton::new("Secondary colour"),
        accent: ColourButton::new("Tertiary colour"),
        syncing: Cell::new(false),
    });

    let chosen = Rc::clone(&colours);
    colours.theme.connect_selected_notify(move |theme| {
        if chosen.syncing.get() {
            return;
        }
        if let Some(set) = harmony::THEMES.get(theme.selected() as usize) {
            let three = [set.focus, set.bullet, set.accent]
                .map(|hex| slipstream_config::rgb(hex).expect("a valid theme"));
            chosen.choose(three);
        }
    });
    let turned = Rc::clone(&colours);
    colours.wheel.connect_value_changed(move |wheel| {
        if !turned.syncing.get() {
            turned.choose_focus(harmony::on_wheel(wheel.value()));
        }
    });
    // The handlers hold the colours weakly: the buttons belong to them, so a strong hold would
    // keep them alive for good.
    let picked = Rc::downgrade(&colours);
    colours.focus.connect_chosen(move |rgb| {
        if let Some(picked) = picked.upgrade() {
            picked.choose_focus(rgb);
        }
    });
    for (which, button) in [(1, &colours.bullet), (2, &colours.accent)] {
        let picked = Rc::downgrade(&colours);
        button.connect_chosen(move |rgb| {
            if let Some(picked) = picked.upgrade() {
                let mut three = picked.shown.get();
                three[which] = rgb;
                picked.choose(three);
            }
        });
    }
    colours.show(colours.shown.get());

    row(
        card,
        "Theme",
        Some("Sets of three colours that go together. Change any of them below to make your own."),
        &colours.theme,
    );
    let focus_control = gtk::Box::builder().spacing(10).build();
    focus_control.append(&colours.wheel);
    focus_control.append(&colours.focus.button);
    row(
        card,
        "Primary colour",
        Some(
            "Where the keyboard is: the ring around the focused window and in the panels, and the \
             workspace you're on. Also a battery that's charging and plugins that are on. Pick \
             any colour, and the two below are chosen to go with it.",
        ),
        &focus_control,
    );
    row(
        card,
        "Secondary colour",
        Some(
            "Bullet time, and whatever wants your attention: the notification count, Caps Lock, \
             a low battery, and apps and plugins asking to be looked at.",
        ),
        &colours.bullet.button,
    );
    row(
        card,
        "Tertiary colour",
        Some(
            "What's on or chosen: switches, tiles and ticks, sliders, today in the calendar, the \
             text cursor, toasts and the button Enter presses.",
        ),
        &colours.accent.button,
    );
}

/// A colour to choose: what the list calls it, what the settings file is given, and the colour.
type Choice = (String, String, slipstream_config::theme::Rgb);

/// The colours on offer under a followed theme: its own, chosen by name so the choice goes with
/// the next theme too.
fn colour_choices(theme: &slipstream_config::theme::Theme) -> Vec<Choice> {
    slipstream_config::theme::PICKS
        .iter()
        .filter_map(|(name, label)| {
            Some((
                (*label).to_string(),
                (*name).to_string(),
                theme.colour(name)?,
            ))
        })
        .collect()
}

/// Which of `choices` is in force: the one the file names (`chosen`), or failing that the one
/// that is the colour in force (`current`), since two of a theme's names can be one colour. A
/// colour that is none of them, a hand-edited one, joins the list as Custom rather than being
/// silently swapped for another.
fn with_current(
    choices: &mut Vec<Choice>,
    chosen: &str,
    current: slipstream_config::theme::Rgb,
) -> usize {
    let named = choices
        .iter()
        .position(|(_, value, _)| value.eq_ignore_ascii_case(chosen.trim()));
    named
        .or_else(|| choices.iter().position(|(.., colour)| *colour == current))
        .unwrap_or_else(|| {
            let hex = slipstream_config::theme::hex(current);
            choices.push(("Custom".to_string(), hex, current));
            choices.len() - 1
        })
}

/// A row that chooses one of `choices`, with a dot in the colour chosen. `chosen` is what the
/// file says and `current` the colour that comes to.
fn colour_row(
    card: &gtk::Box,
    store: &Store,
    title: &str,
    sub: &str,
    mut choices: Vec<Choice>,
    (chosen, current): (&str, slipstream_config::theme::Rgb),
    write: fn(&mut Settings, String),
) {
    let selected = with_current(&mut choices, chosen, current);
    let names: Vec<&str> = choices.iter().map(|(name, ..)| name.as_str()).collect();
    let list = gtk::DropDown::from_strings(&names);
    list.set_selected(selected as u32);

    let shown = Rc::new(Cell::new(current));
    let dot = gtk::DrawingArea::new();
    dot.set_content_width(20);
    dot.set_content_height(20);
    dot.set_valign(gtk::Align::Center);
    let painted = Rc::clone(&shown);
    dot.set_draw_func(move |_, cairo, width, height| {
        let [r, g, b] = painted.get().map(|channel| channel as f64 / 255.0);
        let radius = (width.min(height) as f64) / 2.0 - 1.0;
        cairo.arc(
            width as f64 / 2.0,
            height as f64 / 2.0,
            radius,
            0.0,
            std::f64::consts::TAU,
        );
        cairo.set_source_rgb(r, g, b);
        let _ = cairo.fill();
    });

    let store = store.clone();
    let dot_to_redraw = dot.clone();
    list.connect_selected_notify(move |list| {
        let Some((_, chosen, colour)) = choices.get(list.selected() as usize) else {
            return;
        };
        shown.set(*colour);
        dot_to_redraw.queue_draw();
        store.change(|settings| write(settings, chosen.clone()));
        // This window is drawn in the same three colours.
        crate::retheme();
    });

    let control = gtk::Box::builder().spacing(10).build();
    control.append(&dot);
    control.append(&list);
    row(card, title, Some(sub), &control);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_colours_are_the_first_theme() {
        assert_eq!(
            matching_theme(Colours::current_of(&Settings::default())),
            Some(0)
        );
    }

    #[test]
    fn colours_that_match_no_set_are_your_own() {
        let mut three = Colours::current_of(&Settings::default());
        three[2] = slipstream_config::rgb("#123456").unwrap();
        assert_eq!(matching_theme(three), None);
    }

    #[test]
    fn under_a_theme_the_colours_on_offer_are_the_themes_own_by_name() {
        let text = "background = \"#1a1b26\"\nforeground = \"#a9b1d6\"\naccent = \"#7aa2f7\"\n\
                    yellow = \"#e0af68\"\n";
        let theme = slipstream_config::theme::parse(text, false).unwrap();
        let mut choices = colour_choices(&theme);
        assert_eq!(choices.len(), slipstream_config::theme::PICKS.len());
        let borders = slipstream_config::Borders::default();
        let ring = with_current(&mut choices, "#42d3ff", borders.ring(Some(&theme)));
        assert_eq!(
            choices[ring].1, "accent",
            "what goes in the file is the name"
        );
        // This theme has no orange of its own, so its orange is its yellow: the name decides.
        let yellow = borders.highlight_colour(Some(&theme));
        let highlight = with_current(&mut choices, &borders.accent, yellow);
        assert_eq!(choices[highlight].0, "Yellow");
        assert_eq!(choices[highlight].2, [0xe0, 0xaf, 0x68]);
        assert_eq!(choices.len(), slipstream_config::theme::PICKS.len());
    }
}
