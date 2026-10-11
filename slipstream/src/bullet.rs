//! Bullet time's logic (Super+Alt+Tab): what can be chosen, the letter labels, and moving the
//! selection. Pure and generic over the window type, so it's unit-tested without Wayland.
//!
//! Bullet time uses the everyday keys without Super: arrows and Tab choose, 1–0 look at a
//! workspace, Shift+1–0 sends the chosen window there, PgUp/PgDn weigh it, M minimises it, Delete
//! closes it, Enter goes and Esc goes back. A letter label goes straight to its target.

use smithay::{
    input::keyboard::{Keysym, xkb},
    utils::{Logical, Point, Rectangle},
};

use crate::{
    bar,
    keys::{self, Mods},
    layout::{Direction, Rect},
};

/// Letters on the home row and just above it, easy to reach without looking.
pub const ALPHABET: &str = "jklfhuiont";

#[derive(Debug, Clone, PartialEq)]
pub enum Target<T> {
    Window(T),
    /// A window minimised into the code rain.
    Stream(T),
}

impl<T> Target<T> {
    pub fn window(&self) -> &T {
        match self {
            Target::Window(window) | Target::Stream(window) => window,
        }
    }
}

/// Bullet time while it's open.
#[derive(Debug, Clone)]
pub struct Mode<T> {
    /// The workspace it was opened on.
    pub home: usize,
    /// The workspace being looked at.
    pub view: usize,
    pub selected: Option<Target<T>>,
    pub labels: Vec<(Target<T>, String)>,
    /// Letters of a two-letter label typed so far.
    pub typed: String,
    /// What the pointer is doing in the overview.
    pub pointer: Pointer<T>,
}

/// The pointer in bullet time: the window it's over, and a press or drag under way. Positions are
/// in the overview's flat coordinates, before the tilt, where its hit areas are kept.
#[derive(Debug, Clone)]
pub struct Pointer<T> {
    /// The window under the pointer, which shows its close button, and whether the pointer is on
    /// that button.
    pub hover: Option<T>,
    pub on_close: bool,
    /// A left press on a window, or on its close button, waiting for its release, and where on
    /// screen it went down.
    pub press: Option<(Press<T>, Point<f64, Logical>)>,
    pub drag: Option<Drag<T>>,
}

impl<T> Default for Pointer<T> {
    fn default() -> Self {
        Self {
            hover: None,
            on_close: false,
            press: None,
            drag: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Press<T> {
    Window(T),
    Close(T),
}

/// A window being dragged to another workspace.
#[derive(Debug, Clone)]
pub struct Drag<T> {
    pub window: T,
    /// Where the pointer is, flat.
    pub at: Point<f64, Logical>,
    /// The workspace whose frame is under it.
    pub over: Option<usize>,
}

/// How far a press moves, in logical pixels, before it's a drag rather than a click.
pub const DRAG_START: f64 = 6.0;

/// Whether a press at `from` that has moved to `to` is a drag now.
pub fn is_drag(from: Point<f64, Logical>, to: Point<f64, Logical>) -> bool {
    (to.x - from.x).hypot(to.y - from.y) > DRAG_START
}

/// A window's close button in the overview, where the window is `shown` flat: 28 design pixels
/// square, 8 in from its top right corner.
pub fn close_button(shown: Rectangle<f64, Logical>) -> Rectangle<f64, Logical> {
    const DESIGN_PX: f64 = 0.8;
    let (size, inset) = (28.0 * DESIGN_PX, 8.0 * DESIGN_PX);
    Rectangle::new(
        (
            shown.loc.x + shown.size.w - inset - size,
            shown.loc.y + inset,
        )
            .into(),
        (size, size).into(),
    )
}

/// Everything that can be chosen, in label order: the home workspace's windows, then the code
/// rain's streams, then the other workspaces' windows. Callers pass windows most recent first.
pub fn targets<T>(home: Vec<T>, streams: Vec<T>, others: Vec<T>) -> Vec<Target<T>> {
    home.into_iter()
        .map(Target::Window)
        .chain(streams.into_iter().map(Target::Stream))
        .chain(others.into_iter().map(Target::Window))
        .collect()
}

/// One letter for each target when there are ten or fewer, otherwise two. While the same
/// windows are open the labels don't change, so they can be learned.
pub fn labels<T: Clone + PartialEq>(
    previous: &[(Target<T>, String)],
    targets: &[Target<T>],
) -> Vec<(Target<T>, String)> {
    let same_windows = previous.len() == targets.len()
        && targets.iter().all(|target| {
            previous
                .iter()
                .any(|(old, _)| old.window() == target.window())
        });
    if same_windows {
        return targets
            .iter()
            .filter_map(|target| {
                previous
                    .iter()
                    .find(|(old, _)| old.window() == target.window())
                    .map(|(_, label)| (target.clone(), label.clone()))
            })
            .collect();
    }
    let letters: Vec<char> = ALPHABET.chars().collect();
    let names: Vec<String> = if targets.len() <= letters.len() {
        letters.iter().map(char::to_string).collect()
    } else {
        letters
            .iter()
            .flat_map(|first| letters.iter().map(move |second| format!("{first}{second}")))
            .collect()
    };
    targets.iter().cloned().zip(names).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Typed<T> {
    /// No label starts with what was typed.
    NoMatch,
    /// The first letter of a two-letter label.
    Partial,
    Exact(Target<T>),
}

pub fn resolve<T: Clone>(labels: &[(Target<T>, String)], typed: &str) -> Typed<T> {
    if let Some((target, _)) = labels.iter().find(|(_, label)| label == typed) {
        return Typed::Exact(target.clone());
    }
    if labels.iter().any(|(_, label)| label.starts_with(typed)) {
        Typed::Partial
    } else {
        Typed::NoMatch
    }
}

/// The window an arrow reaches from `from`: the closest one straight across that way, or failing
/// that the nearest at an angle, with sideways distance counting extra.
pub fn nearest<T: Clone + PartialEq>(
    rects: &[(T, Rect)],
    from: &T,
    direction: Direction,
) -> Option<T> {
    let centre = |r: &Rect| (r.x as f64 + r.w as f64 / 2.0, r.y as f64 + r.h as f64 / 2.0);
    let (_, origin) = rects.iter().find(|(window, _)| window == from)?;
    let (ox, oy) = centre(origin);
    let horizontal = matches!(direction, Direction::Left | Direction::Right);
    // How much two rects share across the direction of travel.
    let overlap = |r: &Rect| {
        if horizontal {
            (r.y + r.h).min(origin.y + origin.h) - r.y.max(origin.y)
        } else {
            (r.x + r.w).min(origin.x + origin.w) - r.x.max(origin.x)
        }
    };
    let ahead: Vec<(&T, f64, f64, bool)> = rects
        .iter()
        .filter(|(window, _)| window != from)
        .filter_map(|(window, rect)| {
            let (x, y) = centre(rect);
            let (dx, dy) = (x - ox, y - oy);
            let (along, across) = match direction {
                Direction::Left => (-dx, dy),
                Direction::Right => (dx, dy),
                Direction::Up => (-dy, dx),
                Direction::Down => (dy, dx),
            };
            let score = dx.hypot(dy) + across.abs() * 1.5;
            (along > 20.0).then_some((window, along, score, overlap(rect) > 0))
        })
        .collect();
    // A window straight across wins, the closest first; otherwise the nearest at an angle. On a
    // tie the earlier rect wins, so callers list the most recently used first.
    let in_line = ahead
        .iter()
        .filter(|(.., overlapping)| *overlapping)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    in_line
        .or_else(|| ahead.iter().min_by(|a, b| a.2.total_cmp(&b.2)))
        .map(|(window, ..)| (*window).clone())
}

/// Arriving on a workspace from the side: its leftmost window when going right, its rightmost
/// when going left. On a tie the earlier rect wins, as in `nearest`.
pub fn entering<T: Clone>(rects: &[(T, Rect)], going_right: bool) -> Option<T> {
    rects
        .iter()
        .min_by_key(|(_, rect)| {
            if going_right {
                rect.x
            } else {
                -(rect.x + rect.w)
            }
        })
        .map(|(window, _)| window.clone())
}

/// Tab and Shift+Tab: the next or previous target, wrapping round.
pub fn cycle<T: Clone + PartialEq>(
    targets: &[Target<T>],
    current: Option<&Target<T>>,
    forward: bool,
) -> Option<Target<T>> {
    let count = targets.len();
    if count == 0 {
        return None;
    }
    let at = current.and_then(|current| {
        targets
            .iter()
            .position(|target| target.window() == current.window())
    });
    let index = match (at, forward) {
        (Some(i), true) => (i + 1) % count,
        (Some(i), false) => (i + count - 1) % count,
        (None, true) => 0,
        (None, false) => count - 1,
    };
    Some(targets[index].clone())
}

/// Which workspace the bar highlights, and which it rings as the one bullet time started on.
/// Outside bullet time that's the workspace the screen shows, with no ring. In bullet time the
/// highlight follows the workspace being looked at, on the keypress that moves the view, and home
/// is ringed while the view is elsewhere.
pub fn bar_workspaces<T>(showing: usize, mode: Option<&Mode<T>>) -> (usize, Option<usize>) {
    match mode {
        Some(mode) => (mode.view, (mode.home != mode.view).then_some(mode.home)),
        None => (showing, None),
    }
}

/// Where each workspace sits in bullet time, in workspace widths along the strip, when only the
/// `shown` ones are laid out. A shown workspace's place is its number among the shown ones, so
/// they sit side by side with nothing between them. The others are not drawn, and their places
/// are spread evenly through the gap they left, so a view sliding past them moves steadily.
///
/// `zoomed_out` runs from 0, where every workspace is where its own number puts it, to 1, where
/// the shown ones have closed up: the strip closes as the desktop tips back and opens as it
/// comes forward again.
pub fn places(shown: &[bool], zoomed_out: f64) -> Vec<f64> {
    let kept: Vec<usize> = (0..shown.len()).filter(|index| shown[*index]).collect();
    let (Some(&first), Some(&last)) = (kept.first(), kept.last()) else {
        return (0..shown.len()).map(|index| index as f64).collect();
    };
    (0..shown.len())
        .map(|index| {
            let closed = if index <= first {
                index as f64 - first as f64
            } else if index >= last {
                (kept.len() - 1) as f64 + (index - last) as f64
            } else {
                // Between the shown workspace before it and the one after.
                let after = kept.partition_point(|kept| *kept <= index);
                let (from, to) = (kept[after - 1], kept[after]);
                (after - 1) as f64 + (index - from) as f64 / (to - from) as f64
            };
            index as f64 + (closed - index as f64) * zoomed_out.clamp(0.0, 1.0)
        })
        .collect()
}

/// Where a view at `camera`, in workspaces and part way between two while it slides, sits among
/// `places`.
pub fn place_between(places: &[f64], camera: f64) -> f64 {
    let place = |index: usize| places.get(index).copied().unwrap_or(index as f64);
    let below = camera.floor().max(0.0);
    let (from, to) = (place(below as usize), place(below as usize + 1));
    from + (to - from) * (camera - below)
}

/// The next shown workspace to the right of `from`, or to the left.
pub fn neighbour(shown: &[bool], from: usize, right: bool) -> Option<usize> {
    let is_shown = |index: &usize| shown.get(*index).copied().unwrap_or(false);
    if right {
        (from + 1..shown.len()).find(is_shown)
    } else {
        (0..from.min(shown.len())).rev().find(is_shown)
    }
}

/// "empty", "1 window" or "3 windows": how full a workspace is, as bullet time says it.
pub fn window_count(count: usize) -> String {
    match count {
        0 => "empty".to_string(),
        1 => "1 window".to_string(),
        n => format!("{n} windows"),
    }
}

/// What the bar's title says in bullet time. With a window chosen, its name, marked when it's in
/// the code rain; with nothing chosen, the workspace being looked at and how many windows it
/// holds. `label` is the workspace's name, or its number when it has none.
pub fn bar_title(
    chosen: Option<(&str, bool)>,
    label: &str,
    index: usize,
    windows: usize,
) -> String {
    if let Some((name, minimised)) = chosen {
        return if minimised {
            format!("{name} · minimised")
        } else {
            name.to_string()
        };
    }
    let workspace = if label == (index + 1).to_string() {
        format!("Workspace {label}")
    } else {
        label.to_string()
    };
    format!("{workspace} · {}", window_count(windows))
}

/// What a click on the bar does while bullet time is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarClick {
    /// A workspace's number: look at that workspace, as its number key does.
    Look(usize),
    /// A panel's button: go back out of bullet time with nothing changed, then open the panel.
    Leave,
    /// Stopping a share or Awake needs nothing on screen to change, so the overview stays.
    Stay,
    /// The overview button: back out of bullet time, as Super+Alt+Tab does.
    Back,
}

pub fn bar_click(target: bar::Target) -> BarClick {
    match target {
        bar::Target::Workspace(index) => BarClick::Look(index),
        bar::Target::Sharing | bar::Target::Awake => BarClick::Stay,
        bar::Target::Overview => BarClick::Back,
        bar::Target::Apps
        | bar::Target::Clock
        | bar::Target::Tray
        | bar::Target::Meter
        | bar::Target::Dock
        | bar::Target::Bell => BarClick::Leave,
    }
}

/// What a key does in bullet time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Move the selection.
    Choose(Direction),
    /// Every window and stream in turn.
    Cycle {
        forward: bool,
    },
    /// A letter of a label.
    Jump(char),
    /// Look at a workspace, by index.
    Look(usize),
    Go,
    /// Send the chosen window to a workspace, by index.
    Send(usize),
    /// Send the chosen window to the workspace beside, left or right.
    SendBeside(Direction),
    Weigh {
        heavier: bool,
    },
    Minimise,
    Close,
    /// Out with nothing changed, or first a half-typed label cleared.
    Back,
}

/// A command without what it's aimed at, to name a row of keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Choose,
    Cycle,
    Jump,
    Look,
    Go,
    Send,
    SendBeside,
    Weigh,
    Minimise,
    Close,
    Back,
}

impl Command {
    // Read by the test that keeps `KEYS` and `command` in step.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn kind(self) -> Kind {
        match self {
            Command::Choose(_) => Kind::Choose,
            Command::Cycle { .. } => Kind::Cycle,
            Command::Jump(_) => Kind::Jump,
            Command::Look(_) => Kind::Look,
            Command::Go => Kind::Go,
            Command::Send(_) => Kind::Send,
            Command::SendBeside(_) => Kind::SendBeside,
            Command::Weigh { .. } => Kind::Weigh,
            Command::Minimise => Kind::Minimise,
            Command::Close => Kind::Close,
            Command::Back => Kind::Back,
        }
    }
}

/// The key `key` pressed with `mods` in bullet time: the one reading of the keys, which
/// `bullet_key` acts on and the shortcut sheet and the legend describe through `KEYS`.
pub fn command(key: Keysym, mods: Mods) -> Option<Command> {
    let digit = keys::DIGITS.iter().position(|digit| *digit == key);
    Some(match key {
        Keysym::Escape => Command::Back,
        Keysym::Tab if mods.logo => Command::Back,
        Keysym::Return | Keysym::KP_Enter => Command::Go,
        Keysym::Tab => Command::Cycle {
            forward: !mods.shift,
        },
        Keysym::ISO_Left_Tab => Command::Cycle { forward: false },
        Keysym::Left if mods.shift => Command::SendBeside(Direction::Left),
        Keysym::Right if mods.shift => Command::SendBeside(Direction::Right),
        Keysym::Left => Command::Choose(Direction::Left),
        Keysym::Right => Command::Choose(Direction::Right),
        Keysym::Up => Command::Choose(Direction::Up),
        Keysym::Down => Command::Choose(Direction::Down),
        Keysym::Prior => Command::Weigh { heavier: true },
        Keysym::Next => Command::Weigh { heavier: false },
        Keysym::Delete => Command::Close,
        Keysym::m => Command::Minimise,
        _ => match digit {
            Some(index) if mods.shift => Command::Send(index),
            Some(index) => Command::Look(index),
            None => Command::Jump(
                xkb::keysym_to_utf8(key)
                    .chars()
                    .next()
                    .filter(|letter| ALPHABET.contains(*letter))?,
            ),
        },
    })
}

/// A row of bullet time's keys: what kind of command, the keys as a keycap, what they do, the
/// legend's words for them if the legend shows them, and one key (with Shift or not) that does it.
pub struct KeyRow {
    // `kind` and `example` are read by the test that keeps this table and `command` in step.
    #[cfg_attr(not(test), allow(dead_code))]
    pub kind: Kind,
    pub keys: &'static str,
    pub does: &'static str,
    pub legend: Option<&'static str>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub example: (Keysym, bool),
}

/// Bullet time's keys, in the order the legend and the shortcut sheet list them.
pub const KEYS: [KeyRow; 11] = [
    KeyRow {
        kind: Kind::Choose,
        keys: "← → ↑ ↓",
        does: "choose a window",
        legend: Some("← → ↑ ↓ choose"),
        example: (Keysym::Up, false),
    },
    KeyRow {
        kind: Kind::Cycle,
        keys: "Tab",
        does: "every window in turn",
        legend: None,
        example: (Keysym::Tab, false),
    },
    KeyRow {
        kind: Kind::Jump,
        keys: "J K L…",
        does: "jump to a label",
        legend: Some("j k l… jump"),
        example: (Keysym::j, false),
    },
    KeyRow {
        kind: Kind::Look,
        keys: "1–0",
        does: "look at a workspace",
        legend: None,
        example: (Keysym::_2, false),
    },
    KeyRow {
        kind: Kind::Go,
        keys: "⏎",
        does: "go",
        legend: Some("⏎ go"),
        example: (Keysym::Return, false),
    },
    KeyRow {
        kind: Kind::Send,
        keys: "Shift+1–0",
        does: "send the window to a workspace",
        legend: Some("Shift+1–0 send"),
        example: (Keysym::_3, true),
    },
    KeyRow {
        kind: Kind::SendBeside,
        keys: "Shift+← →",
        does: "send it to the workspace beside",
        legend: None,
        example: (Keysym::Right, true),
    },
    KeyRow {
        kind: Kind::Weigh,
        keys: "PgUp PgDn",
        does: "heavier, lighter",
        legend: Some("PgUp/PgDn weigh"),
        example: (Keysym::Next, false),
    },
    KeyRow {
        kind: Kind::Minimise,
        keys: "M",
        does: "minimise",
        legend: Some("M minimise"),
        example: (Keysym::m, false),
    },
    KeyRow {
        kind: Kind::Close,
        keys: "Del",
        does: "close",
        legend: Some("Del close"),
        example: (Keysym::Delete, false),
    },
    KeyRow {
        kind: Kind::Back,
        keys: "Esc",
        does: "back, with nothing changed",
        legend: Some("Esc back"),
        example: (Keysym::Escape, false),
    },
];

/// The legend along the foot of the screen in bullet time.
pub fn legend() -> String {
    KEYS.iter()
        .filter_map(|row| row.legend)
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_the_shown_workspaces_are_laid_out_side_by_side() {
        // Workspaces 1, 4 and 5 hold windows; 2 and 3 are empty.
        let shown = [true, false, false, true, true];
        let closed = places(&shown, 1.0);
        assert_eq!(closed[0], 0.0);
        assert_eq!(closed[3], 1.0, "the fourth is beside the first");
        assert_eq!(closed[4], 2.0);
        // The empty ones are spread through the gap, so a view sliding across moves steadily.
        assert!((closed[1] - 1.0 / 3.0).abs() < 1e-9 && (closed[2] - 2.0 / 3.0).abs() < 1e-9);
        // Not zoomed out, every workspace is where its number puts it, and half way it's between.
        assert_eq!(places(&shown, 0.0), [0.0, 1.0, 2.0, 3.0, 4.0]);
        assert_eq!(places(&shown, 0.5)[3], 2.0);
        // Empty workspaces at either end keep their distance beyond the shown ones.
        let ends = places(&[false, true, false, true, false, false], 1.0);
        assert_eq!(ends, [-1.0, 0.0, 0.5, 1.0, 2.0, 3.0]);
        // With none shown there is nothing to close up.
        assert_eq!(places(&[false, false], 1.0), [0.0, 1.0]);
    }

    #[test]
    fn a_sliding_view_sits_between_the_places_of_the_workspaces_either_side() {
        let closed = places(&[true, false, false, true], 1.0);
        assert_eq!(place_between(&closed, 0.0), 0.0);
        assert_eq!(place_between(&closed, 3.0), 1.0);
        assert!((place_between(&closed, 1.5) - 0.5).abs() < 1e-9);
        // Past the last workspace, or with no places at all, it's the workspace's own number.
        assert_eq!(place_between(&[], 2.5), 2.5);
    }

    #[test]
    fn the_arrows_step_over_workspaces_that_arent_shown() {
        let shown = [true, false, false, true, false];
        assert_eq!(neighbour(&shown, 0, true), Some(3));
        assert_eq!(neighbour(&shown, 3, false), Some(0));
        assert_eq!(neighbour(&shown, 3, true), None);
        assert_eq!(neighbour(&shown, 0, false), None);
        // From a workspace that isn't shown itself, the nearest shown one either way.
        assert_eq!(neighbour(&shown, 2, true), Some(3));
        assert_eq!(neighbour(&shown, 2, false), Some(0));
    }

    #[test]
    fn the_legend_and_the_keys_agree() {
        assert_eq!(
            legend(),
            "← → ↑ ↓ choose · j k l… jump · ⏎ go · Shift+1–0 send · PgUp/PgDn weigh · M minimise · Del close · Esc back"
        );
        for row in &KEYS {
            let (key, shift) = row.example;
            let mods = Mods {
                shift,
                ..Mods::default()
            };
            assert_eq!(
                command(key, mods).map(Command::kind),
                Some(row.kind),
                "{}",
                row.keys
            );
        }
        // Every kind of command has its row, once.
        for kind in [
            Kind::Choose,
            Kind::Cycle,
            Kind::Jump,
            Kind::Look,
            Kind::Go,
            Kind::Send,
            Kind::SendBeside,
            Kind::Weigh,
            Kind::Minimise,
            Kind::Close,
            Kind::Back,
        ] {
            assert_eq!(KEYS.iter().filter(|row| row.kind == kind).count(), 1);
        }
        assert_eq!(command(Keysym::z, Mods::default()), None);
    }

    use super::*;

    fn mode(home: usize, view: usize) -> Mode<&'static str> {
        Mode {
            home,
            view,
            selected: None,
            labels: Vec::new(),
            typed: String::new(),
            pointer: Pointer::default(),
        }
    }

    #[test]
    fn the_close_button_is_inside_its_window() {
        let shown = Rectangle::new((100.0, 50.0).into(), (300.0, 200.0).into());
        let close = close_button(shown);
        assert!(shown.contains(close.loc));
        let corner: Point<f64, Logical> =
            Point::from((close.loc.x + close.size.w, close.loc.y + close.size.h));
        assert!(corner.x <= shown.loc.x + shown.size.w && corner.y <= shown.loc.y + shown.size.h);
        assert!(
            close.loc.x > shown.loc.x + shown.size.w / 2.0,
            "at the right"
        );
        assert!(close.loc.y < shown.loc.y + shown.size.h / 2.0, "at the top");
    }

    #[test]
    fn a_short_move_is_a_click_not_a_drag() {
        let from: Point<f64, Logical> = Point::from((10.0, 10.0));
        assert!(!is_drag(from, Point::from((14.0, 13.0))), "5 px");
        assert!(!is_drag(from, Point::from((16.0, 10.0))), "exactly 6 px");
        assert!(is_drag(from, Point::from((16.5, 10.0))));
        assert_eq!(bar_click(bar::Target::Overview), BarClick::Back);
    }

    #[test]
    fn the_bar_follows_the_view_and_rings_home_only_while_away_from_it() {
        assert_eq!(bar_workspaces::<&str>(2, None), (2, None));
        assert_eq!(bar_workspaces(0, Some(&mode(0, 0))), (0, None));
        assert_eq!(bar_workspaces(0, Some(&mode(0, 3))), (3, Some(0)));
        // The screen's own workspace doesn't matter once bullet time is open: home does.
        assert_eq!(bar_workspaces(4, Some(&mode(1, 2))), (2, Some(1)));
    }

    #[test]
    fn the_bar_title_names_the_choice_or_the_workspace() {
        assert_eq!(bar_title(Some(("Notes.md", false)), "2", 1, 3), "Notes.md");
        assert_eq!(
            bar_title(Some(("Music", true)), "2", 1, 3),
            "Music · minimised"
        );
        assert_eq!(bar_title(None, "4", 3, 0), "Workspace 4 · empty");
        assert_eq!(bar_title(None, "Mail", 0, 1), "Mail · 1 window");
        assert_eq!(bar_title(None, "3", 2, 2), "Workspace 3 · 2 windows");
    }

    #[test]
    fn bar_clicks_look_leave_or_stay() {
        assert_eq!(bar_click(bar::Target::Workspace(3)), BarClick::Look(3));
        for panel in [
            bar::Target::Apps,
            bar::Target::Clock,
            bar::Target::Tray,
            bar::Target::Meter,
            bar::Target::Bell,
        ] {
            assert_eq!(bar_click(panel), BarClick::Leave, "{panel:?}");
        }
        assert_eq!(bar_click(bar::Target::Sharing), BarClick::Stay);
        assert_eq!(bar_click(bar::Target::Awake), BarClick::Stay);
    }

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn home_windows_come_first_then_streams_then_the_rest() {
        let order = targets(vec!["a", "b"], vec!["r"], vec!["c"]);
        assert_eq!(
            order,
            [
                Target::Window("a"),
                Target::Window("b"),
                Target::Stream("r"),
                Target::Window("c")
            ]
        );
    }

    #[test]
    fn labels_are_single_letters_until_there_are_too_many_and_stay_put() {
        let few = targets(vec!["a", "b", "c"], vec![], vec![]);
        let first = labels(&[], &few);
        assert_eq!(first[0].1, "j");
        assert_eq!(first[2].1, "l");

        // The same windows in a different order keep their letters.
        let shuffled = targets(vec!["c", "a", "b"], vec![], vec![]);
        let again = labels(&first, &shuffled);
        assert!(again.contains(&(Target::Window("a"), "j".to_string())));

        // A window minimised to a stream keeps its letter too.
        let minimised = targets(vec!["a", "b"], vec!["c"], vec![]);
        assert!(labels(&first, &minimised).contains(&(Target::Stream("c"), "l".to_string())));

        let names: Vec<String> = (0..12).map(|i| format!("w{i}")).collect();
        let many = targets(names.iter().map(String::as_str).collect(), vec![], vec![]);
        let pairs = labels(&[], &many);
        assert!(pairs.iter().all(|(_, label)| label.len() == 2));
    }

    #[test]
    fn typing_a_label_finds_its_target() {
        let names: Vec<String> = (0..12).map(|i| format!("w{i}")).collect();
        let many = labels(
            &[],
            &targets(names.iter().map(String::as_str).collect(), vec![], vec![]),
        );
        assert_eq!(resolve(&many, "j"), Typed::Partial);
        assert_eq!(resolve(&many, "jk"), Typed::Exact(Target::Window("w1")));
        assert_eq!(resolve(&many, "z"), Typed::NoMatch);
    }

    #[test]
    fn arrows_prefer_the_window_in_line() {
        // a | b on top, c under b.
        let rects = [
            ("a", rect(0, 0, 500, 1000)),
            ("b", rect(510, 0, 500, 490)),
            ("c", rect(510, 500, 500, 500)),
        ];
        assert_eq!(nearest(&rects, &"a", Direction::Right), Some("b"));
        assert_eq!(nearest(&rects, &"b", Direction::Down), Some("c"));
        assert_eq!(nearest(&rects, &"c", Direction::Left), Some("a"));
        assert_eq!(nearest(&rects, &"a", Direction::Left), None);
        assert_eq!(entering(&rects, true), Some("a"));
        assert_eq!(entering(&rects, false), Some("b"));
    }

    #[test]
    fn tab_wraps_round_the_targets() {
        let all = targets(vec!["a", "b"], vec!["r"], vec![]);
        let last = Target::Stream("r");
        assert_eq!(cycle(&all, Some(&last), true), Some(Target::Window("a")));
        assert_eq!(cycle(&all, None, false), Some(last));
        assert_eq!(cycle::<&str>(&[], None, true), None);
    }
}
