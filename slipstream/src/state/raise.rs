//! Run or raise (`raise.rs`): the keys, and the windows they launch finding their workspace.

use super::*;

use crate::raise::{self, Start, Target};

impl Slipstream {
    /// The run-or-raise settings changed, or were read at startup: the keys and the apps follow.
    pub fn set_run_or_raise(
        &mut self,
        settings: &slipstream_config::RunOrRaise,
        style: slipstream_config::KeyStyle,
    ) {
        self.bindings = keys::bindings(settings, style);
        self.raise_targets = raise::targets(settings, launch::app_class);
    }

    /// Each run-or-raise app's workspace name, by index, for the shortcut sheet.
    pub fn raise_names(&self) -> Vec<String> {
        self.raise_targets
            .iter()
            .map(|target| {
                target
                    .as_ref()
                    .map(|target| target.workspace.clone())
                    .unwrap_or_default()
            })
            .collect()
    }

    /// The run-or-raise app at `index`'s workspace, by name, or a toast saying it isn't there.
    fn raise_workspace(&mut self, index: u8) -> Option<(Target, usize)> {
        let target = self.raise_targets.get(usize::from(index))?.clone()?;
        let Some(workspace) = self.workspaces.named(&target.workspace) else {
            self.show_toast(
                &format!("No workspace {}", target.workspace),
                "Settings › Workspaces names one.",
            );
            return None;
        };
        Some((target, workspace))
    }

    /// Goes to the app's workspace and brings its window there forward; failing that, one of its
    /// windows minimised into the rain, which comes back there; failing that, starts it, and the
    /// window it opens is put there whichever workspace is on screen when it appears.
    pub fn run_or_raise(&mut self, index: u8) {
        let Some((target, workspace)) = self.raise_workspace(index) else {
            return;
        };
        self.switch_workspace(workspace);
        let matches = |window: &Window| {
            window.alive()
                && target
                    .matcher
                    .matches(window_app_id(window).as_deref(), &window_title(window))
        };
        let on_workspace = self.workspaces.get(workspace);
        let found = raise::pick(
            &on_workspace.windows(),
            matches,
            on_workspace.last_focus.as_ref(),
        );
        if let Some(window) = found {
            self.focus_window(&window);
            return;
        }
        let minimised = self
            .rain
            .streams
            .iter()
            .map(|stream| stream.window.clone())
            .find(|window| matches(window));
        if let Some(window) = minimised {
            self.restore(&window);
            return;
        }
        let now = std::time::Instant::now();
        let id = self.workspaces.get(workspace).id;
        // Pressed again while it's still starting: the window will come here, so wait for it.
        if self.launches.launching(&target.matcher, id, now) {
            return;
        }
        self.concentration.asked(now);
        let started = match &target.start {
            Start::App(app) => {
                self.launch_app(*app);
                // `launch_app` has said why if it couldn't; a window may still come.
                Ok(())
            }
            Start::Shell(line) => match missing_program(line) {
                Some(program) => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no program called {program}"),
                )),
                None => launch::shell(line),
            },
        };
        match started {
            Ok(()) => {
                tracing::info!(workspace = target.workspace, "run-or-raise: starting");
                self.launches.start(target.matcher, id, now);
            }
            Err(err) => {
                let program = line_program(&target.start);
                let (title, body) = launch::couldnt_run(&program, &err);
                self.show_toast(title, &body);
            }
        }
    }

    /// Sends the focused window to the app's workspace, and follows it.
    pub fn send_to_raise_workspace(&mut self, index: u8) {
        if let Some((_, workspace)) = self.raise_workspace(index) {
            self.move_focused_to_workspace(workspace);
        }
    }

    /// A new window a run-or-raise launch is waiting for goes onto that launch's workspace. Returns
    /// whether it claimed one, so `add_window` leaves it alone.
    pub(super) fn claim_launched(&mut self, window: &Window, area: Rect) -> bool {
        let now = std::time::Instant::now();
        if !self.launches.any(now) {
            self.raise_unnamed.clear();
            return false;
        }
        let app = window_app_id(window);
        if app.is_none() {
            // A Wayland window says which app it is on a later commit (`claim_launched_on_commit`).
            if !self.raise_unnamed.contains(window) {
                self.raise_unnamed.push(window.clone());
            }
            return false;
        }
        let Some(id) = self
            .launches
            .claim(app.as_deref(), &window_title(window), now)
        else {
            return false;
        };
        let Some(index) = self.workspaces.index_of_id(id) else {
            return false;
        };
        if self.workspaces.find(window) != Some(index) {
            self.take_off_workspace(window);
            let beside = self.workspaces.get(index).last_focus.clone();
            self.workspaces
                .insert(index, window.clone(), beside.as_ref(), area);
        }
        self.retile();
        if self.screens.showing(index).is_some() {
            self.focus_window(window);
        } else {
            self.workspaces.get_mut(index).last_focus = Some(window.clone());
        }
        tracing::info!(
            app,
            workspace = index + 1,
            "run-or-raise: a launched window found its workspace"
        );
        true
    }

    /// A Wayland window that had no app id when it was tiled says which app it is: if it's one a
    /// launch is waiting for, it moves to that launch's workspace now.
    pub fn claim_launched_on_commit(&mut self, surface: &WlSurface) {
        if self.raise_unnamed.is_empty() {
            return;
        }
        let Some(at) = self
            .raise_unnamed
            .iter()
            .position(|window| window.wl_surface().as_deref() == Some(surface))
        else {
            return;
        };
        let window = self.raise_unnamed[at].clone();
        if !window.alive() {
            self.raise_unnamed.swap_remove(at);
            return;
        }
        if window_app_id(&window).is_none() {
            return;
        }
        self.raise_unnamed.swap_remove(at);
        // Only a window that is tiled somewhere moves; one poured into the rain stays there.
        if self.workspaces.find(&window).is_none() {
            return;
        }
        let Some(area) = self.output_area() else {
            return;
        };
        self.claim_launched(&window, area);
    }
}

/// The program a plain command line starts, when it isn't installed. `sh` would start and then
/// fail where nobody sees it, so this is the one chance to say so. A line with shell syntax in it
/// has no one program to look for, and is let through.
fn missing_program(line: &str) -> Option<String> {
    const SYNTAX: &[char] = &[
        '|', '&', ';', '<', '>', '(', ')', '$', '`', '\\', '"', '\'', '*', '?', '[', '#', '~', '=',
    ];
    let program = line.split_whitespace().next()?;
    if line.contains(SYNTAX) || launch::installed(program) {
        return None;
    }
    Some(program.to_string())
}

/// The program a start runs, for a toast that names it.
fn line_program(start: &Start) -> String {
    match start {
        Start::App(app) => format!("{app:?}"),
        Start::Shell(line) => line.split_whitespace().next().unwrap_or(line).to_string(),
    }
}
