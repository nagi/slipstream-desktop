//! Run or raise: one key per app goes to that app's own workspace and brings its window forward
//! there, or starts it there if it isn't running.
//!
//! What a key is for comes from `[[run-or-raise.app]]` in the settings (`Target`). Only the
//! target workspace is searched, so a window of the same app somewhere else is left where it is.
//! A launch is remembered for a while (`Launches`), so the window it opens goes to the target
//! workspace even when it comes from an app that was already running somewhere else, as Chrome's
//! and Ghostty's new windows do, and a second press while it starts doesn't start another.

use std::time::{Duration, Instant};

use crate::keys::App;

/// How long a launch waits for its window. A cold start of a big app fits inside it.
pub const LAUNCH_WAITS: Duration = Duration::from_secs(8);

/// Which windows belong to a target: by app id (X11: class), by title, or both.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Matcher {
    /// Lower case, as `window_app_id` gives it. Empty matches any app.
    pub class: String,
    /// Exactly. Empty matches any title.
    pub title: String,
}

impl Matcher {
    pub fn new(class: &str, title: &str) -> Self {
        Self {
            class: class.trim().to_lowercase(),
            title: title.to_string(),
        }
    }

    /// Whether there's anything to match on at all: a matcher with neither can't tell a window of
    /// its app from any other.
    pub fn is_empty(&self) -> bool {
        self.class.is_empty() && self.title.is_empty()
    }

    pub fn matches(&self, app_id: Option<&str>, title: &str) -> bool {
        !self.is_empty()
            && (self.class.is_empty() || app_id == Some(self.class.as_str()))
            && (self.title.is_empty() || title == self.title)
    }
}

/// How a target starts its app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// One of the apps Slipstream already starts for its own keys.
    App(App),
    /// A command line, run with `sh -c`.
    Shell(String),
}

/// One `[[run-or-raise.app]]`, read and resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The workspace's name, looked up each time, since workspaces can be renamed or reordered.
    pub workspace: String,
    pub matcher: Matcher,
    pub start: Start,
}

/// The settings' apps read into targets, one per entry so a binding's index still finds its own;
/// an entry that can't work is `None`, with a warning. `class_of` names the window class of one of
/// Slipstream's own apps, as `launch::app_class` does.
pub fn targets(
    settings: &slipstream_config::RunOrRaise,
    class_of: impl Fn(App) -> Option<String>,
) -> Vec<Option<Target>> {
    settings
        .app
        .iter()
        .map(|entry| {
            let workspace = entry.workspace.trim().to_string();
            let app = match entry.app.trim().to_ascii_lowercase().as_str() {
                "" => None,
                "browser" => Some(App::Browser),
                "terminal" => Some(App::Terminal),
                "files" => Some(App::Files),
                other => {
                    tracing::warn!(
                        app = other,
                        "run-or-raise: app is browser, terminal or files"
                    );
                    return None;
                }
            };
            let class = match (entry.class.trim(), app) {
                ("", Some(app)) => class_of(app).unwrap_or_default(),
                (class, _) => class.to_string(),
            };
            let matcher = Matcher::new(&class, &entry.title);
            let start = match (entry.command.trim(), app) {
                ("", Some(app)) => Start::App(app),
                ("", None) => {
                    tracing::warn!(workspace, "run-or-raise: no command to start");
                    return None;
                }
                (command, _) => Start::Shell(command.to_string()),
            };
            if workspace.is_empty() || matcher.is_empty() {
                tracing::warn!(
                    workspace,
                    "run-or-raise: needs a workspace and a class or title"
                );
                return None;
            }
            Some(Target {
                workspace,
                matcher,
                start,
            })
        })
        .collect()
}

/// The window to bring forward from a workspace's `windows`: the one last focused there if it
/// matches, otherwise the first that does.
pub fn pick<W: PartialEq + Clone>(
    windows: &[W],
    matches: impl Fn(&W) -> bool,
    last_focus: Option<&W>,
) -> Option<W> {
    last_focus
        .filter(|last| windows.contains(last) && matches(last))
        .or_else(|| windows.iter().find(|window| matches(window)))
        .cloned()
}

/// The window a second press goes on to: the matching one after `focused` in the workspace's
/// `windows`, round to the first again. None when `focused` isn't one of them or is the only one,
/// so the press is a first press, for `pick`.
pub fn next<W: PartialEq + Clone>(
    windows: &[W],
    matches: impl Fn(&W) -> bool,
    focused: Option<&W>,
) -> Option<W> {
    let focused = focused.filter(|focused| matches(focused))?;
    let at = windows.iter().position(|window| window == focused)?;
    windows[at + 1..]
        .iter()
        .chain(&windows[..at])
        .find(|window| matches(window))
        .cloned()
}

/// A launch waiting for its window.
#[derive(Debug, Clone)]
struct Launch {
    matcher: Matcher,
    /// The workspace's settings id, which survives renaming and reordering.
    workspace: u32,
    until: Instant,
}

/// Launches waiting for their windows.
#[derive(Debug, Default)]
pub struct Launches {
    waiting: Vec<Launch>,
}

impl Launches {
    /// A launch onto workspace `workspace` (its id) has just started.
    pub fn start(&mut self, matcher: Matcher, workspace: u32, now: Instant) {
        self.forget_old(now);
        self.waiting.push(Launch {
            matcher,
            workspace,
            until: now + LAUNCH_WAITS,
        });
    }

    /// Whether something is still starting for that workspace and app, so another press waits
    /// for it rather than starting a second copy.
    pub fn launching(&self, matcher: &Matcher, workspace: u32, now: Instant) -> bool {
        self.waiting.iter().any(|launch| {
            launch.workspace == workspace && launch.matcher == *matcher && now < launch.until
        })
    }

    /// A new window: the workspace id of the oldest launch it answers, which it is then off the
    /// list for. None when it answers none.
    pub fn claim(&mut self, app_id: Option<&str>, title: &str, now: Instant) -> Option<u32> {
        self.forget_old(now);
        let at = self
            .waiting
            .iter()
            .position(|launch| launch.matcher.matches(app_id, title))?;
        Some(self.waiting.remove(at).workspace)
    }

    /// Whether any launch is still waiting.
    pub fn any(&self, now: Instant) -> bool {
        self.waiting.iter().any(|launch| now < launch.until)
    }

    fn forget_old(&mut self, now: Instant) {
        self.waiting.retain(|launch| now < launch.until);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chrome() -> Matcher {
        Matcher::new("Chromium", "")
    }

    #[test]
    fn a_matcher_needs_the_app_and_the_title_it_names() {
        assert!(chrome().matches(Some("chromium"), "Inbox"));
        assert!(!chrome().matches(Some("chrome-discord.com__-default"), "Discord"));
        assert!(!chrome().matches(None, "Chromium"));
        let lists = Matcher::new("", "Lists");
        assert!(lists.matches(Some("anything"), "Lists"));
        assert!(!lists.matches(Some("anything"), "lists"));
        let both = Matcher::new("emacs", "notes");
        assert!(!both.matches(Some("emacs"), "scratch"));
        assert!(
            !Matcher::default().matches(Some("chromium"), ""),
            "an empty one matches nothing"
        );
    }

    #[test]
    fn the_window_last_focused_on_the_workspace_wins() {
        let windows = ["term-a", "chrome", "term-b"];
        let terminal = |w: &&str| w.starts_with("term");
        assert_eq!(pick(&windows, terminal, None), Some("term-a"));
        assert_eq!(pick(&windows, terminal, Some(&"term-b")), Some("term-b"));
        assert_eq!(
            pick(&windows, terminal, Some(&"chrome")),
            Some("term-a"),
            "unless it's another app's"
        );
        assert_eq!(
            pick(&windows, terminal, Some(&"term-c")),
            Some("term-a"),
            "or on another workspace"
        );
        assert_eq!(pick(&windows, |w: &&str| w.is_empty(), None), None);
    }

    #[test]
    fn a_second_press_goes_round_the_apps_windows() {
        let windows = ["term-a", "chrome", "term-b", "term-c"];
        let terminal = |w: &&str| w.starts_with("term");
        assert_eq!(next(&windows, terminal, Some(&"term-a")), Some("term-b"));
        assert_eq!(next(&windows, terminal, Some(&"term-b")), Some("term-c"));
        assert_eq!(next(&windows, terminal, Some(&"term-c")), Some("term-a"));
        assert_eq!(next(&windows, terminal, None), None);
        assert_eq!(
            next(&windows, terminal, Some(&"chrome")),
            None,
            "from another app's window it's a first press"
        );
        assert_eq!(
            next(&windows, terminal, Some(&"term-d")),
            None,
            "and from another workspace"
        );
        assert_eq!(
            next(&["term-a", "chrome"], terminal, Some(&"term-a")),
            None,
            "the only one stays"
        );
    }

    #[test]
    fn a_new_window_claims_the_launch_it_answers_once() {
        let now = Instant::now();
        let mut launches = Launches::default();
        launches.start(chrome(), 11, now);
        assert!(launches.launching(&chrome(), 11, now));
        assert!(!launches.launching(&chrome(), 12, now), "another workspace");
        assert_eq!(launches.claim(Some("ghostty"), "", now), None);
        assert_eq!(launches.claim(Some("chromium"), "New Tab", now), Some(11));
        assert_eq!(launches.claim(Some("chromium"), "New Tab", now), None);
        assert!(!launches.launching(&chrome(), 11, now));
        assert!(!launches.any(now));
    }

    #[test]
    fn a_launch_stops_waiting_after_a_while() {
        let now = Instant::now();
        let mut launches = Launches::default();
        launches.start(chrome(), 11, now);
        let later = now + LAUNCH_WAITS;
        assert!(!launches.launching(&chrome(), 11, later));
        assert_eq!(launches.claim(Some("chromium"), "", later), None);
    }

    #[test]
    fn two_launches_are_claimed_in_the_order_they_started() {
        let now = Instant::now();
        let mut launches = Launches::default();
        launches.start(chrome(), 11, now);
        launches.start(chrome(), 12, now + Duration::from_millis(10));
        assert_eq!(launches.claim(Some("chromium"), "", now), Some(11));
        assert_eq!(launches.claim(Some("chromium"), "", now), Some(12));
    }

    #[test]
    fn settings_read_into_targets_one_per_entry() {
        use slipstream_config::{RaiseApp, RunOrRaise};
        let settings = RunOrRaise {
            app: vec![
                RaiseApp {
                    workspace: "Chrome".into(),
                    key: "c".into(),
                    app: "browser".into(),
                    ..Default::default()
                },
                RaiseApp {
                    workspace: "Lists".into(),
                    key: "l".into(),
                    class: "chrome-trello.com__b_x-Default".into(),
                    command: "omarchy-launch-webapp https://trello.com/b/x".into(),
                    ..Default::default()
                },
                RaiseApp {
                    workspace: "Nowhere".into(),
                    key: "n".into(),
                    class: "nothing".into(),
                    ..Default::default()
                },
                RaiseApp {
                    workspace: "Odd".into(),
                    key: "o".into(),
                    app: "kettle".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let targets = targets(&settings, |app| {
            (app == App::Browser).then(|| "chromium".to_string())
        });
        assert_eq!(targets.len(), 4, "one per entry, so indices still line up");
        let chrome = targets[0].as_ref().unwrap();
        assert_eq!(chrome.matcher, Matcher::new("chromium", ""));
        assert_eq!(chrome.start, Start::App(App::Browser));
        let lists = targets[1].as_ref().unwrap();
        assert_eq!(lists.matcher.class, "chrome-trello.com__b_x-default");
        assert!(matches!(lists.start, Start::Shell(_)));
        assert_eq!(targets[2], None, "nothing to start");
        assert_eq!(targets[3], None, "no such app");
    }
}
