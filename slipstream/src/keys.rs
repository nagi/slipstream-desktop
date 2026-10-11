//! Compositor key bindings.
//!
//! The defaults and the reserved list follow one rule: a compositor binding is global and reaches
//! us before the focused app, so anything the kernel, input methods, screen readers, games or
//! ordinary apps rely on must never be bound.

use smithay::input::keyboard::{Keysym, ModifiersState, xkb};

use crate::{
    copypaste::Edit,
    layout::{Direction, Resize},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Open the way out: lock, log out, restart or shut down, chosen on a card.
    WayOut,
    /// Move keyboard focus to the neighbouring tiled window.
    Focus(Direction),
    /// Swap the focused window with the tiled window in that direction, so a tile can be moved
    /// around the workspace from the keyboard.
    MoveTile(Direction),
    /// Move the split beside the focused tile: wider or narrower, taller or shorter.
    Resize(Resize),
    /// Turn the layout a quarter, clockwise or anticlockwise.
    Rotate { clockwise: bool },
    /// Go to a workspace by its number; the number keys reach 1–9.
    Workspace(u8),
    /// Go to the previous (−1) or next (+1) workspace.
    WorkspaceBy(i8),
    /// Move the focused window to a workspace by its number (1–9 by key) and follow it.
    MoveToWorkspace(u8),
    /// Move the focused window to the previous or next workspace and follow it.
    MoveToWorkspaceBy(i8),
    /// Ask the focused window to close.
    Close,
    /// Start an app.
    Launch(App),
    /// Go to virtual terminal 1–12. Never a binding: Ctrl+Alt+F1–F12 produce it through the
    /// keyboard layout, on real hardware only.
    SwitchVt(i32),
    /// Volume up or down by this many percent.
    Volume(i8),
    /// Mute or unmute the speakers, or the microphone.
    ToggleMute { microphone: bool },
    /// Screen brightness up or down by this many percent.
    Brightness(i8),
    /// Alt+Tab: focus the next (or, with Shift, the previous) most recently used window.
    CycleWindows { forward: bool },
    /// Open or close the app explorer.
    Explorer,
    /// Gravity on, at the arrangement last kept, or back to tiling; held, the arrangements side by
    /// side.
    ToggleGravity,
    /// Minimise the focused window into the code rain.
    Minimise,
    /// Bring back the most recently minimised window.
    Restore,
    /// Put every window on this workspace into the code rain; again, bring those same windows
    /// back. Windows' own "minimise all", which Super+M only does one window at a time.
    HideAll,
    /// Send the focused window up to the bar, where it hangs in front of every workspace; on
    /// the window already there, its next size.
    Dock,
    /// Bring the docked window down from the bar, a standard window again.
    Undock,
    /// Open bullet time, the overview of every workspace, or go back out of it.
    BulletTime,
    /// Open or close quick settings.
    QuickSettings,
    /// Open or close the notification centre.
    NotificationCentre,
    /// Move the keyboard to the next screen along.
    NextScreen,
    /// Send the focused window to the next screen along; the keyboard stays where it is.
    MoveToNextScreen,
    /// Trade workspaces with the next screen along.
    SwapScreens,
    /// The focused tiled window fills the tiling area above its neighbours, or goes back.
    Maximise,
    /// Save the focused screen, or the focused window, as a PNG and copy it.
    Screenshot { window: bool },
    /// Go to an empty workspace, or back from it.
    ShowDesktop,
    /// Open or close the shortcut sheet: every key, listed.
    ShortcutSheet,
    /// Lock the screen.
    Lock,
    /// A media key: to whichever player is playing.
    Media(crate::media::Transport),
    /// Take the pointer and the keyboard shortcuts back from the app holding them.
    TakeBack,
    /// Open or close the clipboard history.
    ClipboardHistory,
    /// Copy, cut or paste in the focused app, with whichever keys it takes for that.
    Edit(Edit),
    /// Float the focused window, or put it back into the tiling.
    ToggleFloating,
    /// Pop the focused window out to float on whichever workspace its screen shows, or put it
    /// back into the tiling.
    Pop,
    /// Move the keyboard between the floating windows and the tiles.
    SwitchFloatingFocus,
    /// Run or raise the `[[run-or-raise.app]]` at this index: go to its workspace and bring its
    /// window forward, or start it there.
    RunOrRaise(u8),
    /// Send the focused window to that app's workspace, and follow it.
    SendTo(u8),
}

/// The shortcut sheet's groups, in the order it shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    AppsAndWindows,
    WorkspacesAndArranging,
    BulletTime,
    PanelsAndSystem,
}

impl Group {
    pub const ALL: [Group; 4] = [
        Group::AppsAndWindows,
        Group::WorkspacesAndArranging,
        Group::BulletTime,
        Group::PanelsAndSystem,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Group::AppsAndWindows => "Apps and windows",
            Group::WorkspacesAndArranging => "Workspaces and arranging",
            Group::BulletTime => "Bullet time",
            Group::PanelsAndSystem => "Panels and system",
        }
    }
}

impl Action {
    /// What the action does, as the shortcut sheet says it. Actions that are two directions of
    /// one thing say both, so their keys share a row.
    pub fn describe(self) -> &'static str {
        match self {
            Action::WayOut => "lock, log out, restart, shut down",
            Action::Focus(_) => "move focus",
            Action::MoveTile(_) => "move the window",
            Action::Resize(Resize::Wider | Resize::Narrower) => "narrower, wider (gravity: weight)",
            Action::Resize(Resize::Taller | Resize::Shorter) => "shorter, taller",
            Action::Rotate { .. } => "turn the layout a quarter, either way",
            Action::Workspace(_) => "go to a workspace",
            Action::WorkspaceBy(_) => "previous, next workspace",
            Action::MoveToWorkspace(_) => "send window to a workspace",
            Action::MoveToWorkspaceBy(_) => "send window to previous, next",
            Action::Close => "close window",
            Action::Launch(App::Terminal) => "terminal",
            Action::Launch(App::Files) => "files",
            Action::Launch(App::Settings) => "settings",
            Action::Launch(App::Browser) => "browser",
            Action::SwitchVt(_) => "virtual terminal",
            Action::Volume(_) => "volume up, down",
            Action::ToggleMute { microphone: false } => "mute",
            Action::ToggleMute { microphone: true } => "mute the microphone",
            Action::Brightness(_) => "brightness up, down",
            Action::CycleWindows { .. } => "switch window",
            Action::Explorer => "apps",
            Action::ToggleGravity => "tiling or gravity; hold to choose",
            Action::Minimise => "minimise to a stream",
            Action::Restore => "bring back the last minimised",
            Action::HideAll => "hide every window, and back",
            Action::Dock => "dock the window to the bar, then its next size",
            Action::Undock => "bring the docked window back down",
            Action::BulletTime => "bullet time",
            Action::QuickSettings => "quick settings",
            Action::NotificationCentre => "notifications",
            Action::NextScreen => "keyboard to the next screen",
            Action::MoveToNextScreen => "send window to the next screen",
            Action::SwapScreens => "swap workspaces with the next screen",
            Action::Maximise => "maximise",
            Action::Screenshot { window: false } => "screenshot",
            Action::Screenshot { window: true } => "snip a region or a window",
            Action::ShowDesktop => "an empty workspace, and back",
            Action::ShortcutSheet => "every key",
            Action::Lock => "lock the screen",
            Action::Media(_) => "play, pause, next, previous",
            Action::TakeBack => "take the mouse and keys back from an app",
            Action::ClipboardHistory => "clipboard history",
            Action::Edit(Edit::Copy) => "copy, in any app",
            Action::Edit(Edit::Cut) => "cut, in any app",
            Action::Edit(Edit::Paste) => "paste, in any app",
            Action::ToggleFloating => "float the window, or tile it",
            Action::Pop => "pop the window out, to follow you",
            Action::SwitchFloatingFocus => "between floating and tiled windows",
            Action::RunOrRaise(_) => "go to an app, or start it",
            Action::SendTo(_) => "send window to an app's workspace",
        }
    }

    /// Where the shortcut sheet lists it.
    pub fn group(self) -> Group {
        match self {
            Action::Focus(_)
            | Action::MoveTile(_)
            | Action::Close
            | Action::Launch(_)
            | Action::CycleWindows { .. }
            | Action::Explorer
            | Action::Minimise
            | Action::Restore
            | Action::HideAll
            | Action::Dock
            | Action::Undock
            | Action::Maximise
            | Action::ToggleFloating
            | Action::Pop
            | Action::SwitchFloatingFocus
            | Action::Edit(_)
            | Action::RunOrRaise(_) => Group::AppsAndWindows,
            Action::Resize(_)
            | Action::Rotate { .. }
            | Action::Workspace(_)
            | Action::WorkspaceBy(_)
            | Action::MoveToWorkspace(_)
            | Action::MoveToWorkspaceBy(_)
            | Action::ToggleGravity
            | Action::NextScreen
            | Action::MoveToNextScreen
            | Action::SwapScreens
            | Action::ShowDesktop
            | Action::SendTo(_) => Group::WorkspacesAndArranging,
            Action::BulletTime => Group::BulletTime,
            Action::WayOut
            | Action::SwitchVt(_)
            | Action::Volume(_)
            | Action::ToggleMute { .. }
            | Action::Brightness(_)
            | Action::QuickSettings
            | Action::NotificationCentre
            | Action::Screenshot { .. }
            | Action::ShortcutSheet
            | Action::Lock
            | Action::Media(_)
            | Action::TakeBack
            | Action::ClipboardHistory => Group::PanelsAndSystem,
        }
    }
}

/// A key's name as people write it: `Esc`, `PgUp`, `←`, `]`, `Volume Up`, `A`.
pub fn key_name(key: Keysym) -> String {
    let name = match key {
        Keysym::Escape => "Esc",
        Keysym::Return | Keysym::KP_Enter => "Enter",
        Keysym::space => "Space",
        Keysym::Tab | Keysym::ISO_Left_Tab => "Tab",
        Keysym::Left => "←",
        Keysym::Right => "→",
        Keysym::Up => "↑",
        Keysym::Down => "↓",
        Keysym::Prior => "PgUp",
        Keysym::Next => "PgDn",
        Keysym::Delete => "Del",
        Keysym::BackSpace => "Backspace",
        Keysym::Print => "Print",
        Keysym::slash => "/",
        Keysym::bracketleft | Keysym::braceleft => "[",
        Keysym::bracketright | Keysym::braceright => "]",
        Keysym::minus | Keysym::underscore => "-",
        Keysym::equal | Keysym::plus => "=",
        Keysym::XF86_AudioRaiseVolume => "Volume Up",
        Keysym::XF86_AudioLowerVolume => "Volume Down",
        Keysym::XF86_AudioMute => "Mute",
        Keysym::XF86_AudioMicMute => "Mic Mute",
        Keysym::XF86_AudioPlay => "Play",
        Keysym::XF86_AudioPause => "Pause",
        Keysym::XF86_AudioNext => "Next Track",
        Keysym::XF86_AudioPrev => "Previous Track",
        Keysym::XF86_AudioStop => "Stop",
        Keysym::XF86_MonBrightnessUp => "Brightness Up",
        Keysym::XF86_MonBrightnessDown => "Brightness Down",
        _ => {
            let typed = xkb::keysym_to_utf8(key);
            let mut chars = typed.chars().filter(|ch| !ch.is_control());
            return match (chars.next(), chars.next()) {
                (Some(ch), None) => ch.to_uppercase().collect(),
                _ => xkb::keysym_get_name(key),
            };
        }
    };
    name.to_string()
}

/// The modifiers as a prefix, `Super+Ctrl+Alt+Shift+`, in the order people say them.
pub fn mods_prefix(mods: Mods) -> String {
    let mut prefix = String::new();
    for (held, name) in [
        (mods.logo, "Super+"),
        (mods.ctrl, "Ctrl+"),
        (mods.alt, "Alt+"),
        (mods.shift, "Shift+"),
    ] {
        if held {
            prefix.push_str(name);
        }
    }
    prefix
}

/// A key combination for prose, toasts and keycaps: `Super+Shift+S`.
pub fn label(mods: Mods, key: Keysym) -> String {
    mods_prefix(mods) + &key_name(key)
}

/// Apps with their own keys, Windows-style. `launch.rs` picks whichever is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum App {
    Terminal,
    /// Super+E, as on Windows.
    Files,
    /// Super+I, as on Windows: Slipstream's own settings app.
    Settings,
    /// Super+B: whichever browser the desktop is set to open a web page with.
    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// Slipstream's Mod key: Super on real hardware (see `from_state`).
    pub logo: bool,
}

impl Mods {
    /// Nested inside another desktop, that desktop keeps Super for itself (KDE binds
    /// Super+Shift+Esc, for one), so Alt stands in for Super.
    pub fn from_state(m: &ModifiersState, nested: bool) -> Self {
        let mut mods = Self {
            ctrl: m.ctrl,
            alt: m.alt,
            shift: m.shift,
            logo: m.logo,
        };
        if nested && mods.alt && !mods.logo {
            mods.alt = false;
            mods.logo = true;
        }
        mods
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    pub mods: Mods,
    pub key: Keysym,
    pub action: Action,
}

pub fn defaults() -> Vec<Binding> {
    let mod_ = Mods {
        logo: true,
        ..Mods::default()
    };
    let mod_shift = Mods {
        shift: true,
        ..mod_
    };
    let mod_ctrl = Mods { ctrl: true, ..mod_ };
    let mod_alt = Mods { alt: true, ..mod_ };
    let alt = Mods {
        alt: true,
        ..Mods::default()
    };
    let ctrl_alt = Mods {
        ctrl: true,
        alt: true,
        ..Mods::default()
    };
    let alt_shift = Mods { shift: true, ..alt };
    let mod_ctrl_shift = Mods {
        shift: true,
        ..mod_ctrl
    };
    let bind = |mods, key, action| Binding { mods, key, action };
    // Where Omarchy has the same action, it has Omarchy's key, so hands used to Hyprland under
    // Omarchy find it where they reach. Slipstream's own ideas fit around them.
    let mut bindings = vec![
        // Windows' own security screen, on the chord every Windows user already reaches for
        // under stress. The kernel never sees it: libinput hands it to us first. Super+Esc is
        // Omarchy's system menu, the same card.
        bind(ctrl_alt, Keysym::Delete, Action::WayOut),
        bind(mod_, Keysym::Escape, Action::WayOut),
        // A game holding the pointer, or a virtual machine holding every key, gives them back.
        bind(mod_shift, Keysym::Escape, Action::TakeBack),
        // Super+arrows move the focus to the neighbouring tile.
        bind(mod_, Keysym::Left, Action::Focus(Direction::Left)),
        bind(mod_, Keysym::Right, Action::Focus(Direction::Right)),
        bind(mod_, Keysym::Up, Action::Focus(Direction::Up)),
        bind(mod_, Keysym::Down, Action::Focus(Direction::Down)),
        // Add Shift and the window comes with the focus: the tile swaps places with its
        // neighbour.
        bind(mod_shift, Keysym::Left, Action::MoveTile(Direction::Left)),
        bind(mod_shift, Keysym::Right, Action::MoveTile(Direction::Right)),
        bind(mod_shift, Keysym::Up, Action::MoveTile(Direction::Up)),
        bind(mod_shift, Keysym::Down, Action::MoveTile(Direction::Down)),
        // Resizing: Super+- = for width, with Shift for height. Keys are matched on the raw
        // symbol, so Shift keeps them - and =; a layout that reports _ and + there works too.
        bind(mod_, Keysym::equal, Action::Resize(Resize::Wider)),
        bind(mod_, Keysym::minus, Action::Resize(Resize::Narrower)),
        bind(mod_shift, Keysym::equal, Action::Resize(Resize::Taller)),
        bind(mod_shift, Keysym::minus, Action::Resize(Resize::Shorter)),
        bind(mod_shift, Keysym::plus, Action::Resize(Resize::Taller)),
        bind(
            mod_shift,
            Keysym::underscore,
            Action::Resize(Resize::Shorter),
        ),
        // Workspaces along: Omarchy's Super+Tab, and Windows' virtual desktop keys.
        bind(mod_, Keysym::Tab, Action::WorkspaceBy(1)),
        bind(mod_shift, Keysym::Tab, Action::WorkspaceBy(-1)),
        bind(mod_shift, Keysym::ISO_Left_Tab, Action::WorkspaceBy(-1)),
        bind(mod_ctrl, Keysym::Left, Action::WorkspaceBy(-1)),
        bind(mod_ctrl, Keysym::Right, Action::WorkspaceBy(1)),
        bind(mod_ctrl_shift, Keysym::Left, Action::MoveToWorkspaceBy(-1)),
        bind(mod_ctrl_shift, Keysym::Right, Action::MoveToWorkspaceBy(1)),
        // Omarchy's close key, and Windows'. On real hardware only: nested inside KDE, KWin takes
        // Alt+F4 for its own window.
        bind(mod_, Keysym::w, Action::Close),
        bind(alt, Keysym::F4, Action::Close),
        // Windows' task switcher. Letting go of Alt settles on the window (see input.rs).
        bind(alt, Keysym::Tab, Action::CycleWindows { forward: true }),
        bind(
            alt_shift,
            Keysym::Tab,
            Action::CycleWindows { forward: false },
        ),
        bind(
            alt_shift,
            Keysym::ISO_Left_Tab,
            Action::CycleWindows { forward: false },
        ),
        bind(mod_, Keysym::Return, Action::Launch(App::Terminal)),
        bind(mod_shift, Keysym::f, Action::Launch(App::Files)),
        bind(mod_, Keysym::i, Action::Launch(App::Settings)),
        bind(mod_shift, Keysym::b, Action::Launch(App::Browser)),
        bind(mod_shift, Keysym::Return, Action::Launch(App::Browser)),
        // The app explorer has no binding here: a tapped Super opens it, as the Windows key opens
        // Start (see `SuperTap`). Super+Space stays IBus's, to switch input methods.
        //
        // Windows' own display key: Win+P is where a Windows user looks for anything to do with
        // screens. With one screen it says there's nowhere to go.
        bind(mod_, Keysym::p, Action::NextScreen),
        bind(mod_shift, Keysym::p, Action::MoveToNextScreen),
        bind(mod_ctrl, Keysym::p, Action::SwapScreens),
        // Windows' quick settings key.
        bind(mod_, Keysym::a, Action::QuickSettings),
        // Windows' screenshot keys: Print for the screen, Win+Shift+S to snip a region or a window
        // (gamescope keeps it while focused, as it does Super+S).
        bind(
            Mods::default(),
            Keysym::Print,
            Action::Screenshot { window: false },
        ),
        bind(mod_shift, Keysym::s, Action::Screenshot { window: true }),
        // Windows' show-desktop key: an empty workspace, and back.
        bind(mod_, Keysym::d, Action::ShowDesktop),
        // Omarchy's lock key.
        bind(mod_ctrl, Keysym::l, Action::Lock),
        // Omarchy's clipboard manager key.
        bind(mod_ctrl, Keysym::v, Action::ClipboardHistory),
        // Omarchy's copy, cut and paste: the same three keys in a terminal, an editor and
        // everything else, each sent the keys it takes for them.
        bind(mod_, Keysym::c, Action::Edit(Edit::Copy)),
        bind(mod_, Keysym::x, Action::Edit(Edit::Cut)),
        bind(mod_, Keysym::v, Action::Edit(Edit::Paste)),
        // Omarchy's float key, and the floating windows' own focus beside the clipboard's V.
        bind(mod_, Keysym::t, Action::ToggleFloating),
        bind(mod_alt, Keysym::v, Action::SwitchFloatingFocus),
        // Windows' notification centre key.
        bind(mod_, Keysym::n, Action::NotificationCentre),
        // Every key, one keystroke away, where Omarchy keeps its list.
        bind(mod_, Keysym::k, Action::ShortcutSheet),
        bind(mod_, Keysym::g, Action::ToggleGravity),
        // Turning the layout. Windows has no equivalent, so this is a new key for a new idea;
        // the shortcut sheet (Super+K) lists it.
        bind(mod_, Keysym::r, Action::Rotate { clockwise: true }),
        bind(mod_shift, Keysym::r, Action::Rotate { clockwise: false }),
        // Fill the tiling area, keeping the neighbours' tiles underneath. gamescope keeps Super+F
        // while it's focused (`passes_to_gamescope`).
        bind(mod_, Keysym::f, Action::Maximise),
        // Minimise into the code rain on M: Super+↓, Windows' own minimise key, moves focus here.
        bind(mod_, Keysym::m, Action::Minimise),
        // Super+Tab goes along the workspaces, as on Omarchy; Alt joins it for the overview.
        bind(mod_alt, Keysym::Tab, Action::BulletTime),
        // Bring back the last window minimised.
        bind(mod_shift, Keysym::m, Action::Restore),
        // Windows' own minimise-all key is Super+M, which is one window here, as its restore
        // key Super+Shift+M is. Super+H is free, and next to them on the keyboard.
        bind(mod_, Keysym::h, Action::HideAll),
        // A window docked to the bar as a pane of glass, and back with Shift as Super+M's
        // restore is. Super+W closes and Super+O pops a window out, as on Omarchy, so the pane
        // that goes up to the bar takes U.
        bind(mod_, Keysym::u, Action::Dock),
        bind(mod_shift, Keysym::u, Action::Undock),
        // Omarchy's pop-out key: floating, and on every workspace its screen goes to.
        bind(mod_, Keysym::o, Action::Pop),
        // Media keys (the Fn row on laptops) need no modifier, as on Windows.
        bind(
            Mods::default(),
            Keysym::XF86_AudioPlay,
            Action::Media(crate::media::Transport::PlayPause),
        ),
        bind(
            Mods::default(),
            Keysym::XF86_AudioPause,
            Action::Media(crate::media::Transport::PlayPause),
        ),
        bind(
            Mods::default(),
            Keysym::XF86_AudioNext,
            Action::Media(crate::media::Transport::Next),
        ),
        bind(
            Mods::default(),
            Keysym::XF86_AudioPrev,
            Action::Media(crate::media::Transport::Previous),
        ),
        bind(
            Mods::default(),
            Keysym::XF86_AudioStop,
            Action::Media(crate::media::Transport::Stop),
        ),
        bind(
            Mods::default(),
            Keysym::XF86_AudioRaiseVolume,
            Action::Volume(5),
        ),
        bind(
            Mods::default(),
            Keysym::XF86_AudioLowerVolume,
            Action::Volume(-5),
        ),
        bind(
            Mods::default(),
            Keysym::XF86_AudioMute,
            Action::ToggleMute { microphone: false },
        ),
        bind(
            Mods::default(),
            Keysym::XF86_AudioMicMute,
            Action::ToggleMute { microphone: true },
        ),
        bind(
            Mods::default(),
            Keysym::XF86_MonBrightnessUp,
            Action::Brightness(5),
        ),
        bind(
            Mods::default(),
            Keysym::XF86_MonBrightnessDown,
            Action::Brightness(-5),
        ),
    ];
    // Ten keys reach the first ten workspaces, 0 being the tenth; past that, bullet time and
    // Super+Tab do.
    for (i, key) in DIGITS.into_iter().enumerate() {
        let n = i as u8 + 1;
        bindings.push(bind(mod_, key, Action::Workspace(n)));
        bindings.push(bind(mod_shift, key, Action::MoveToWorkspace(n)));
    }
    bindings
}

/// The defaults with the run-or-raise keys from settings laid over them. Each app gets the raise
/// chord on its key and the same with Shift to send a window there; the digits under the raise
/// chord go to a workspace by number, and with Shift send a window there. A run-or-raise key
/// takes over a default on the same keys. An app that can't be bound is logged and left out.
pub fn bindings(settings: &slipstream_config::RunOrRaise) -> Vec<Binding> {
    let mut extra = Vec::new();
    if !settings.app.is_empty() {
        match parse_mods(&settings.raise) {
            Some(raise) => {
                let send = Mods {
                    shift: true,
                    ..raise
                };
                for (index, app) in settings.app.iter().enumerate().take(usize::from(u8::MAX)) {
                    let Some(key) = parse_key(&app.key) else {
                        tracing::warn!(key = app.key, "run-or-raise: no such key");
                        continue;
                    };
                    let index = index as u8;
                    extra.push(Binding {
                        mods: raise,
                        key,
                        action: Action::RunOrRaise(index),
                    });
                    extra.push(Binding {
                        mods: send,
                        key,
                        action: Action::SendTo(index),
                    });
                }
                for (i, key) in DIGITS.into_iter().enumerate() {
                    let n = i as u8 + 1;
                    extra.push(Binding {
                        mods: raise,
                        key,
                        action: Action::Workspace(n),
                    });
                    extra.push(Binding {
                        mods: send,
                        key,
                        action: Action::MoveToWorkspace(n),
                    });
                }
            }
            None => tracing::warn!(raise = settings.raise, "run-or-raise: can't read the chord"),
        }
    }
    let mut all: Vec<Binding> = Vec::new();
    for binding in extra.into_iter().chain(defaults()) {
        if all
            .iter()
            .any(|b| (b.mods, b.key) == (binding.mods, binding.key))
        {
            continue;
        }
        all.push(binding);
    }
    checked(all)
}

/// Modifiers written `ctrl+super+alt`, in any order and case. `logo`, `mod4` and `win` are Super
/// too. None for an empty chord or a word that isn't a modifier.
pub fn parse_mods(text: &str) -> Option<Mods> {
    let mut mods = Mods::default();
    let mut any = false;
    for word in text.split('+').map(str::trim) {
        match word.to_ascii_lowercase().as_str() {
            "super" | "logo" | "mod4" | "win" => mods.logo = true,
            "ctrl" | "control" => mods.ctrl = true,
            "alt" | "mod1" => mods.alt = true,
            "shift" => mods.shift = true,
            _ => return None,
        }
        any = true;
    }
    any.then_some(mods)
}

/// A key by its xkb name (`c`, `Return`, `F5`). A single letter is taken as the lower-case key,
/// which is what bindings match on whatever Shift is doing.
pub fn parse_key(text: &str) -> Option<Keysym> {
    let text = text.trim();
    let mut chars = text.chars();
    if let (Some(ch), None) = (chars.next(), chars.clone().next())
        && ch.is_ascii_alphabetic()
    {
        return Some(xkb::utf32_to_keysym(ch.to_ascii_lowercase() as u32));
    }
    let key = xkb::keysym_from_name(text, xkb::KEYSYM_NO_FLAGS);
    let key = if key.raw() == 0 {
        xkb::keysym_from_name(text, xkb::KEYSYM_CASE_INSENSITIVE)
    } else {
        key
    };
    (key.raw() != 0).then_some(key)
}

/// How long Super can be held and still count as a tap.
pub const TAP_MS: u32 = 400;

/// Super tapped on its own, which opens the explorer as the Windows key opens Start.
///
/// Armed by Super pressed with no other key held; disarmed by any other key press (modifiers
/// included, so the Copilot key's Super+Shift+F23 never taps), a pointer button or the wheel;
/// fires when Super is let go within `TAP_MS` while still armed.
#[derive(Debug, Default)]
pub struct SuperTap {
    /// When Super went down, in milliseconds, while a tap is still possible.
    armed_at: Option<u32>,
}

impl SuperTap {
    /// A key pressed or let go at `time` (milliseconds). `alone` is whether no other key was held
    /// as it went down, and whether a tap is allowed at all. True when this completes a tap.
    pub fn key(&mut self, key: Keysym, pressed: bool, time: u32, alone: bool) -> bool {
        let is_super = key == Keysym::Super_L || key == Keysym::Super_R;
        match (is_super, pressed) {
            (true, true) => {
                self.armed_at = alone.then_some(time);
                false
            }
            (true, false) => self
                .armed_at
                .take()
                .is_some_and(|at| time.wrapping_sub(at) < TAP_MS),
            (false, true) => {
                self.armed_at = None;
                false
            }
            (false, false) => false,
        }
    }

    /// A pointer button or the wheel: whatever Super is held for, it isn't a tap.
    pub fn disarm(&mut self) {
        self.armed_at = None;
    }
}

/// The number keys 1–9 then 0, in order: 0 is the tenth.
pub const DIGITS: [Keysym; 10] = [
    Keysym::_1,
    Keysym::_2,
    Keysym::_3,
    Keysym::_4,
    Keysym::_5,
    Keysym::_6,
    Keysym::_7,
    Keysym::_8,
    Keysym::_9,
    Keysym::_0,
];

/// The keyboard's own hardware keys: volume, brightness, media and the like, which stay the
/// desktop's even while an app holds every other key.
pub fn is_hardware_key(key: Keysym) -> bool {
    (0x1008_ff00..=0x1008_ffff).contains(&key.raw())
}

pub fn action_for(bindings: &[Binding], mods: Mods, key: Keysym) -> Option<Action> {
    bindings
        .iter()
        .find(|b| b.mods == mods && b.key == key)
        .map(|b| b.action)
}

/// Drops any binding that takes a reserved combination, with a warning. Every binding goes
/// through this, so a future config file can't break input methods, screen readers or games.
pub fn checked(bindings: Vec<Binding>) -> Vec<Binding> {
    bindings
        .into_iter()
        .filter(|b| {
            let reserved = is_reserved(b.mods, b.key);
            if reserved {
                tracing::warn!(
                    ?b,
                    "ignoring binding: reserved for input methods, screen readers or apps"
                );
            }
            !reserved
        })
        .collect()
}

const F_KEYS: [Keysym; 12] = [
    Keysym::F1,
    Keysym::F2,
    Keysym::F3,
    Keysym::F4,
    Keysym::F5,
    Keysym::F6,
    Keysym::F7,
    Keysym::F8,
    Keysym::F9,
    Keysym::F10,
    Keysym::F11,
    Keysym::F12,
];

const MODIFIER_KEYS: [Keysym; 9] = [
    Keysym::Shift_L,
    Keysym::Shift_R,
    Keysym::Control_L,
    Keysym::Control_R,
    Keysym::Alt_L,
    Keysym::Alt_R,
    Keysym::Super_L,
    Keysym::Super_R,
    Keysym::ISO_Level3_Shift,
];

/// Combinations that belong to something other than the window manager.
pub fn is_reserved(mods: Mods, key: Keysym) -> bool {
    let f_key = F_KEYS.contains(&key);
    // Ctrl+Alt+F1–F12 switch virtual terminals. Ctrl+Alt+Del is not reserved: it is the chord a
    // Windows user reaches for under stress, the kernel never sees it while we hold the keyboard
    // through libinput, and leaving it unanswered is worse than answering it.
    if mods.ctrl && mods.alt && f_key {
        return true;
    }
    // Alt+SysRq is the kernel's emergency interface.
    if mods.alt && (key == Keysym::Sys_Req || key == Keysym::Print) {
        return true;
    }
    // Input-method and layout switching: Ctrl+Space, Alt+Shift, Ctrl+Shift, and IBus's
    // Super+Space, which a tapped Super already covers here.
    if (mods.ctrl || mods.logo) && key == Keysym::space {
        return true;
    }
    if MODIFIER_KEYS.contains(&key) || key == Keysym::Menu {
        return true;
    }
    // Emoji and Unicode entry (Super+. Super+; Ctrl+. Ctrl+;), and the whole Ctrl+Shift layer,
    // which terminals and IDEs use (including Ctrl+Shift+U). With Super held too the chord is
    // never an app's: apps don't get Super chords, so Super+Ctrl+Shift, and the hyper chord's
    // Shift half, stay the desktop's.
    if (mods.logo || mods.ctrl) && (key == Keysym::period || key == Keysym::semicolon) {
        return true;
    }
    if mods.ctrl && mods.shift && !mods.logo {
        return true;
    }
    // Orca: Insert and Caps Lock act as its modifier; Super+Alt+S toggles it.
    if key == Keysym::Insert || key == Keysym::Caps_Lock {
        return true;
    }
    if mods.logo && mods.alt && key == Keysym::s {
        return true;
    }
    if !mods.logo {
        // Steam overlay (Shift+Tab), Steam screenshots (F12), MangoHud (Shift+F-keys).
        // Only plain Shift+Tab: Alt+Shift+Tab is Windows' reverse task switch.
        if mods.shift
            && !mods.alt
            && !mods.ctrl
            && (key == Keysym::Tab || key == Keysym::ISO_Left_Tab)
        {
            return true;
        }
        if key == Keysym::F12 || (mods.shift && f_key) {
            return true;
        }
        // App navigation: Ctrl+Tab, Alt+←/→, Ctrl+Alt+←/→, F5, F10, F11.
        if mods.ctrl && key == Keysym::Tab {
            return true;
        }
        if mods.alt && (key == Keysym::Left || key == Keysym::Right) {
            return true;
        }
        if key == Keysym::F5 || key == Keysym::F10 || key == Keysym::F11 {
            return true;
        }
    }
    false
}

/// Super+letter keys gamescope uses for itself. While a gamescope window is focused these
/// go to it instead of running Slipstream's binding.
pub fn passes_to_gamescope(mods: Mods, key: Keysym) -> bool {
    const GAMESCOPE: [Keysym; 9] = [
        Keysym::f,
        Keysym::n,
        Keysym::b,
        Keysym::u,
        Keysym::y,
        Keysym::i,
        Keysym::o,
        Keysym::s,
        Keysym::g,
    ];
    mods.logo && !mods.ctrl && !mods.alt && GAMESCOPE.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUPER: Mods = Mods {
        ctrl: false,
        alt: false,
        shift: false,
        logo: true,
    };

    #[test]
    fn defaults_never_use_reserved_combos() {
        for b in defaults() {
            assert!(
                !is_reserved(b.mods, b.key),
                "{b:?} takes a reserved combination"
            );
        }
    }

    #[test]
    fn defaults_are_unique() {
        let all = defaults();
        for (i, a) in all.iter().enumerate() {
            assert!(
                all[i + 1..]
                    .iter()
                    .all(|b| (b.mods, b.key) != (a.mods, a.key)),
                "{a:?} is bound twice"
            );
        }
    }

    #[test]
    fn the_way_out_is_on_ctrl_alt_del_and_super_esc() {
        let bindings = defaults();
        let ctrl_alt = Mods {
            ctrl: true,
            alt: true,
            ..Mods::default()
        };
        assert_eq!(
            action_for(&bindings, ctrl_alt, Keysym::Delete),
            Some(Action::WayOut)
        );
        // Super+Esc is Omarchy's system menu: the same card, which ends nothing by itself.
        assert_eq!(
            action_for(&bindings, SUPER, Keysym::Escape),
            Some(Action::WayOut)
        );
        assert_eq!(action_for(&bindings, Mods::default(), Keysym::Escape), None);
        // Super+Shift+Esc takes the mouse and keys back.
        let super_shift = Mods {
            shift: true,
            ..SUPER
        };
        assert_eq!(
            action_for(&bindings, super_shift, Keysym::Escape),
            Some(Action::TakeBack)
        );
    }

    #[test]
    fn only_the_laptops_own_keys_count_as_hardware_keys() {
        assert!(is_hardware_key(Keysym::XF86_AudioRaiseVolume));
        assert!(is_hardware_key(Keysym::XF86_MonBrightnessDown));
        assert!(!is_hardware_key(Keysym::Escape));
        assert!(!is_hardware_key(Keysym::Print));
        assert!(!is_hardware_key(Keysym::Super_L));
    }

    #[test]
    fn hide_all_is_its_own_key_beside_minimise() {
        let bindings = defaults();
        assert_eq!(
            action_for(&bindings, SUPER, Keysym::h),
            Some(Action::HideAll)
        );
        // Super+M stays one window, as on Windows, and Super+D still only shows the desktop.
        assert_eq!(
            action_for(&bindings, SUPER, Keysym::m),
            Some(Action::Minimise)
        );
        assert_eq!(
            action_for(&bindings, SUPER, Keysym::d),
            Some(Action::ShowDesktop)
        );
    }

    #[test]
    fn super_u_docks_a_window_and_with_shift_brings_it_back() {
        let bindings = defaults();
        let super_shift = Mods {
            shift: true,
            ..SUPER
        };
        assert_eq!(action_for(&bindings, SUPER, Keysym::u), Some(Action::Dock));
        assert_eq!(
            action_for(&bindings, super_shift, Keysym::u),
            Some(Action::Undock)
        );
        assert_eq!(action_for(&bindings, SUPER, Keysym::o), Some(Action::Pop));
    }

    #[test]
    fn mod_arrows_move_focus() {
        let bindings = defaults();
        assert_eq!(
            action_for(&bindings, SUPER, Keysym::Right),
            Some(Action::Focus(Direction::Right))
        );
        assert_eq!(action_for(&bindings, Mods::default(), Keysym::Right), None);
    }

    #[test]
    fn mod_shift_arrows_move_the_tile_itself() {
        let bindings = defaults();
        let super_shift = Mods {
            shift: true,
            ..SUPER
        };
        assert_eq!(
            action_for(&bindings, super_shift, Keysym::Down),
            Some(Action::MoveTile(Direction::Down))
        );
        let super_alt = Mods { alt: true, ..SUPER };
        // Orca's toggle is the one Super+Alt combination that must not be ours.
        assert!(is_reserved(super_alt, Keysym::s));
        assert!(!is_reserved(super_alt, Keysym::Down));
    }

    #[test]
    fn windows_style_workspace_keys() {
        let bindings = defaults();
        let super_ctrl = Mods {
            ctrl: true,
            ..SUPER
        };
        let super_shift = Mods {
            shift: true,
            ..SUPER
        };
        let alt = Mods {
            alt: true,
            ..Mods::default()
        };
        assert_eq!(
            action_for(&bindings, SUPER, Keysym::_3),
            Some(Action::Workspace(3))
        );
        assert_eq!(
            action_for(&bindings, super_shift, Keysym::_5),
            Some(Action::MoveToWorkspace(5))
        );
        assert_eq!(
            action_for(&bindings, super_ctrl, Keysym::Right),
            Some(Action::WorkspaceBy(1))
        );
        let super_ctrl_shift = Mods {
            shift: true,
            ..super_ctrl
        };
        assert_eq!(
            action_for(&bindings, super_ctrl_shift, Keysym::Left),
            Some(Action::MoveToWorkspaceBy(-1))
        );
        assert_eq!(action_for(&bindings, alt, Keysym::F4), Some(Action::Close));
        assert_eq!(
            action_for(&bindings, SUPER, Keysym::r),
            Some(Action::Rotate { clockwise: true })
        );
        // Super+Space is the input methods' again: a tapped Super opens the explorer.
        assert_eq!(action_for(&bindings, SUPER, Keysym::space), None);
    }

    #[test]
    fn nested_alt_stands_in_for_super() {
        let alt = ModifiersState {
            alt: true,
            ..ModifiersState::default()
        };
        let nested = Mods::from_state(&alt, true);
        assert_eq!(
            action_for(&defaults(), nested, Keysym::Right),
            Some(Action::Focus(Direction::Right))
        );
        let hardware = Mods::from_state(&alt, false);
        assert_eq!(action_for(&defaults(), hardware, Keysym::Right), None);
    }

    #[test]
    fn well_known_combinations_are_reserved() {
        let ctrl_alt = Mods {
            ctrl: true,
            alt: true,
            ..Mods::default()
        };
        let ctrl_shift = Mods {
            ctrl: true,
            shift: true,
            ..Mods::default()
        };
        let shift = Mods {
            shift: true,
            ..Mods::default()
        };
        let alt = Mods {
            alt: true,
            ..Mods::default()
        };
        let ctrl = Mods {
            ctrl: true,
            ..Mods::default()
        };
        assert!(is_reserved(ctrl, Keysym::space), "input-method switch");
        assert!(
            is_reserved(SUPER, Keysym::space),
            "IBus input-method switch"
        );
        assert!(is_reserved(ctrl_alt, Keysym::F2), "virtual terminal switch");
        // The one chord taken back off the list: nothing below us answers it.
        assert!(!is_reserved(ctrl_alt, Keysym::Delete), "the way out");
        assert!(is_reserved(ctrl_shift, Keysym::c), "terminal copy");
        assert!(is_reserved(shift, Keysym::Tab), "Steam overlay");
        assert!(is_reserved(alt, Keysym::Left), "browser back");
        assert!(
            is_reserved(Mods::default(), Keysym::Insert),
            "Orca modifier"
        );
    }

    #[test]
    fn planned_slipstream_keys_are_free() {
        let super_ctrl = Mods {
            ctrl: true,
            ..SUPER
        };
        let super_shift = Mods {
            shift: true,
            ..SUPER
        };
        for (mods, key) in [
            (SUPER, Keysym::Tab),
            (SUPER, Keysym::z),
            (SUPER, Keysym::Left),
            (super_ctrl, Keysym::Right),
            (super_shift, Keysym::_1),
            (SUPER, Keysym::t),
            (SUPER, Keysym::r),
            (SUPER, Keysym::i),
            (SUPER, Keysym::a),
            (SUPER, Keysym::n),
            (SUPER, Keysym::f),
            (SUPER, Keysym::slash),
        ] {
            assert!(!is_reserved(mods, key), "{key:?} should be available");
        }
    }

    #[test]
    fn checked_drops_reserved_bindings() {
        let input_method_switch = Binding {
            mods: Mods {
                ctrl: true,
                ..Mods::default()
            },
            key: Keysym::space,
            action: Action::Explorer,
        };
        let way_out = defaults()[0];
        assert_eq!(checked(vec![input_method_switch, way_out]), vec![way_out]);
    }

    #[test]
    fn alt_tab_switches_windows() {
        let alt = Mods {
            alt: true,
            ..Mods::default()
        };
        let alt_shift = Mods { shift: true, ..alt };
        let bindings = checked(defaults());
        assert_eq!(
            action_for(&bindings, alt, Keysym::Tab),
            Some(Action::CycleWindows { forward: true })
        );
        assert_eq!(
            action_for(&bindings, alt_shift, Keysym::ISO_Left_Tab),
            Some(Action::CycleWindows { forward: false })
        );
        let shift = Mods {
            shift: true,
            ..Mods::default()
        };
        assert!(
            is_reserved(shift, Keysym::Tab),
            "Steam overlay stays reserved"
        );
    }

    #[test]
    fn super_f_maximises_and_stays_gamescope_s_while_it_is_focused() {
        assert_eq!(
            action_for(&checked(defaults()), SUPER, Keysym::f),
            Some(Action::Maximise)
        );
        assert!(passes_to_gamescope(SUPER, Keysym::f));
    }

    #[test]
    fn every_default_binding_is_described() {
        for b in checked(defaults()) {
            assert!(!b.action.describe().is_empty(), "{b:?} has no description");
            assert!(!b.action.group().title().is_empty());
        }
        let super_shift = Mods {
            shift: true,
            ..SUPER
        };
        assert_eq!(label(super_shift, Keysym::Escape), "Super+Shift+Esc");
        assert_eq!(label(SUPER, Keysym::slash), "Super+/");
        assert_eq!(label(super_shift, Keysym::braceright), "Super+Shift+]");
        assert_eq!(label(Mods::default(), Keysym::Print), "Print");
        assert_eq!(label(SUPER, Keysym::a), "Super+A");
        assert_eq!(
            action_for(&checked(defaults()), SUPER, Keysym::k),
            Some(Action::ShortcutSheet)
        );
    }

    #[test]
    fn where_omarchy_has_the_action_it_has_omarchys_key() {
        let bindings = checked(defaults());
        let with = |shift, ctrl, alt| Mods {
            shift,
            ctrl,
            alt,
            logo: true,
        };
        for (mods, key, action) in [
            (SUPER, Keysym::w, Action::Close),
            (SUPER, Keysym::t, Action::ToggleFloating),
            (SUPER, Keysym::f, Action::Maximise),
            (SUPER, Keysym::k, Action::ShortcutSheet),
            (SUPER, Keysym::Return, Action::Launch(App::Terminal)),
            (SUPER, Keysym::Tab, Action::WorkspaceBy(1)),
            (SUPER, Keysym::equal, Action::Resize(Resize::Wider)),
            (SUPER, Keysym::minus, Action::Resize(Resize::Narrower)),
            (SUPER, Keysym::_0, Action::Workspace(10)),
            (
                with(true, false, false),
                Keysym::_0,
                Action::MoveToWorkspace(10),
            ),
            (
                with(true, false, false),
                Keysym::Tab,
                Action::WorkspaceBy(-1),
            ),
            (
                with(true, false, false),
                Keysym::f,
                Action::Launch(App::Files),
            ),
            (
                with(true, false, false),
                Keysym::b,
                Action::Launch(App::Browser),
            ),
            (
                with(true, false, false),
                Keysym::Return,
                Action::Launch(App::Browser),
            ),
            (
                with(true, false, false),
                Keysym::equal,
                Action::Resize(Resize::Taller),
            ),
            (
                with(true, false, false),
                Keysym::Left,
                Action::MoveTile(Direction::Left),
            ),
            (with(false, true, false), Keysym::l, Action::Lock),
            (
                with(false, true, false),
                Keysym::v,
                Action::ClipboardHistory,
            ),
            (SUPER, Keysym::c, Action::Edit(Edit::Copy)),
            (SUPER, Keysym::x, Action::Edit(Edit::Cut)),
            (SUPER, Keysym::v, Action::Edit(Edit::Paste)),
        ] {
            assert_eq!(
                action_for(&bindings, mods, key),
                Some(action),
                "{mods:?} {key:?}"
            );
        }
        // Slipstream's own, moved out of the way.
        assert_eq!(
            action_for(&bindings, SUPER, Keysym::g),
            Some(Action::ToggleGravity)
        );
        assert_eq!(
            action_for(&bindings, with(false, false, true), Keysym::Tab),
            Some(Action::BulletTime)
        );
        for key in [Keysym::l, Keysym::e, Keysym::b, Keysym::slash] {
            assert_eq!(action_for(&bindings, SUPER, key), None, "{key:?}");
        }
    }

    fn hyper(shift: bool) -> Mods {
        Mods {
            ctrl: true,
            alt: true,
            shift,
            logo: true,
        }
    }

    fn app(workspace: &str, key: &str) -> slipstream_config::RaiseApp {
        slipstream_config::RaiseApp {
            workspace: workspace.into(),
            key: key.into(),
            ..Default::default()
        }
    }

    #[test]
    fn each_run_or_raise_app_gets_the_hyper_chord_and_its_shift_half() {
        let settings = slipstream_config::RunOrRaise {
            app: vec![
                app("Chrome", "c"),
                app("Terminal", "T"),
                app("Bad", "nokey"),
            ],
            ..Default::default()
        };
        let bindings = bindings(&settings);
        assert_eq!(
            action_for(&bindings, hyper(false), Keysym::c),
            Some(Action::RunOrRaise(0))
        );
        assert_eq!(
            action_for(&bindings, hyper(true), Keysym::c),
            Some(Action::SendTo(0))
        );
        assert_eq!(
            action_for(&bindings, hyper(false), Keysym::t),
            Some(Action::RunOrRaise(1)),
            "a capital letter is the letter's key"
        );
        assert_eq!(
            action_for(&bindings, hyper(false), Keysym::_0),
            Some(Action::Workspace(10))
        );
        assert_eq!(
            action_for(&bindings, hyper(true), Keysym::_6),
            Some(Action::MoveToWorkspace(6))
        );
        // The defaults are all still there beside them.
        assert_eq!(action_for(&bindings, SUPER, Keysym::w), Some(Action::Close));
    }

    #[test]
    fn without_apps_there_are_no_hyper_keys() {
        let bindings = bindings(&slipstream_config::RunOrRaise::default());
        assert_eq!(action_for(&bindings, hyper(false), Keysym::_1), None);
        assert_eq!(bindings, checked(defaults()));
    }

    #[test]
    fn a_run_or_raise_key_takes_over_a_default_on_the_same_keys() {
        let settings = slipstream_config::RunOrRaise {
            raise: "super".into(),
            app: vec![app("Mail", "w")],
        };
        let bindings = bindings(&settings);
        assert_eq!(
            action_for(&bindings, SUPER, Keysym::w),
            Some(Action::RunOrRaise(0))
        );
        let once = bindings
            .iter()
            .filter(|b| (b.mods, b.key) == (SUPER, Keysym::w))
            .count();
        assert_eq!(once, 1);
    }

    #[test]
    fn chords_and_keys_read_as_written() {
        assert_eq!(parse_mods("ctrl+super+alt"), Some(hyper(false)));
        assert_eq!(
            parse_mods(" Control + Win + Alt + Shift "),
            Some(hyper(true))
        );
        assert_eq!(parse_mods(""), None);
        assert_eq!(parse_mods("super+hyper"), None);
        assert_eq!(parse_key("c"), Some(Keysym::c));
        assert_eq!(parse_key("C"), Some(Keysym::c));
        assert_eq!(parse_key("Return"), Some(Keysym::Return));
        assert_eq!(parse_key("return"), Some(Keysym::Return));
        assert_eq!(parse_key("F5"), Some(Keysym::F5));
        assert_eq!(parse_key("1"), Some(Keysym::_1));
        assert_eq!(parse_key("nokey"), None);
    }

    #[test]
    fn the_hyper_chords_shift_half_is_not_reserved_but_ctrl_shift_alone_is() {
        assert!(!is_reserved(hyper(true), Keysym::c));
        let super_ctrl_shift = Mods {
            alt: false,
            ..hyper(true)
        };
        assert!(!is_reserved(super_ctrl_shift, Keysym::Left));
        let ctrl_shift = Mods {
            ctrl: true,
            shift: true,
            ..Mods::default()
        };
        assert!(is_reserved(ctrl_shift, Keysym::c));
    }

    #[test]
    fn a_tap_opens_the_explorer() {
        let mut tap = SuperTap::default();
        assert!(!tap.key(Keysym::Super_L, true, 1000, true));
        assert!(tap.key(Keysym::Super_L, false, 1100, true));
        assert!(
            !tap.key(Keysym::Super_L, false, 1150, true),
            "once per press"
        );
        assert!(!tap.key(Keysym::Super_R, true, 2000, true));
        assert!(tap.key(Keysym::Super_R, false, 2399, true));
    }

    #[test]
    fn a_chord_disarms_it() {
        let mut tap = SuperTap::default();
        tap.key(Keysym::Super_L, true, 1000, true);
        tap.key(Keysym::m, true, 1050, true);
        tap.key(Keysym::m, false, 1080, true);
        assert!(!tap.key(Keysym::Super_L, false, 1100, true));
        // Super pressed while another key is already down never arms.
        tap.key(Keysym::Super_L, true, 2000, false);
        assert!(!tap.key(Keysym::Super_L, false, 2100, true));
    }

    #[test]
    fn a_slow_press_doesnt() {
        let mut tap = SuperTap::default();
        tap.key(Keysym::Super_L, true, 1000, true);
        assert!(!tap.key(Keysym::Super_L, false, 1000 + TAP_MS, true));
        tap.key(Keysym::Super_L, true, u32::MAX - 10, true);
        assert!(
            tap.key(Keysym::Super_L, false, 50, true),
            "the clock wrapping is still a tap"
        );
    }

    #[test]
    fn shift_between_disarms_it() {
        // The Copilot key: Super, Shift and F23 together.
        let mut tap = SuperTap::default();
        tap.key(Keysym::Super_L, true, 1000, true);
        tap.key(Keysym::Shift_L, true, 1001, true);
        tap.key(Keysym::F23, true, 1002, true);
        tap.key(Keysym::F23, false, 1060, true);
        tap.key(Keysym::Shift_L, false, 1061, true);
        assert!(!tap.key(Keysym::Super_L, false, 1062, true));
    }

    #[test]
    fn a_pointer_button_disarms_it() {
        let mut tap = SuperTap::default();
        tap.key(Keysym::Super_L, true, 1000, true);
        tap.disarm();
        assert!(!tap.key(Keysym::Super_L, false, 1100, true));
    }

    #[test]
    fn gamescope_keys_pass_through() {
        assert!(passes_to_gamescope(SUPER, Keysym::s));
        assert!(!passes_to_gamescope(SUPER, Keysym::Tab));
        assert!(!passes_to_gamescope(Mods::default(), Keysym::s));
    }
}
