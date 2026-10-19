//! Focus follows the mouse, and the mouse follows focus, as on Omarchy.
//!
//! Moving the pointer onto a window gives it the keyboard; no click needed. The other way round,
//! a window that gets the keyboard by any other route (a focus key, Alt+Tab, run-or-raise, a new
//! window opening) has the pointer moved to its middle. The two keep each other honest: without
//! the second, the next nudge of the mouse would hand the keyboard straight back to whatever
//! window the pointer was left over.

use smithay::{
    backend::input::InputTime,
    desktop::Window,
    utils::{Logical, Point, Rectangle},
};

use crate::Slipstream;

impl Slipstream {
    /// The mouse moved to `pos`: the window under it takes the keyboard, if it hasn't already.
    ///
    /// Only for the mouse moving. A window sliding under a pointer that stayed put (a retile, a
    /// panel closing) does not take the keyboard, or opening an app beneath the pointer would
    /// lose it again at once.
    pub fn hover_focus(&mut self, pos: Point<f64, Logical>) {
        // Whatever has the pointer to itself keeps it: the lock, a card, a panel, bullet time.
        if self.pointer_blocked() || self.over_chrome(pos) {
            return;
        }
        // A menu's grab, a held button, or a tile being dragged or resized.
        if self.drag.is_some() || self.seat.get_pointer().unwrap().is_grabbed() {
            return;
        }
        // A launcher or another program's panel that took the keyboard keeps it until it closes.
        if self.keyboard_layer().is_some() {
            return;
        }
        // A layer surface drawn above the windows (a notification daemon's pop-ups, say), the bar
        // and the code rain take the pointer themselves.
        if self
            .layer_surface_under(pos, &crate::layers::FRONT)
            .is_some()
            || self.bar_target_at(pos).is_some()
            || self.rain_window_at(pos).is_some()
        {
            return;
        }
        let Some((window, _)) = self.window_under(pos) else {
            return;
        };
        // An X11 menu or tooltip is part of the window that opened it, not one to focus.
        if window
            .x11_surface()
            .is_some_and(|surface| surface.is_override_redirect())
        {
            return;
        }
        if self.focused_window().as_ref() == Some(&window) {
            return;
        }
        self.focus_window(&window);
    }

    /// The keyboard just moved to a window: once this pass of the event loop is over, and the
    /// layout has settled, the pointer goes to its middle unless it is already over it.
    pub(crate) fn pointer_follows_focus(&mut self) {
        self.loop_handle.insert_idle(|state| state.warp_to_focus());
    }

    fn warp_to_focus(&mut self) {
        // The mouse is busy with something else: an overlay, a drag, a menu.
        if self.lock.is_some() || self.pointer_blocked() || self.drag.is_some() {
            return;
        }
        if self.seat.get_pointer().unwrap().is_grabbed() {
            return;
        }
        let Some(window) = self.focused_window() else {
            return;
        };
        // Already over it, which is always the case when a hover or a click chose it.
        let at = self.pointer_location();
        if self
            .window_under(at)
            .is_some_and(|(under, _)| under == window)
        {
            return;
        }
        let Some(rect) = self.input_rect(&window) else {
            return;
        };
        let centre = self.clamp_to_outputs(Point::from((
            rect.loc.x + rect.size.w / 2.0,
            rect.loc.y + rect.size.h / 2.0,
        )));
        tracing::debug!(?centre, "pointer follows focus");
        self.pointer_placed_at(centre);
    }

    /// Where `window` is on screen for input: its tile's drawn rect when scaled into it, otherwise
    /// its own geometry in the space. A window that has only just opened has a place but no size
    /// yet, so it is taken at the size it was told to be.
    fn input_rect(&self, window: &Window) -> Option<Rectangle<f64, Logical>> {
        if let Some((_, drawn)) = self.fitted.iter().find(|(w, _)| w == window) {
            return Some(*drawn);
        }
        let mut rect = self.space.element_geometry(window)?;
        if rect.size.w <= 0 || rect.size.h <= 0 {
            let told = window
                .toplevel()
                .and_then(|toplevel| toplevel.with_pending_state(|state| state.size))?;
            rect.size = told;
        }
        Some(rect.to_f64())
    }

    /// Puts the pointer at `pos` without it counting as the mouse moving: whatever is there gets
    /// an enter, but the keyboard stays where it is.
    pub fn pointer_placed_at(&mut self, pos: Point<f64, Logical>) {
        self.pointer_motion_to(pos, InputTime::now());
    }
}
