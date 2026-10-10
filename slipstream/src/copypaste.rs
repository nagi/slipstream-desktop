//! Copy, cut and paste on Super+C, X and V, in whatever has the keyboard.
//!
//! Apps don't agree on the keys. Most take Ctrl+C, X and V. A terminal keeps those for the
//! program running in it, and copies and pastes on Ctrl+Shift+C and V. Emacs and gVim have uses
//! of their own for all three (Ctrl+V scrolls one and starts a block in the other), and yank,
//! copy and kill on the Insert and Delete keys every toolkit once shared. The desktop knows which
//! app has the keyboard, so one set of keys sends each its own, and Super held is never part of
//! what the app sees (`Slipstream::send_chord`).
//!
//! A program inside a terminal is pasted into by the terminal, as text typed at it, so Vim and
//! Emacs running there need nothing of their own.

use smithay::input::keyboard::Keysym;

/// What Super+C, X and V ask the focused app for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    Copy,
    Cut,
    Paste,
}

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

/// The keys that do `edit` in the app with this id. `None` is nothing to send: a terminal has
/// nothing to cut, and Ctrl+X there would go to the program running in it.
pub fn chord(edit: Edit, app_id: Option<&str>) -> Option<Chord> {
    let id = app_id.unwrap_or_default().to_lowercase();
    let (ctrl, shift, key) = if crate::history::is_terminal(&id) {
        match edit {
            Edit::Copy => (true, true, Keysym::c),
            Edit::Paste => (true, true, Keysym::v),
            Edit::Cut => return None,
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
    Some(Chord { ctrl, shift, key })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(edit: Edit, app_id: &str) -> Option<(bool, bool, Keysym)> {
        chord(edit, Some(app_id)).map(|chord| (chord.ctrl, chord.shift, chord.key))
    }

    #[test]
    fn most_apps_get_ctrl_c_x_and_v() {
        assert_eq!(keys(Edit::Copy, "firefox"), Some((true, false, Keysym::c)));
        assert_eq!(keys(Edit::Cut, "firefox"), Some((true, false, Keysym::x)));
        assert_eq!(keys(Edit::Paste, "firefox"), Some((true, false, Keysym::v)));
        // A panel or anything else with no app id is treated the same.
        assert_eq!(
            chord(Edit::Paste, None).map(|chord| chord.key),
            Some(Keysym::v)
        );
    }

    #[test]
    fn a_terminal_gets_shift_as_well_and_has_nothing_to_cut() {
        assert_eq!(
            keys(Edit::Copy, "com.mitchellh.ghostty"),
            Some((true, true, Keysym::c))
        );
        assert_eq!(
            keys(Edit::Paste, "Alacritty"),
            Some((true, true, Keysym::v))
        );
        assert_eq!(keys(Edit::Cut, "foot"), None);
    }

    #[test]
    fn emacs_and_gvim_yank_copy_and_kill_on_insert_and_delete() {
        for editor in ["emacs", "Emacs", "gvim"] {
            assert_eq!(
                keys(Edit::Paste, editor),
                Some((false, true, Keysym::Insert)),
                "{editor}"
            );
            assert_eq!(
                keys(Edit::Copy, editor),
                Some((true, false, Keysym::Insert))
            );
            assert_eq!(keys(Edit::Cut, editor), Some((false, true, Keysym::Delete)));
        }
    }
}
