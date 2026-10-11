//! Popped windows: the ones that float on whichever workspace their screen shows.
//!
//! A popped window is a floating window like any other (`floating.rs`), on one workspace at a
//! time. The difference is that when its screen goes to another workspace it goes too, keeping
//! its place on the glass: a video or a call stays in the corner of the eye wherever the work
//! moves. It stays on its screen; sending it to another screen is the same key as for any
//! window.

use super::*;
use crate::floating::centred;

/// The size a tile takes when it's popped out, where the screen has the room.
const POPPED: (i32, i32) = (1300, 900);

impl Slipstream {
    pub fn is_popped(&self, window: &Window) -> bool {
        self.popped.contains(window)
    }

    /// Super+O: the focused window pops out of the tiling, to float in the middle of the screen
    /// and follow it from workspace to workspace. One already floating keeps its place and
    /// size. Again, it stops following and goes back into the tiling.
    pub fn pop_focused(&mut self) {
        let Some(window) = self.focused_window() else {
            return;
        };
        if self.is_fullscreen(&window) {
            return;
        }
        if self.is_popped(&window) {
            self.popped.retain(|popped| *popped != window);
            tracing::info!(app = logged_app(&window), "window no longer popped out");
            if self.workspaces.is_floating(&window) {
                self.toggle_floating();
            }
            return;
        }
        let Some(index) = self.workspaces.find(&window) else {
            return;
        };
        let Some(area) = self.area_for_workspace(index) else {
            return;
        };
        if !self.workspaces.is_floating(&window) {
            self.take_off_workspace(&window);
            let (min_w, min_h) = min_size(&window);
            // A screen too small for the whole of it keeps a margin of the desktop round it.
            let size = (
                POPPED.0.min(area.w * 4 / 5).max(min_w),
                POPPED.1.min(area.h * 4 / 5).max(min_h),
            );
            let floating = &mut self.workspaces.get_mut(index).floating;
            floating.add_at(window.clone(), centred(area, size), size, area);
            floating.ask(&window, size);
        }
        self.popped.push(window.clone());
        tracing::info!(app = logged_app(&window), "window popped out");
        self.retile();
        self.focus_window(&window);
    }

    /// Before the windows are laid out: each popped window whose screen has gone to another
    /// workspace joins that workspace's floating windows, in front and where it was. One that
    /// was closed, or put back into the tiling by another key, is popped no longer; one that is
    /// minimised or docked meanwhile, or filling the screen, still is when it floats again.
    pub(super) fn follow_popped(&mut self) {
        if self.popped.is_empty() {
            return;
        }
        let (workspaces, refloat) = (&self.workspaces, &self.refloat);
        self.popped.retain(|window| {
            window.alive()
                && (workspaces.is_floating(window)
                    || workspaces.find(window).is_none()
                    || refloat.iter().any(|(held, _)| held == window))
        });
        for window in self.popped.clone() {
            let Some(from) = self.workspaces.find(&window) else {
                continue;
            };
            let Some(to) = self.screens.following(from) else {
                continue;
            };
            if let Some(float) = self.workspaces.get_mut(from).floating.remove(&window) {
                self.workspaces.get_mut(to).floating.put(float);
            }
        }
    }
}
