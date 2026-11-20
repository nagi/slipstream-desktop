//! Run or raise: a key per app that goes to its own workspace.

use serde::{Deserialize, Serialize};

/// Run or raise: a key per app that goes to the app's own workspace and brings its window
/// forward there, or starts it there if it isn't running.
///
/// ```toml
/// [run-or-raise]
/// raise = "ctrl+super+alt"
///
/// [[run-or-raise.app]]
/// workspace = "Chrome"
/// key = "c"
/// app = "browser"
/// command = "chromium --new-window"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RunOrRaise {
    /// The modifiers that raise, joined with `+`: any of `super`, `ctrl`, `alt` and `shift`. The
    /// same chord with Shift added sends the focused window to the app's workspace, and with a
    /// digit goes to (or sends to) a workspace by its number.
    pub raise: String,
    pub app: Vec<RaiseApp>,
}

impl Default for RunOrRaise {
    /// Ctrl+Super+Alt, the "hyper" chord a keyd layer on one key usually makes, and no apps.
    fn default() -> Self {
        Self {
            raise: "ctrl+super+alt".into(),
            app: Vec::new(),
        }
    }
}

/// One app's key.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RaiseApp {
    /// The workspace it lives on, by name, as in `[[workspaces.list]]`.
    pub workspace: String,
    /// The key, by its xkb name: `c`, `Return`, `F5`.
    pub key: String,
    /// `browser`, `terminal` or `files`: the one Slipstream already starts for those, whatever is
    /// set as the default, with its window class worked out to match. Empty uses `class`,
    /// `title` and `command`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub app: String,
    /// The window's app id (X11: class), compared ignoring case.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub class: String,
    /// The window's title, exactly, for apps whose class isn't enough to tell them apart.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// Run with `sh -c` when no window matches. With `app` set, empty starts that app.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub command: String,
    /// Every new window of the app goes to its workspace, however it was started (a book opened
    /// from the file manager, say), and you go there with it. Without it, only the windows its
    /// key starts do.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub every_window: bool,
}

#[cfg(test)]
mod tests {
    use crate::file::parse;

    #[test]
    fn run_or_raise_apps_and_home_screens_read_and_write_back_the_same() {
        let text = r#"
            [[workspaces.list]]
            id = 1
            name = " Chrome "
            screen = " DP-1 "

            [run-or-raise]
            raise = "ctrl+super+alt"

            [[run-or-raise.app]]
            workspace = "Chrome"
            key = "c"
            app = "browser"

            [[run-or-raise.app]]
            workspace = "Lists"
            key = "l"
            class = "chrome-trello.com__b_x-Default"
            command = "omarchy-launch-webapp https://trello.com/b/x"

            [[run-or-raise.app]]
            workspace = "Reading"
            key = "r"
            class = "io.github.peterwalker78.SlipstreamReader"
            command = "slipstream-reader"
            every-window = true
        "#;
        let settings = parse(text).unwrap();
        let entries = settings.workspaces.entries();
        assert_eq!(entries[0].name, "Chrome");
        assert_eq!(entries[0].screen, "DP-1");
        assert_eq!(settings.run_or_raise.app.len(), 3);
        assert_eq!(settings.run_or_raise.app[0].app, "browser");
        assert_eq!(settings.run_or_raise.app[1].key, "l");
        assert!(!settings.run_or_raise.app[1].every_window);
        assert!(settings.run_or_raise.app[2].every_window);
        let written = toml::to_string(&settings).unwrap();
        assert_eq!(
            written.matches("every-window").count(),
            1,
            "written only when set"
        );
        let again = parse(&written).unwrap();
        assert_eq!(again, settings);
    }

    #[test]
    fn with_no_run_or_raise_section_the_chord_is_hyper_and_there_are_no_apps() {
        let settings = parse("").unwrap();
        assert_eq!(settings.run_or_raise.raise, "ctrl+super+alt");
        assert!(settings.run_or_raise.app.is_empty());
        assert!(
            !toml::to_string(&settings.workspaces)
                .unwrap()
                .contains("screen")
        );
    }
}
