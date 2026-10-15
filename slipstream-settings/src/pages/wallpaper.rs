//! The living wallpaper: when it shows, and which variations take turns.

use super::*;
use crate::previews;

/// The waits offered before the UI fades to the living wallpaper, in seconds; 0 never fades.
pub(super) const FADE_CHOICES: [u64; 7] = [30, 60, 120, 300, 600, 1800, 0];

/// How long a variation holds before the next one takes over, in minutes; 0 never changes.
pub(super) const CHANGE_CHOICES: [u64; 8] = [1, 2, 5, 10, 15, 30, 60, 0];

pub(super) fn build(store: &Store) -> gtk::Widget {
    let page = page(
        "Wallpaper",
        "The living wallpaper behind your windows. Tick more than one and they take turns.",
    );

    let turns = group(&page, "Taking turns");
    let chosen: Vec<&str> = store
        .get()
        .wallpaper
        .chosen()
        .iter()
        .map(|variation| variation.id)
        .collect();
    let all = gtk::CheckButton::new();
    let all_sub = row(&turns, "All of them", Some(""), &all);
    let mut pictures = Vec::new();
    let mut checks: Vec<(&'static str, gtk::CheckButton)> = Vec::new();
    for (family, ids) in FAMILIES {
        let gallery = gallery(&page, family);
        for id in ids {
            let Some(variation) = slipstream_config::VARIATIONS.iter().find(|v| v.id == *id) else {
                continue;
            };
            let check = gtk::CheckButton::new();
            check.set_active(chosen.contains(&variation.id));
            let picture = previews::picture();
            picture.update_property(&[gtk::accessible::Property::Label(&format!(
                "Preview of {}",
                variation.title
            ))]);
            gallery.append(&variation_card(variation, &picture, &check));
            pictures.push((variation.id, picture));
            checks.push((variation.id, check));
        }
        // A card chosen anywhere but on its tick: the keyboard's Enter or Space, or a click on
        // the picture or the words.
        gallery.connect_child_activated(|_, child| {
            if let Some(check) = card_check(child) {
                check.set_active(!check.is_active());
            }
        });
    }
    let checks = Rc::new(checks);
    previews::fill(pictures, store.get().motion.reduced);

    // Setting ticks from here must not be taken for the user's own changes.
    let settling = Rc::new(Cell::new(false));
    // The "All of them" tick follows the others: ticked when every one is, half-ticked when some
    // are.
    let sync_all = {
        let (all, checks, settling) = (all.clone(), Rc::clone(&checks), Rc::clone(&settling));
        Rc::new(move || {
            let ticked = checks.iter().filter(|(_, check)| check.is_active()).count();
            let every = ticked == checks.len();
            settling.set(true);
            all.set_active(every);
            all.set_inconsistent(!every && ticked > 0);
            settling.set(false);
            if let Some(sub) = &all_sub {
                sub.set_label(&all_blurb(ticked, checks.len()));
            }
        })
    };
    sync_all();
    let save = {
        let (checks, store) = (Rc::clone(&checks), store.clone());
        move || {
            let ticked: Vec<&str> = checks
                .iter()
                .filter(|(_, check)| check.is_active())
                .map(|(id, _)| *id)
                .collect();
            store.change(|settings| {
                settings.wallpaper.variations =
                    ticked_list(&settings.wallpaper.variations, &ticked);
            });
        }
    };
    let save = Rc::new(save);
    // Unticking the last one is refused rather than leaving the desktop with no wallpaper.
    for (_, check) in checks.iter() {
        let (checks, settling, sync_all, save) = (
            Rc::clone(&checks),
            Rc::clone(&settling),
            Rc::clone(&sync_all),
            Rc::clone(&save),
        );
        check.connect_toggled(move |check| {
            if settling.get() {
                return;
            }
            if checks.iter().all(|(_, check)| !check.is_active()) {
                settling.set(true);
                check.set_active(true);
                settling.set(false);
                return;
            }
            save();
            sync_all();
        });
    }
    // Ticking "All of them" ticks every one. Unticking it, which can only happen when every one
    // is ticked, goes back to the first alone, since the set may never be empty.
    all.connect_toggled(move |all| {
        if settling.get() {
            return;
        }
        let ticks = all_ticks(all.is_active(), checks.len());
        settling.set(true);
        for ((_, check), tick) in checks.iter().zip(ticks) {
            check.set_active(tick);
        }
        settling.set(false);
        save();
        sync_all();
    });

    let cycle = group(&page, "Cycle");
    let current = store.get().wallpaper.change_every_mins;
    let every = choices(&CHANGE_CHOICES, current);
    let names: Vec<String> = every.iter().map(|&mins| describe(mins * 60)).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let change = gtk::DropDown::from_strings(&names);
    change.set_selected(every.iter().position(|&mins| mins == current).unwrap_or(0) as u32);
    let change_store = store.clone();
    change.connect_selected_notify(move |change| {
        if let Some(&mins) = every.get(change.selected() as usize) {
            change_store.change(|settings| settings.wallpaper.change_every_mins = mins);
        }
    });
    row(
        &cycle,
        "Change the picture every",
        Some(
            "With more than one ticked, they take turns in a random order. The next one takes \
             over at the end of a turn, so the screen is nearly empty when the picture changes.",
        ),
        &change,
    );

    let idle = group(&page, "Taking the screen");
    let current = store.get().wallpaper.fade_after_secs;
    let waits = choices(&FADE_CHOICES, current);
    let names: Vec<String> = waits.iter().map(|&secs| describe(secs)).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let fade = gtk::DropDown::from_strings(&names);
    fade.set_selected(waits.iter().position(|&secs| secs == current).unwrap_or(0) as u32);
    let fade_store = store.clone();
    fade.connect_selected_notify(move |fade| {
        if let Some(&secs) = waits.get(fade.selected() as usize) {
            fade_store.change(|settings| settings.wallpaper.fade_after_secs = secs);
        }
    });
    row(
        &idle,
        "Fade to the wallpaper after",
        Some(
            "Time with no keyboard or mouse input. A fullscreen window or a playing video keeps \
             the desktop up, and so does Awake: hold Caps Lock down to turn it on or off.",
        ),
        &fade,
    );
    page.upcast()
}

/// The ticks after "All of them" is pressed, for `count` variations: every one when it goes on,
/// and the first alone when it goes off, since the set may never be empty.
fn all_ticks(on: bool, count: usize) -> Vec<bool> {
    (0..count).map(|index| on || index == 0).collect()
}

/// What the "All of them" row says, with `ticked` of `count` variations ticked.
fn all_blurb(ticked: usize, count: usize) -> String {
    if ticked == count {
        format!(
            "Every one takes its turn, including any added later. Untick to keep only {}.",
            slipstream_config::VARIATIONS[0].title
        )
    } else if ticked == 1 {
        "One is ticked, so it stays. Tick more below and they take turns.".to_string()
    } else {
        format!("{ticked} of {count} take turns. Tick this to have every one take its turn.")
    }
}

/// The variations in families, so they are chosen among rather than scrolled past. Every
/// variation is in exactly one.
const FAMILIES: [(&str, &[&str]); 3] = [
    (
        "Air and speed",
        &[
            "slipstream",
            "vortex",
            "warp",
            "contours",
            "contrails",
            "tide",
        ],
    ),
    (
        "Nature and mathematics",
        &[
            "attractor",
            "galaxies",
            "physarum",
            "coral",
            "frost",
            "chladni",
            "murmuration",
            "life",
        ],
    ),
    (
        "Machines and signals",
        &["departures", "prompt", "circuit", "sonar", "glitch", "maze"],
    ),
];

/// A titled gallery of cards on `page`, as many across as fit.
fn gallery(page: &gtk::Box, title: &str) -> gtk::FlowBox {
    let group = gtk::Box::new(gtk::Orientation::Vertical, 7);
    group.append(&label(&title.to_uppercase(), "group-title"));
    let gallery = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .homogeneous(true)
        .min_children_per_line(2)
        .max_children_per_line(4)
        .column_spacing(12)
        .row_spacing(12)
        .activate_on_single_click(true)
        .css_classes(["gallery"])
        .build();
    group.append(&gallery);
    page.append(&group);
    gallery
}

/// A variation as a card: its moving picture with the tick over a corner, and its name and what
/// it does beneath. A chosen card is outlined, so what takes turns shows at a glance.
fn variation_card(
    variation: &slipstream_config::Variation,
    picture: &gtk::Picture,
    check: &gtk::CheckButton,
) -> gtk::FlowBoxChild {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .css_classes(["variation"])
        .build();
    let overlay = gtk::Overlay::builder().halign(gtk::Align::Start).build();
    overlay.set_child(Some(picture));
    check.set_halign(gtk::Align::End);
    check.set_valign(gtk::Align::Start);
    check.set_margin_top(8);
    check.set_margin_end(8);
    // The card takes the keyboard, so the tick is only for the pointer.
    check.set_focusable(false);
    overlay.add_overlay(check);
    card.append(&overlay);
    let title = label(variation.title, "variation-title");
    card.append(&title);
    let blurb = label(variation.blurb, "row-sub");
    blurb.set_lines(3);
    blurb.set_ellipsize(gtk::pango::EllipsizeMode::End);
    blurb.set_tooltip_text(Some(variation.blurb));
    card.append(&blurb);

    let child = gtk::FlowBoxChild::builder()
        .child(&card)
        .tooltip_text(variation.blurb)
        .build();
    child.update_property(&[
        gtk::accessible::Property::Label(variation.title),
        gtk::accessible::Property::Description(variation.blurb),
    ]);
    let mark = {
        let (card, child) = (card.clone(), child.clone());
        move |check: &gtk::CheckButton| {
            if check.is_active() {
                card.add_css_class("chosen");
            } else {
                card.remove_css_class("chosen");
            }
            child.update_state(&[gtk::accessible::State::Checked(if check.is_active() {
                gtk::AccessibleTristate::True
            } else {
                gtk::AccessibleTristate::False
            })]);
        }
    };
    mark(check);
    check.connect_toggled(mark);
    child
}

/// The tick on a card made by `variation_card`.
fn card_check(child: &gtk::FlowBoxChild) -> Option<gtk::CheckButton> {
    child
        .child()?
        .first_child()?
        .downcast::<gtk::Overlay>()
        .ok()?
        .last_child()?
        .downcast::<gtk::CheckButton>()
        .ok()
}

/// What to write for the ticked variations: nothing at all when every one of them is ticked, so
/// that goes on meaning "all of them" as more are added. Otherwise the ones still ticked keep the
/// order the file had them in, and any newly ticked follow in the order they are listed.
fn ticked_list(current: &[String], ticked: &[&str]) -> Vec<String> {
    if slipstream_config::VARIATIONS
        .iter()
        .all(|variation| ticked.contains(&variation.id))
    {
        return Vec::new();
    }
    // A replaced variation's old id is kept in its place under the id that replaced it.
    let mut list: Vec<String> = Vec::new();
    for id in current {
        let id = slipstream_config::current_id(id);
        if ticked.contains(&id) && !list.iter().any(|kept| kept == id) {
            list.push(id.to_string());
        }
    }
    for id in ticked {
        if !list.iter().any(|kept| kept == id) {
            list.push((*id).to_string());
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_of_them_ticks_every_one_or_goes_back_to_the_first() {
        let count = slipstream_config::VARIATIONS.len();
        let every = all_ticks(true, count);
        assert!(every.iter().all(|&tick| tick));
        let ids: Vec<&str> = slipstream_config::VARIATIONS.iter().map(|v| v.id).collect();
        assert_eq!(
            ticked_list(&["tide".to_string()], &ids),
            Vec::<String>::new()
        );
        let first = all_ticks(false, count);
        assert_eq!(first.iter().filter(|&&tick| tick).count(), 1, "never none");
        assert!(first[0]);
    }

    #[test]
    fn every_variation_ticked_is_written_as_no_list_at_all() {
        let all: Vec<&str> = slipstream_config::VARIATIONS
            .iter()
            .map(|variation| variation.id)
            .collect();
        assert_eq!(
            ticked_list(&["slipstream".to_string()], &all),
            Vec::<String>::new()
        );
    }

    #[test]
    fn ticking_keeps_the_order_the_file_had_and_adds_the_new_ones_after() {
        let current = ["vortex".to_string(), "slipstream".to_string()];
        assert_eq!(
            ticked_list(&current, &["slipstream", "vortex", "tide"]),
            ["vortex", "slipstream", "tide"],
            "the file's own order survives a tick"
        );
        assert_eq!(
            ticked_list(&current, &["vortex"]),
            ["vortex"],
            "unticking one leaves the rest alone"
        );
        // A replaced variation's id is written out as its replacement's, in the same place.
        let old = ["decode".to_string(), "slipstream".to_string()];
        assert_eq!(
            ticked_list(&old, &["slipstream", "vortex", "tide"]),
            ["vortex", "slipstream", "tide"]
        );
        // A file that names none means all of them, so ticking one off writes the rest out.
        let from_all: Vec<&str> = slipstream_config::VARIATIONS
            .iter()
            .map(|variation| variation.id)
            .filter(|id| *id != "glitch")
            .collect();
        assert_eq!(ticked_list(&[], &from_all).len(), from_all.len());
    }

    #[test]
    fn every_variation_is_in_one_family() {
        for variation in slipstream_config::VARIATIONS {
            let homes = FAMILIES
                .iter()
                .filter(|(_, ids)| ids.contains(&variation.id))
                .count();
            assert_eq!(homes, 1, "{} is in {homes} families", variation.id);
        }
        let listed: usize = FAMILIES.iter().map(|(_, ids)| ids.len()).sum();
        assert_eq!(listed, slipstream_config::VARIATIONS.len());
    }
}
