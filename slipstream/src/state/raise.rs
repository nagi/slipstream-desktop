//! Run or raise (`raise.rs`): the keys, and the windows they launch finding their workspace.

use super::*;

use crate::raise::{self, Matcher, Start, Target};

/// The workspace reading mode keeps the reader on, by name. Without one, the reader opens on
/// whichever workspace is on screen.
pub const READING: &str = "Reading";

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

    /// Goes to the app's workspace and brings its window there forward, or the next of its windows
    /// there when one already has the keyboard; failing that, one of its windows minimised into
    /// the rain, which comes back there; failing that, starts it, and the window it opens is put
    /// there whichever workspace is on screen when it appears.
    pub fn run_or_raise(&mut self, index: u8) {
        let Some((target, workspace)) = self.raise_workspace(index) else {
            return;
        };
        self.raise_on(target, workspace);
    }

    /// Run or raise `target` on `workspace`, as `run_or_raise` describes.
    fn raise_on(&mut self, target: Target, workspace: usize) {
        // Pressed with one of its windows already forward: the press goes on to the next.
        let focused = self.focused_window();
        self.switch_workspace(workspace);
        let matches = |window: &Window| {
            window.alive()
                && target
                    .matcher
                    .matches(window_app_id(window).as_deref(), &window_title(window))
        };
        let on_workspace = self.workspaces.get(workspace);
        let windows = on_workspace.windows();
        let found = raise::next(&windows, matches, focused.as_ref())
            .or_else(|| raise::pick(&windows, matches, on_workspace.last_focus.as_ref()));
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

    /// Super+R: reading mode. The reader is run or raised on the Reading workspace, full screen,
    /// and a reader left on any other workspace is brought there first.
    pub fn reading_mode(&mut self) {
        let Some(reading) = self.workspaces.named(READING) else {
            // No Reading workspace: the reader's window wherever it is, or a new one here.
            let open = self.all_open_windows().into_iter().find(is_reader);
            match open {
                Some(window) => self.activate_window(&window),
                None => self.launch_app(crate::keys::App::Reader),
            }
            return;
        };
        let strays: Vec<Window> = self
            .workspaces
            .all_windows()
            .into_iter()
            .filter(|window| is_reader(window) && self.workspaces.find(window) != Some(reading))
            .collect();
        if let Some(area) = self.output_area() {
            for window in strays {
                self.take_off_workspace(&window);
                self.workspaces.insert(reading, window, None, area);
            }
            self.retile();
        }
        let target = Target {
            workspace: READING.to_string(),
            matcher: Matcher::new(launch::READER_ID, ""),
            start: Start::App(crate::keys::App::Reader),
        };
        self.raise_on(target, reading);
    }

    /// A new reader window goes to the Reading workspace, however it was started: by Super+R, or
    /// by opening a book from the file manager. Returns whether it was taken there.
    pub(super) fn claim_reader(&mut self, window: &Window, area: Rect) -> bool {
        if !is_reader(window) {
            return false;
        }
        let Some(reading) = self.workspaces.named(READING) else {
            return false;
        };
        if self.workspaces.find(window) != Some(reading) {
            self.take_off_workspace(window);
            self.workspaces.insert(reading, window.clone(), None, area);
        }
        self.retile();
        if self.screens.showing(reading).is_none() {
            self.switch_workspace(reading);
        }
        self.focus_window(window);
        tracing::info!(workspace = reading + 1, "reading mode: the reader is on its workspace");
        true
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

/// Whether `window` is the reader's.
fn is_reader(window: &Window) -> bool {
    window_app_id(window).is_some_and(|id| id.eq_ignore_ascii_case(launch::READER_ID))
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
