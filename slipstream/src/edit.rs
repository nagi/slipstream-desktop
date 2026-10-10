//! Copy, paste and cut in whichever app has the keyboard, by sending it the keys it already uses
//! for them.
//!
//! Apps don't agree on the keys. Most take Ctrl+C, X and V. A terminal keeps those for the
//! program running in it, and copies and pastes on Ctrl+Shift+C and V; there is nothing to cut,
//! so cutting copies. Emacs and gVim have uses of their own for all three (Ctrl+V scrolls one and
//! starts a block in the other), and yank, copy and kill on the Insert and Delete keys every
//! toolkit once shared. The desktop knows which app has the keyboard, so one set of keys sends
//! each its own, and Super held is never part of what the app sees.
//!
//! A program inside a terminal is pasted into by the terminal, as text typed at it, so Vim and
//! Emacs running there need nothing of their own.

use smithay::{
    backend::input::{InputTime, KeyState},
    input::keyboard::{KeyboardTarget, Keysym, ModifiersState},
    reexports::wayland_server::Resource,
    utils::SERIAL_COUNTER,
    wayland::seat::WaylandFocus,
};

use crate::{Slipstream, keys::Edit};

/// A key and the only modifiers held with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub ctrl: bool,
    pub shift: bool,
    pub key: Keysym,
}

/// App ids of editors that keep Ctrl+C, X and V for themselves and take the Insert and Delete
/// keys instead.
const OWN_KEYS: [&str; 2] = ["emacs", "gvim"];

/// The keys that do `edit` in the app with this id.
pub fn chord(edit: Edit, app_id: Option<&str>) -> Chord {
    let id = app_id.unwrap_or_default().to_lowercase();
    let (ctrl, shift, key) = if crate::history::is_terminal(&id) {
        match edit {
            // Ctrl+C would interrupt what's running, and Ctrl+X is a shell's prefix key.
            Edit::Copy | Edit::Cut => (true, true, Keysym::c),
            Edit::Paste => (true, true, Keysym::v),
        }
    } else if OWN_KEYS.iter().any(|editor| id.contains(editor)) {
        match edit {
            Edit::Copy => (true, false, Keysym::Insert),
            Edit::Paste => (false, true, Keysym::Insert),
            Edit::Cut => (false, true, Keysym::Delete),
        }
    } else {
        match edit {
            Edit::Copy => (true, false, Keysym::c),
            Edit::Paste => (true, false, Keysym::v),
            Edit::Cut => (true, false, Keysym::x),
        }
    };
    Chord { ctrl, shift, key }
}

impl Slipstream {
    /// Sends the focused app the keys for `edit`.
    pub fn send_edit(&mut self, edit: Edit) {
        if self.lock.is_some() {
            return;
        }
        let app_id = self
            .focused_window()
            .and_then(|window| crate::state::window_app_id(&window));
        let chord = chord(edit, app_id.as_deref());
        tracing::info!(?edit, ?chord, "sending the app its keys");
        self.send_chord(chord);
        // Super is still held, but it was part of a chord: letting go of it is no tap.
        self.super_tap.disarm();
    }

    /// Presses and lets go of the chord's key in the focused app, which is told that the chord's
    /// modifiers and no others are held while it happens, then what really is. Super is down when
    /// this is asked for, and a key pressed through the keyboard's own routing would arrive with
    /// it: Super+Ctrl+V, which pastes nowhere.
    ///
    /// The keyboard's own state isn't touched. `set_modifier_state` would do the same through it,
    /// but it loses the layout on the way, and whoever types in a second layout would be back in
    /// the first after every paste.
    ///
    /// The key goes to the app's own keyboards and not through the keyboard's grab. An input
    /// method holds that grab for as long as the app is taking text, in a terminal all the time,
    /// and would be handed the key without the modifiers the app was just told of.
    fn send_chord(&mut self, chord: Chord) {
        // Shift and Control are real modifiers, which have the same place in every keymap.
        const SHIFT: u32 = 1 << 0;
        const CONTROL: u32 = 1 << 2;
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        let Some(focus) = keyboard.current_focus() else {
            return;
        };
        let Some(keycode) = self.keycode_for(chord.key) else {
            tracing::warn!(key = ?chord.key, "no key for that in the layout, so nothing was sent");
            return;
        };
        let held = keyboard.modifier_state();
        let mut sent = ModifiersState {
            ctrl: chord.ctrl,
            shift: chord.shift,
            alt: false,
            logo: false,
            iso_level3_shift: false,
            iso_level5_shift: false,
            ..held
        };
        sent.serialized.depressed =
            if chord.ctrl { CONTROL } else { 0 } | if chord.shift { SHIFT } else { 0 };
        sent.serialized.latched = 0;
        let Some(client) = focus.wl_surface().and_then(|surface| surface.client()) else {
            return;
        };
        let time = InputTime::now().millis();
        let seat = self.seat.clone();
        focus.modifiers(&seat, self, sent, SERIAL_COUNTER.next_serial());
        for state in [KeyState::Pressed, KeyState::Released] {
            let serial = SERIAL_COUNTER.next_serial();
            for wl_keyboard in keyboard.client_keyboards(&client) {
                wl_keyboard.key(serial.into(), time, keycode.raw() - 8, state.into());
            }
        }
        focus.modifiers(&seat, self, held, SERIAL_COUNTER.next_serial());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(edit: Edit, app_id: &str) -> (bool, bool, Keysym) {
        let chord = chord(edit, Some(app_id));
        (chord.ctrl, chord.shift, chord.key)
    }

    #[test]
    fn most_apps_get_ctrl_c_x_and_v() {
        assert_eq!(keys(Edit::Copy, "firefox"), (true, false, Keysym::c));
        assert_eq!(keys(Edit::Cut, "firefox"), (true, false, Keysym::x));
        assert_eq!(keys(Edit::Paste, "firefox"), (true, false, Keysym::v));
        // A panel or anything else with no app id is treated the same.
        assert_eq!(chord(Edit::Paste, None).key, Keysym::v);
    }

    #[test]
    fn a_terminal_gets_its_own_keys_and_is_never_interrupted() {
        assert_eq!(
            keys(Edit::Copy, "com.mitchellh.ghostty"),
            (true, true, Keysym::c)
        );
        assert_eq!(keys(Edit::Paste, "Alacritty"), (true, true, Keysym::v));
        assert_eq!(keys(Edit::Cut, "foot"), (true, true, Keysym::c));
    }

    #[test]
    fn emacs_and_gvim_yank_copy_and_kill_on_insert_and_delete() {
        for editor in ["emacs", "Emacs", "gvim"] {
            assert_eq!(
                keys(Edit::Paste, editor),
                (false, true, Keysym::Insert),
                "{editor}"
            );
            assert_eq!(keys(Edit::Copy, editor), (true, false, Keysym::Insert));
            assert_eq!(keys(Edit::Cut, editor), (false, true, Keysym::Delete));
        }
    }
}
