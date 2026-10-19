//! Three-finger swipes on a touchpad: left and right step between workspaces (in bullet time,
//! the workspace being looked at), up opens bullet time, and down (or up again) leaves it.
//! Swipes with any other number of fingers go to the window under the pointer, as they always
//! have.
//!
//! A swipe acts once, as soon as it has gone far enough in one direction, so the workspace
//! changes under the fingers rather than when they lift. The rest of that swipe is ignored.

use crate::Slipstream;

/// The fingers a swipe needs to be ours.
pub const FINGERS: u32 = 3;
/// How far a swipe travels, in libinput's pointer units, before it counts.
const DISTANCE: f64 = 60.0;
/// How much further it has to have gone along its direction than across it.
const DOMINANCE: f64 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// A three-finger swipe under way.
#[derive(Debug, Default)]
pub struct Swipe {
    travelled: (f64, f64),
    done: bool,
}

impl Swipe {
    /// The fingers moved by (`dx`, `dy`): the direction, the first time the swipe goes far
    /// enough in one.
    pub fn moved(&mut self, dx: f64, dy: f64) -> Option<Direction> {
        if self.done {
            return None;
        }
        self.travelled.0 += dx;
        self.travelled.1 += dy;
        let (x, y) = self.travelled;
        let direction = if x.abs() >= DISTANCE && x.abs() >= y.abs() * DOMINANCE {
            if x < 0.0 {
                Direction::Left
            } else {
                Direction::Right
            }
        } else if y.abs() >= DISTANCE && y.abs() >= x.abs() * DOMINANCE {
            if y < 0.0 {
                Direction::Up
            } else {
                Direction::Down
            }
        } else {
            return None;
        };
        self.done = true;
        Some(direction)
    }
}

impl Slipstream {
    /// Whether a swipe starting now with `fingers` is the desktop's rather than the app's. The
    /// lock screen and an app holding the shortcuts keep them, as they keep the keys.
    pub(super) fn takes_swipe(&self, fingers: u32) -> bool {
        fingers == FINGERS && self.lock.is_none() && !self.shortcuts_inhibited()
    }

    /// A three-finger swipe went far enough. Fingers move the desktop as they'd move a page:
    /// swiping left brings in the workspace to the right. In bullet time that's the workspace
    /// being looked at, with its most recent window chosen, as its number key does.
    pub fn swiped(&mut self, direction: Direction) {
        tracing::info!(?direction, "three-finger swipe");
        let step: i32 = match direction {
            Direction::Up => return self.toggle_bullet_time(),
            Direction::Down if self.bullet.is_some() => return self.toggle_bullet_time(),
            Direction::Down => return,
            Direction::Left => 1,
            Direction::Right => -1,
        };
        match self.bullet.as_ref().map(|mode| mode.view) {
            Some(view) => {
                let to = view as i32 + step;
                if (0..self.workspaces.count() as i32).contains(&to) {
                    self.bullet_pan(to as usize, true);
                }
            }
            None => self.switch_workspace_by(step),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swipe_acts_once_it_has_gone_far_enough() {
        let mut swipe = Swipe::default();
        assert_eq!(swipe.moved(-30.0, 2.0), None);
        assert_eq!(swipe.moved(-35.0, 3.0), Some(Direction::Left));
        assert_eq!(swipe.moved(-200.0, 0.0), None, "only once a swipe");
    }

    #[test]
    fn a_diagonal_swipe_is_nothing() {
        let mut swipe = Swipe::default();
        assert_eq!(swipe.moved(70.0, -70.0), None);
        assert_eq!(swipe.moved(0.0, -60.0), Some(Direction::Up));
    }

    #[test]
    fn each_direction_is_read() {
        for (dx, dy, direction) in [
            (80.0, 0.0, Direction::Right),
            (-80.0, 0.0, Direction::Left),
            (0.0, -80.0, Direction::Up),
            (0.0, 80.0, Direction::Down),
        ] {
            assert_eq!(Swipe::default().moved(dx, dy), Some(direction));
        }
    }
}
