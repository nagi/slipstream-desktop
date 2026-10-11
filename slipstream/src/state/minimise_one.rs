//! Super+M as a toggle: one window minimised at a time, and the same key brings it back.
//!
//! Kept apart from `streams`, which it only calls into, so that file stays as upstream has it.

use super::*;

/// Whether Super+M brings back the minimised window before it will minimise another. Off, the
/// key is upstream's: every press pours the focused window into the rain.
const MINIMISE_ONE: bool = true;

impl Slipstream {
    /// Super+M: with nothing in the rain the focused window pours into a stream, and with a
    /// window already there the key brings that one back, so a second never joins the first.
    pub fn minimise_key(&mut self) {
        if MINIMISE_ONE && !self.rain.streams.is_empty() {
            self.restore_latest();
        } else {
            self.minimise_focused();
        }
    }
}
