//! Agents, the plugin behind the agents panel. It keeps the usage records Omarchy's
//! `omarchy-agent-usage-update` writes current, and prints them on its card as `agents` for the
//! compositor's panel to draw, which it does for this plugin alone (`slipstream_config::agents`).
//! The card has nothing else on it, and the plugin panel leaves it out.
//!
//! - `agents` keeps running: it reads the records every minute, picking up whatever else wrote
//!   them, runs the updater every quarter of an hour, and prints a card whenever what it holds
//!   changes, `updating` included.
//! - `agents press limits` and `agents press full` run beside it when the panel opens or R is
//!   pressed, and only leave the press in its own folder (`asked/`) for the running one: the
//!   limits again, at most every 30 seconds, or everything.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, ExitCode, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use slipstream_config::agents::{self, AgentsCard};

/// How often the records are made again.
const EVERY: Duration = Duration::from_secs(15 * 60);
/// How often they are read again, to pick up whatever else wrote them.
const RESCAN: Duration = Duration::from_secs(60);
/// Opening the panel asks for fresh limits, but not more often than this.
const LIMITS_SPACING: Duration = Duration::from_secs(30);
/// An update still running after this long is stopped.
const UPDATE_LIMIT: Duration = Duration::from_secs(180);
/// How often the program that keeps running looks for presses and at its update.
const LOOK: Duration = Duration::from_millis(250);
/// Where presses wait for the program that keeps running, in its own folder.
const ASKED: &str = "asked";

/// How a refresh is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refresh {
    /// What the collectors reuse their recent scans for: the limits that go stale on the wire.
    Limits,
    /// Everything again.
    Full,
}

impl Refresh {
    fn from_id(id: &str) -> Option<Refresh> {
        match id {
            "limits" => Some(Refresh::Limits),
            "full" => Some(Refresh::Full),
            _ => None,
        }
    }

    /// The more of the two.
    fn and(self, other: Refresh) -> Refresh {
        if self == Refresh::Full || other == Refresh::Full {
            Refresh::Full
        } else {
            Refresh::Limits
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match args.as_slice() {
        [] => keep_running(),
        ["press", id] => press(id),
        _ => Err("run it as agents, or agents press limits|full".into()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("agents: {why}");
            ExitCode::FAILURE
        }
    }
}

/// The plugin's own folder, which Slipstream names.
fn own_folder() -> Result<PathBuf, String> {
    std::env::var_os("SLIPSTREAM_PLUGIN_STATE")
        .map(PathBuf::from)
        .ok_or_else(|| "SLIPSTREAM_PLUGIN_STATE isn't set: Slipstream sets it for a plugin".into())
}

/// Leaves `id` for the program that keeps running, which acts on it within a quarter of a second.
fn press(id: &str) -> Result<(), String> {
    if Refresh::from_id(id).is_none() {
        return Err(format!("nothing is pressed as {id:?}"));
    }
    let asked = own_folder()?.join(ASKED);
    fs::create_dir_all(&asked).map_err(|err| err.to_string())?;
    // Written under a hidden name first, so a half-written one is never read.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let name = format!("{nanos:024}-{}", std::process::id());
    let partial = asked.join(format!(".{name}"));
    fs::write(&partial, id).map_err(|err| err.to_string())?;
    fs::rename(&partial, asked.join(name)).map_err(|err| err.to_string())
}

/// The refresh the presses waiting in `folder` add up to, each taken away as it's read.
fn take_asked(folder: &Path) -> Option<Refresh> {
    let entries = fs::read_dir(folder).ok()?;
    entries
        .flatten()
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|entry| {
            let id = fs::read_to_string(entry.path()).ok();
            let _ = fs::remove_file(entry.path());
            Refresh::from_id(id?.trim())
        })
        .reduce(Refresh::and)
}

/// Omarchy's updater, started: `how` is a kind of refresh, or `None` for the scheduled one.
fn start_update(how: Option<Refresh>) -> std::io::Result<Child> {
    let mut command = Command::new("omarchy-agent-usage-update");
    match how {
        Some(Refresh::Limits) => {
            command.arg("--limits-only");
        }
        Some(Refresh::Full) => {
            command.arg("--force");
        }
        None => {}
    }
    // The updater finds its collectors from here; a session started outside Omarchy's own
    // wouldn't have it.
    if std::env::var_os("OMARCHY_PATH").is_none() {
        command.env("OMARCHY_PATH", "/usr/share/omarchy");
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
}

/// An update under way.
struct Updating {
    child: Child,
    started: Instant,
}

/// The card for `card`: the panel's data and nothing else, as one line.
fn card_line(card: &AgentsCard) -> String {
    serde_json::json!({ "agents": card }).to_string()
}

/// The program that keeps running.
fn keep_running() -> Result<(), String> {
    let asked = own_folder()?.join(ASKED);
    let dir = agents::usage_dir();
    let mut card = AgentsCard::default();
    let mut printed: Option<String> = None;
    let mut next_update = Instant::now();
    let mut next_rescan = Instant::now();
    let mut last_update: Option<Instant> = None;
    let mut wanted: Option<Refresh> = None;
    let mut updating: Option<Updating> = None;
    let mut warned = false;
    loop {
        if let Some(how) = take_asked(&asked) {
            wanted = Some(wanted.map_or(how, |was| was.and(how)));
        }
        // An update that's finished, or run too long, ends here.
        if let Some(update) = updating.as_mut() {
            let ended = match update.child.try_wait() {
                Ok(Some(status)) if status.success() => Some(Ok(())),
                Ok(Some(status)) => Some(Err(format!("{status}: {}", said(&mut update.child)))),
                Ok(None) if update.started.elapsed() >= UPDATE_LIMIT => {
                    let _ = update.child.kill();
                    let _ = update.child.wait();
                    Some(Err(format!(
                        "still running after {} s",
                        UPDATE_LIMIT.as_secs()
                    )))
                }
                Ok(None) => None,
                Err(err) => Some(Err(err.to_string())),
            };
            if let Some(ended) = ended {
                match ended {
                    Ok(()) => warned = false,
                    Err(why) if !warned => {
                        eprintln!("agents: couldn't update the agents' usage: {why}");
                        warned = true;
                    }
                    Err(_) => {}
                }
                updating = None;
                last_update = Some(Instant::now());
                next_update = Instant::now() + EVERY;
                next_rescan = Instant::now();
            }
        }
        // Start one when it's due or asked for, unless one runs; asked-for limits wait for their
        // spacing.
        if updating.is_none() {
            let spaced = last_update.is_none_or(|at| at.elapsed() >= LIMITS_SPACING);
            let how = match wanted {
                Some(Refresh::Limits) if !spaced => None,
                Some(how) => Some(Some(how)),
                None if Instant::now() >= next_update => Some(None),
                None => None,
            };
            if let Some(how) = how {
                wanted = None;
                match start_update(how) {
                    Ok(child) => {
                        updating = Some(Updating {
                            child,
                            started: Instant::now(),
                        })
                    }
                    Err(err) => {
                        if !warned {
                            eprintln!("agents: couldn't start omarchy-agent-usage-update: {err}");
                        }
                        warned = true;
                        last_update = Some(Instant::now());
                        next_update = Instant::now() + EVERY;
                    }
                }
            }
        }
        if Instant::now() >= next_rescan {
            card.tools = dir.as_deref().map(agents::load).unwrap_or_default();
            next_rescan = Instant::now() + RESCAN;
        }
        card.updating = updating.is_some();
        let line = card_line(&card);
        if printed.as_ref() != Some(&line) {
            println!("{line}");
            printed = Some(line);
        }
        std::thread::sleep(LOOK);
    }
}

/// The last line an update that failed wrote to its standard error.
fn said(child: &mut Child) -> String {
    use std::io::Read;
    let mut text = String::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut text);
    }
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_manifest_reads_as_slipstream_reads_it() {
        use slipstream_config::plugins::{self, Need};
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest = plugins::read(dir).unwrap().manifest;
        assert_eq!(manifest.name, "Agents");
        assert_eq!(manifest.needs, [Need::Everything]);
    }

    #[test]
    fn a_full_refresh_wins_over_limits() {
        assert_eq!(Refresh::Limits.and(Refresh::Full), Refresh::Full);
        assert_eq!(Refresh::Limits.and(Refresh::Limits), Refresh::Limits);
        assert_eq!(Refresh::from_id("nope"), None);
    }

    #[test]
    fn presses_add_up_and_are_taken_once() {
        let dir = std::env::temp_dir().join(format!("agents-asked-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("1"), "limits").unwrap();
        fs::write(dir.join("2"), "full").unwrap();
        fs::write(dir.join(".3"), "full").unwrap();
        assert_eq!(take_asked(&dir), Some(Refresh::Full));
        assert_eq!(take_asked(&dir), None, "taken away as they were read");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_card_fits_on_one_line_slipstream_reads() {
        let line = card_line(&AgentsCard::default());
        assert!(!line.contains('\n'));
        let card = slipstream_config::plugins::Card::parse(&line, false, 0).unwrap();
        assert_eq!(card.agents, Some(AgentsCard::default()));
        assert!(card.lines.is_empty() && card.sections.is_empty());
    }
}
