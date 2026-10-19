//! The agents panel, from the icon right of the bar's clock: how much of each AI coding tool's
//! allowance is used, and what it has been used for. A port of Omarchy's agents bar panel, which
//! draws what `omarchy-agent-usage-update` records (see `slipstream_config::agents`).
//!
//! For each tool: the plan it runs on, the percentage used of each allowance with the time until
//! it starts over, a prepaid account's credit, the tokens spent each day of the last week, and
//! the busiest models. The numbers are read on a thread of its own, which also runs the update
//! every quarter of an hour, and when the panel opens, since limits go stale on the wire.
//!
//! While it's open it takes every key that isn't a binding: ←/→ (or H/L, or Tab) switch tool,
//! R or Enter refresh, Esc closes. The icon shows only once some tool has numbers.

use std::{
    path::Path,
    sync::{
        Arc, Mutex, OnceLock, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    thread::Thread,
    time::{Duration, Instant},
};

use slipstream_config::{
    agents::{self, Agent, Day, Limit},
    meter::{Run, unix_now},
};
use smithay::{
    backend::renderer::{ImportMem, Renderer, element::memory::MemoryRenderBufferRenderElement},
    input::keyboard::Keysym,
    utils::{Logical, Point, Rectangle, Size},
};

use crate::{
    Slipstream, bar,
    calendar::{self, Date},
    meter,
    paint::Painted,
    palette,
    panel::{self, DESIGN_PX, GAP, PADDING, ink},
    text::{self, Face, Style},
};

/// How often the records are made again.
const EVERY: Duration = Duration::from_secs(15 * 60);
/// How often they are read again, to pick up whatever else wrote them.
const RESCAN: Duration = Duration::from_secs(60);
/// Opening the panel asks for fresh limits, but not more often than this.
const LIMITS_SPACING: Duration = Duration::from_secs(30);
/// An update still running after this long is stopped.
const UPDATE_LIMIT: Duration = Duration::from_secs(180);

// Sizes in design pixels.
const WIDTH: f32 = 440.0;
const HERO_H: f32 = 48.0;
const CHIPS_H: f32 = 30.0;
const TITLE_H: f32 = 24.0;
const LIMIT_H: f32 = 54.0;
/// What a weekly limit's row grows by for the bar of the week gone by.
const WEEK_BAR_H: f32 = 10.0;
/// A weekly limit's window.
const WEEK_SECS: i64 = 7 * 24 * 60 * 60;
const DAY_H: f32 = 26.0;
const MODEL_H: f32 = 52.0;
const BALANCE_H: f32 = 66.0;
const FOOT_H: f32 = 40.0;
const EMPTY_H: f32 = 56.0;

/// How a refresh is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refresh {
    /// What the collectors reuse their recent scans for: the limits that go stale on the wire.
    Limits,
    /// Everything again.
    Full,
}

static ASKED: Mutex<Option<Refresh>> = Mutex::new(None);
static READER: OnceLock<Thread> = OnceLock::new();
static UPDATING: AtomicBool = AtomicBool::new(false);

/// Asks the reader for a refresh now.
pub fn refresh(how: Refresh) {
    let mut asked = ASKED.lock().unwrap_or_else(PoisonError::into_inner);
    *asked = match (*asked, how) {
        (Some(Refresh::Full), _) | (_, Refresh::Full) => Some(Refresh::Full),
        _ => Some(Refresh::Limits),
    };
    drop(asked);
    if let Some(reader) = READER.get() {
        reader.unpark();
    }
}

/// Whether an update is running.
pub fn updating() -> bool {
    UPDATING.load(Ordering::Relaxed)
}

/// Starts the reader: the agents with numbers to show, kept current.
pub fn start() -> Arc<Mutex<Vec<Agent>>> {
    let shared = Arc::new(Mutex::new(Vec::new()));
    let writer = shared.clone();
    let reader = std::thread::spawn(move || {
        let dir = agents::usage_dir();
        let mut next_update = Instant::now();
        let mut last_update: Option<Instant> = None;
        let mut warned = false;
        loop {
            // Whatever is on disk now first, so the icon needn't wait for an update.
            reload(dir.as_deref(), &writer);
            let asked = ASKED.lock().unwrap_or_else(PoisonError::into_inner).take();
            let spaced = last_update.is_none_or(|at| at.elapsed() >= LIMITS_SPACING);
            let how = match asked {
                Some(Refresh::Limits) if !spaced => None,
                Some(how) => Some(Some(how)),
                None if Instant::now() >= next_update => Some(None),
                None => None,
            };
            if let Some(how) = how {
                UPDATING.store(true, Ordering::Relaxed);
                if let Err(why) = update(how) {
                    if !warned {
                        tracing::warn!("couldn't update the agents' usage: {why}");
                    }
                    warned = true;
                } else {
                    warned = false;
                }
                UPDATING.store(false, Ordering::Relaxed);
                last_update = Some(Instant::now());
                next_update = Instant::now() + EVERY;
                reload(dir.as_deref(), &writer);
            }
            std::thread::park_timeout(
                RESCAN.min(next_update.saturating_duration_since(Instant::now())),
            );
        }
    });
    let _ = READER.set(reader.thread().clone());
    shared
}

fn reload(dir: Option<&Path>, shared: &Mutex<Vec<Agent>>) {
    let fresh = dir.map(agents::load).unwrap_or_default();
    let mut current = shared.lock().unwrap_or_else(PoisonError::into_inner);
    if *current != fresh {
        *current = fresh;
    }
}

/// Runs Omarchy's updater: `how` is a kind of refresh, or `None` for the scheduled one.
fn update(how: Option<Refresh>) -> Result<(), String> {
    let mut command = crate::launch::command("omarchy-agent-usage-update");
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
    match slipstream_config::meter::run(command, UPDATE_LIMIT) {
        Run::Printed(_) => Ok(()),
        Run::Failed { code, error } => Err(format!("exit {code:?}: {error}")),
        Run::TimedOut => Err(format!("still running after {} s", UPDATE_LIMIT.as_secs())),
        Run::CouldntStart(err) => Err(err),
    }
}

/// What a key or click asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Nothing,
    Close,
    Refresh,
}

/// Everything the panel shows that it doesn't own.
#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    pub agents: Vec<Agent>,
    pub today: Option<Date>,
    /// The minute, since the Unix epoch, so that countdowns move while it's open.
    pub minute: i64,
    pub updating: bool,
    pub reduced_motion: bool,
    pub ring: u32,
}

#[derive(Clone, PartialEq)]
struct Look {
    facts: Facts,
    chosen: usize,
    width: i32,
    scale: f64,
}

#[derive(Default)]
pub struct Agents {
    open: bool,
    /// When it opened, on the animation clock.
    opened_at: f64,
    /// The tool shown, by id: a tool arriving while it's open mustn't change what's read.
    chosen: String,
    shown: Option<Look>,
    painted: Option<Painted>,
    origin: Point<f64, Logical>,
    frame: Rectangle<f64, Logical>,
    chips: Vec<(usize, Rectangle<f64, Logical>)>,
    /// Opens with a short fade instead of rising (`Slipstream::set_reduced_motion`).
    pub reduced_motion: bool,
}

/// One part of the card.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Block {
    Hero,
    Chips,
    Problem(usize),
    Balance,
    /// The limits, and how many of them are weekly ones with a bar for the week gone by.
    Limits(usize, usize),
    Days(usize, bool),
    Models(usize),
    Nothing,
}

impl Block {
    fn height(self) -> f32 {
        match self {
            Block::Hero => HERO_H,
            Block::Chips => CHIPS_H,
            Block::Problem(lines) => 22.0 + lines as f32 * 18.0,
            Block::Balance => BALANCE_H,
            Block::Limits(n, weeks) => TITLE_H + n as f32 * LIMIT_H + weeks as f32 * WEEK_BAR_H,
            Block::Days(n, today_line) => {
                TITLE_H + n as f32 * DAY_H + if today_line { 22.0 } else { 0.0 }
            }
            Block::Models(n) => TITLE_H + n as f32 * MODEL_H,
            Block::Nothing => EMPTY_H,
        }
    }
}

/// How `text` breaks into lines of at most `room`, up to `most` of them, the last cut short.
fn wrap(text: &str, style: &Style, room: f32, most: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let joined = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if line.is_empty() || text::width(&joined, style) <= room {
            line = joined;
        } else {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.len() > most {
        lines.truncate(most);
        let last = lines.pop().unwrap_or_default();
        lines.push(format!("{}…", last.trim_end_matches('…')));
    }
    lines
        .into_iter()
        .map(|line| text::ellipsize(&line, style, room))
        .collect()
}

fn help_style() -> Style {
    Style::new(Face::Body, 13.0, panel::hint())
}

/// The blocks of one tool's card, top to bottom. `room` is the width text has inside it.
fn blocks(agent: &Agent, several: bool, room: f32) -> Vec<Block> {
    let mut blocks = vec![Block::Hero];
    if several {
        blocks.push(Block::Chips);
    }
    if agent.problem.is_some() {
        let help = agent.help.as_deref().unwrap_or("");
        let lines = wrap(help, &help_style(), room - 24.0, 3).len();
        blocks.push(Block::Problem(lines));
    }
    if agent.balance.is_some() {
        blocks.push(Block::Balance);
    }
    if !agent.limits.is_empty() {
        let weeks = agent.limits.iter().filter(|limit| is_weekly(limit)).count();
        blocks.push(Block::Limits(agent.limits.len(), weeks));
    }
    if !agent.days.is_empty() {
        blocks.push(Block::Days(agent.days.len(), agent.counts_prompts));
    }
    if !agent.models.is_empty() {
        blocks.push(Block::Models(agent.models.len()));
    }
    blocks
}

/// The weekday's three letters for a `2026-10-14`.
fn day_name(date: &str) -> String {
    let mut parts = date.split('-').map(|part| part.parse::<u32>().ok());
    if let (Some(Some(year)), Some(Some(month)), Some(Some(day))) =
        (parts.next(), parts.next(), parts.next())
        && (1..=12).contains(&month)
        && (1..=31).contains(&day)
    {
        let name = calendar::WEEKDAYS[calendar::weekday(Date {
            year: year as i32,
            month,
            day,
        }) as usize];
        return name[..3].to_string();
    }
    date.to_string()
}

fn is_today(days: &[Day], at: usize, today: Option<Date>) -> bool {
    match today {
        Some(today) => {
            days[at].date == format!("{:04}-{:02}-{:02}", today.year, today.month, today.day)
        }
        None => at + 1 == days.len(),
    }
}

/// A rounded track with `fraction` of it filled.
fn meter_bar(
    p: &mut crate::paint::Painter,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    fraction: f64,
    ink: u32,
) {
    p.fill(x, y, w, h, h / 2.0, palette::tone(0xffffff1f));
    let filled = w * fraction.clamp(0.0, 1.0) as f32;
    if filled > 0.0 {
        p.fill(x, y, filled.max(h), h, h / 2.0, ink);
    }
}

/// Whether `limit` is a week's allowance that says when it starts over, and so gets the bar of the
/// week gone by.
fn is_weekly(limit: &Limit) -> bool {
    limit.resets.is_some() && limit.title.to_lowercase().contains("weekly")
}

/// How far through its week a weekly limit is at `now`, 0 to 1. The week is the seven days up to
/// when it resets, not the calendar's.
fn week_gone(resets: i64, now: i64) -> f64 {
    let left = (resets - now).clamp(0, WEEK_SECS);
    1.0 - left as f64 / WEEK_SECS as f64
}

fn percent_of(limit: &Limit) -> u8 {
    (limit.used * 100.0).round().clamp(0.0, 255.0) as u8
}

impl Agents {
    /// Paints its text again next frame: a face for characters it lacked has landed.
    pub fn forget_painted_text(&mut self) {
        self.shown = None;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, now: f64) {
        self.open = true;
        self.opened_at = now;
        self.shown = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.shown = None;
        self.painted = None;
        self.chips.clear();
    }

    fn index(&self, facts: &Facts) -> usize {
        facts
            .agents
            .iter()
            .position(|agent| agent.id == self.chosen)
            .unwrap_or(0)
    }

    fn choose(&mut self, facts: &Facts, to: isize) {
        let count = facts.agents.len() as isize;
        if count > 0 {
            let at = (self.index(facts) as isize + to).rem_euclid(count) as usize;
            self.chosen = facts.agents[at].id.clone();
        }
    }

    /// A key while it's open.
    pub fn key(&mut self, sym: Keysym, shift: bool, facts: &Facts) -> Request {
        match sym {
            Keysym::Escape => return Request::Close,
            Keysym::Left | Keysym::h | Keysym::H | Keysym::ISO_Left_Tab => self.choose(facts, -1),
            Keysym::Tab if shift => self.choose(facts, -1),
            Keysym::Right | Keysym::l | Keysym::L | Keysym::Tab => self.choose(facts, 1),
            Keysym::Home => self.choose(facts, -(facts.agents.len() as isize)),
            Keysym::r | Keysym::R | Keysym::Return | Keysym::KP_Enter => return Request::Refresh,
            _ => {}
        }
        Request::Nothing
    }

    /// A click, in logical pixels from the output's corner. Outside the card it closes.
    pub fn click(&mut self, x: f64, y: f64, facts: &Facts) -> Request {
        if let Some((at, _)) = self.chips.iter().find(|(_, area)| area.contains((x, y))) {
            if let Some(agent) = facts.agents.get(*at) {
                self.chosen = agent.id.clone();
            }
            return Request::Nothing;
        }
        if self.frame.contains((x, y)) {
            Request::Nothing
        } else {
            Request::Close
        }
    }

    /// The panel for a screen `width` logical pixels wide at `scale`, repainted when what it
    /// shows changes.
    pub fn element<R>(
        &mut self,
        renderer: &mut R,
        width: i32,
        scale: f64,
        now: f64,
        facts: Facts,
    ) -> Option<MemoryRenderBufferRenderElement<R>>
    where
        R: Renderer + ImportMem,
        R::TextureId: Send + Clone + 'static,
    {
        if !self.open {
            return None;
        }
        let reduced_motion = facts.reduced_motion;
        let look = Look {
            chosen: self.index(&facts),
            facts,
            width,
            scale,
        };
        if self.shown.as_ref() != Some(&look) {
            self.repaint(&look)?;
            self.shown = Some(look);
        }
        let (alpha, rise) = panel::opening(now - self.opened_at, reduced_motion);
        let at = Point::from((self.origin.x, self.origin.y + rise * DESIGN_PX as f64));
        self.painted.as_ref()?.element(renderer, at, alpha)
    }

    fn repaint(&mut self, look: &Look) -> Option<()> {
        let facts = &look.facts;
        let agent = facts.agents.get(look.chosen);
        let inner = WIDTH - 2.0 * PADDING;
        let plan = agent.map(|agent| blocks(agent, facts.agents.len() > 1, inner));
        let plan = plan.unwrap_or_else(|| vec![Block::Nothing]);
        let body: f32 = plan.iter().map(|block| block.height()).sum::<f32>()
            + GAP * plan.len().saturating_sub(1) as f32;
        let height = PADDING + body + GAP + FOOT_H + PADDING;
        let (fx, fy) = (panel::MARGIN, panel::MARGIN);
        // The card's left edge on screen, in design pixels: under the clock and its icon.
        let left = ((look.width as f32 / DESIGN_PX - WIDTH) / 2.0).max(panel::RIGHT);
        let on_screen = |x: f32, y: f32, w: f32, h: f32| {
            Rectangle::<f64, Logical>::new(
                (
                    ((left + x - fx) * DESIGN_PX) as f64,
                    ((panel::TOP + y - fy) * DESIGN_PX) as f64,
                )
                    .into(),
                ((w * DESIGN_PX) as f64, (h * DESIGN_PX) as f64).into(),
            )
        };
        let logical = Size::<i32, Logical>::from((
            ((WIDTH + 2.0 * panel::MARGIN) * DESIGN_PX).ceil() as i32,
            ((height + 2.0 * panel::MARGIN + panel::BELOW) * DESIGN_PX).ceil() as i32,
        ));
        let mut chips = Vec::new();

        let painted = Painted::new(logical, look.scale, |p| {
            p.f *= DESIGN_PX;
            panel::glass(p, fx, fy, WIDTH, height);
            let x0 = fx + PADDING;
            let right = x0 + inner;
            let mut y = fy + PADDING;
            let section = panel::section_style();
            let hint = Style::new(Face::Body, 13.0, panel::hint());
            let quiet = Style::new(Face::Body, 13.0, panel::tertiary());
            let number = |ink: u32| Style {
                tabular: true,
                ..Style::new(Face::Mono, 13.0, ink)
            };

            for block in &plan {
                let top = y;
                match (*block, agent) {
                    (Block::Nothing, _) | (_, None) => {
                        let style = Style::new(Face::Body, 14.0, panel::tertiary());
                        for (i, line) in [
                            "No AI coding subscriptions found.",
                            "Agents show up here once you've used them.",
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let w = text::width(line, &style);
                            p.text(
                                line,
                                x0 + (inner - w) / 2.0,
                                top + 14.0 + i as f32 * 22.0,
                                &style,
                            );
                        }
                    }
                    (Block::Hero, Some(agent)) => {
                        panel::app_placeholder(p, &agent.name, x0, top + 4.0, 40.0, 10.0, 20.0);
                        let name = Style::new(Face::BodyBold, 18.0, panel::bright());
                        p.text(
                            &text::ellipsize(&agent.name, &name, inner - 56.0),
                            x0 + 52.0,
                            top + 14.0,
                            &name,
                        );
                        let plan_ink = if agent.problem.is_some() {
                            panel::amber()
                        } else {
                            panel::tertiary()
                        };
                        let line = agent.problem.as_deref().unwrap_or(&agent.plan);
                        let plan_style = Style::new(Face::Body, 14.0, plan_ink);
                        p.text(
                            &text::ellipsize(line, &plan_style, inner - 56.0),
                            x0 + 52.0,
                            top + 35.0,
                            &plan_style,
                        );
                    }
                    (Block::Chips, _) => {
                        let style = Style::new(Face::BodyBold, 13.0, ink());
                        let mut x = x0;
                        for (at, other) in facts.agents.iter().enumerate() {
                            let label = text::ellipsize(&other.name, &style, 150.0);
                            let w = text::width(&label, &style) + 24.0;
                            if x + w > right {
                                break;
                            }
                            if at == look.chosen {
                                p.fill(x, top, w, CHIPS_H, 15.0, panel::amber());
                                let on = Style::new(Face::BodyBold, 13.0, panel::dark());
                                p.text(&label, x + 12.0, top + CHIPS_H / 2.0, &on);
                            } else {
                                panel::quiet_button(p, x, top, w, CHIPS_H, 15.0, false);
                                p.text(&label, x + 12.0, top + CHIPS_H / 2.0, &style);
                            }
                            chips.push((at, on_screen(x, top, w, CHIPS_H)));
                            x += w + 8.0;
                        }
                    }
                    (Block::Problem(lines), Some(agent)) => {
                        let h = block.height();
                        p.fill(x0, top, inner, h, panel::TILE_RADIUS, panel::tile());
                        let help = agent.help.as_deref().unwrap_or("");
                        for (i, line) in wrap(help, &help_style(), inner - 24.0, 3)
                            .iter()
                            .take(lines)
                            .enumerate()
                        {
                            p.text(line, x0 + 12.0, top + 20.0 + i as f32 * 18.0, &help_style());
                        }
                    }
                    (Block::Balance, Some(agent)) => {
                        if let Some(balance) = &agent.balance {
                            p.text("BALANCE", x0, top + 6.0, &section);
                            let low =
                                balance.funded > 0.0 && balance.remaining / balance.funded <= 0.1;
                            let amount = Style::new(
                                Face::BodyBold,
                                20.0,
                                if low { meter::ink(90) } else { panel::bright() },
                            );
                            let text = agents::money(balance.remaining, &balance.currency);
                            p.text(&text, x0, top + 28.0, &amount);
                            let note = if balance.estimated {
                                "estimated credit left"
                            } else {
                                "credit left"
                            };
                            p.text(
                                note,
                                x0 + text::width(&text, &amount) + 8.0,
                                top + 30.0,
                                &quiet,
                            );
                            if balance.funded > 0.0 {
                                meter_bar(
                                    p,
                                    x0,
                                    top + 44.0,
                                    inner,
                                    6.0,
                                    balance.remaining / balance.funded,
                                    meter::ink(if low { 90 } else { 0 }),
                                );
                                let detail = format!(
                                    "{} spent of {} funded",
                                    agents::money(balance.spent, &balance.currency),
                                    agents::money(balance.funded, &balance.currency)
                                );
                                p.text(&detail, x0, top + 58.0, &hint);
                            }
                        }
                    }
                    (Block::Limits(..), Some(agent)) => {
                        p.text("LIMITS", x0, top + 6.0, &section);
                        let mut row = top + TITLE_H;
                        for limit in &agent.limits {
                            let percent = percent_of(limit);
                            let tint = meter::ink(percent);
                            let title = Style::new(Face::Body, 14.0, panel::prose());
                            p.text(&limit.title, x0, row + 8.0, &title);
                            let value = format!("{percent}%");
                            let style = number(if percent >= 90 { tint } else { ink() });
                            p.text(
                                &value,
                                right - text::width(&value, &style),
                                row + 8.0,
                                &style,
                            );
                            let now = unix_from_minute(facts.minute);
                            let week = is_weekly(limit);
                            // A week's tokens are in the theme's accent, and under them, in grey,
                            // how much of the week has gone, so the two can be compared at a glance.
                            let bar_ink = if week {
                                palette::accent(0x42d3ffff)
                            } else {
                                tint
                            };
                            meter_bar(p, x0, row + 22.0, inner, 6.0, limit.used, bar_ink);
                            let mut text_y = row + 40.0;
                            if let (true, Some(at)) = (week, limit.resets) {
                                let gone = week_gone(at, now);
                                meter_bar(
                                    p,
                                    x0,
                                    row + 32.0,
                                    inner,
                                    4.0,
                                    gone,
                                    palette::tone(0xc3c9d3ff),
                                );
                                text_y += WEEK_BAR_H;
                            }
                            if let Some(at) = limit.resets {
                                let left = agents::time_left(at - now);
                                let words = if week {
                                    let gone = (week_gone(at, now) * 100.0).round();
                                    format!("{gone}% of the week gone · resets in {left}")
                                } else {
                                    format!("Resets in {left}")
                                };
                                p.text(&words, x0, text_y, &hint);
                            }
                            row += LIMIT_H + if week { WEEK_BAR_H } else { 0.0 };
                        }
                    }
                    (Block::Days(_, today_line), Some(agent)) => {
                        p.text("TOKENS BY DAY", x0, top + 6.0, &section);
                        let peak = agent.days.iter().map(|day| day.tokens).max().unwrap_or(0);
                        let mut row = top + TITLE_H;
                        for (at, day) in agent.days.iter().enumerate() {
                            let today = is_today(&agent.days, at, facts.today);
                            let centre = row + DAY_H / 2.0;
                            let name = if today {
                                "Today".to_string()
                            } else {
                                day_name(&day.date)
                            };
                            let label = Style::new(
                                if today { Face::BodyBold } else { Face::Body },
                                13.0,
                                if today {
                                    panel::bright()
                                } else {
                                    panel::tertiary()
                                },
                            );
                            p.text(&name, x0, centre, &label);
                            let value = agents::tokens(day.tokens);
                            let figure = number(if today {
                                panel::bright()
                            } else {
                                panel::secondary()
                            });
                            p.text(
                                &value,
                                right - text::width(&value, &figure),
                                centre,
                                &figure,
                            );
                            let track_x = x0 + 56.0;
                            let track_w = right - 64.0 - track_x;
                            let fraction = if peak > 0 {
                                day.tokens as f64 / peak as f64
                            } else {
                                0.0
                            };
                            let bar_ink = if today {
                                palette::accent(0x33ccffff)
                            } else {
                                palette::tone(0x6b7690ff)
                            };
                            meter_bar(p, track_x, centre - 3.0, track_w, 6.0, fraction, bar_ink);
                            row += DAY_H;
                        }
                        if today_line {
                            let line = format!(
                                "{} prompts · {} sessions today",
                                agent.today_prompts, agent.today_sessions
                            );
                            p.text(&line, x0, row + 10.0, &hint);
                        }
                    }
                    (Block::Models(_), Some(agent)) => {
                        p.text("TOKENS BY MODEL", x0, top + 6.0, &section);
                        let peak = agent
                            .models
                            .iter()
                            .map(|model| model.total)
                            .max()
                            .unwrap_or(0);
                        let mut row = top + TITLE_H;
                        for model in &agent.models {
                            let title = Style::new(Face::Body, 14.0, panel::prose());
                            let total = agents::tokens(model.total);
                            let figure = number(panel::secondary());
                            p.text(
                                &total,
                                right - text::width(&total, &figure),
                                row + 8.0,
                                &figure,
                            );
                            p.text(
                                &text::ellipsize(
                                    &model.name,
                                    &title,
                                    inner - text::width(&total, &figure) - 12.0,
                                ),
                                x0,
                                row + 8.0,
                                &title,
                            );
                            let fraction = if peak > 0 {
                                model.total as f64 / peak as f64
                            } else {
                                0.0
                            };
                            meter_bar(
                                p,
                                x0,
                                row + 22.0,
                                inner,
                                6.0,
                                fraction,
                                palette::accent(0x33ccff99),
                            );
                            let split = format!(
                                "in {} · out {} · cache {} read, {} written",
                                agents::tokens(model.input),
                                agents::tokens(model.output),
                                agents::tokens(model.cache_read),
                                agents::tokens(model.cache_write),
                            );
                            p.text(
                                &text::ellipsize(&split, &hint, inner),
                                x0,
                                row + 40.0,
                                &hint,
                            );
                            row += MODEL_H;
                        }
                    }
                }
                y += block.height() + GAP;
            }

            // The foot: whether it's being brought up to date, and the keys.
            p.fill(x0, y - GAP / 2.0, inner, 1.0, 0.0, panel::divider());
            let centre = y + FOOT_H / 2.0 - 2.0;
            if facts.updating {
                p.text("Updating…", x0, centre, &hint);
            }
            let mut start = right;
            if facts.agents.len() > 1 {
                start = panel::key_hint(p, start, centre, &["←", "→"], "switch") - panel::HINT_GAP;
            }
            start = panel::key_hint(p, start, centre, &["R"], "refresh") - panel::HINT_GAP;
            panel::key_hint(p, start, centre, &["Esc"], "");
        })?;

        self.origin = on_screen(0.0, 0.0, 0.0, 0.0).loc;
        self.frame = on_screen(fx, fy, WIDTH, height);
        self.chips = chips;
        self.painted = Some(painted);
        Some(())
    }
}

/// The start of `minute`, in seconds since the Unix epoch.
fn unix_from_minute(minute: i64) -> i64 {
    minute * 60
}

impl Slipstream {
    /// The bar's agents icon: `None` while no tool has numbers, else whether one is nearly used up.
    pub fn agents_icon(&self) -> Option<bool> {
        let agents = self
            .agent_usage
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        (!agents.is_empty()).then(|| agents.iter().any(Agent::alarming))
    }

    /// The bar's agents icon: opens the agents panel, or closes it.
    pub fn toggle_agents(&mut self) {
        if self.agents.is_open() {
            self.agents.close();
        } else {
            self.close_panels();
            let now = self.clock.tick();
            self.agents.open(now);
            refresh(Refresh::Limits);
        }
    }

    pub fn agents_facts(&self) -> Facts {
        Facts {
            agents: self
                .agent_usage
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone(),
            today: self.status.lock().unwrap().today,
            minute: unix_now() / 60,
            updating: updating(),
            reduced_motion: self.agents.reduced_motion,
            ring: self.panel_ring(),
        }
    }

    pub fn agents_key(&mut self, key: Keysym, shift: bool) {
        let facts = self.agents_facts();
        let request = self.agents.key(key, shift, &facts);
        self.agents_request(request);
    }

    /// A click while the panel is open, in logical pixels from the output's corner. Outside it
    /// closes it; on another of the bar's buttons, that button then does its own thing.
    pub fn agents_click(&mut self, x: f64, y: f64) {
        let facts = self.agents_facts();
        match self.agents.click(x, y, &facts) {
            Request::Close => {
                self.agents.close();
                let target = (!self.fullscreen_on_screen())
                    .then(|| self.focused_bar_target(x, y))
                    .flatten()
                    .filter(|target| *target != bar::Target::Agents);
                if let Some(target) = target {
                    self.bar_clicked(target);
                }
            }
            request => self.agents_request(request),
        }
    }

    fn agents_request(&mut self, request: Request) {
        match request {
            Request::Nothing => {}
            Request::Close => self.agents.close(),
            Request::Refresh => refresh(Refresh::Full),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(days: usize, models: usize, limits: usize) -> Agent {
        Agent {
            id: "claude".into(),
            name: "Claude Code".into(),
            plan: "Pro".into(),
            limits: (0..limits)
                .map(|i| Limit {
                    title: format!("Limit {i}"),
                    used: 0.3,
                    resets: None,
                })
                .collect(),
            days: (0..days)
                .map(|i| Day {
                    date: format!("2026-10-{:02}", 8 + i),
                    tokens: 1000 * i as u64,
                })
                .collect(),
            models: (0..models)
                .map(|i| agents::Model {
                    name: format!("Model {i}"),
                    total: 10,
                    input: 1,
                    output: 2,
                    cache_read: 3,
                    cache_write: 4,
                })
                .collect(),
            counts_prompts: true,
            ..Agent::default()
        }
    }

    #[test]
    fn the_card_has_a_block_for_what_there_is_to_show() {
        let full = blocks(&agent(7, 4, 3), false, 400.0);
        assert_eq!(
            full,
            [
                Block::Hero,
                Block::Limits(3, 0),
                Block::Days(7, true),
                Block::Models(4)
            ]
        );
        let bare = blocks(&agent(0, 0, 0), true, 400.0);
        assert_eq!(bare, [Block::Hero, Block::Chips]);
        let mut broken = agent(0, 0, 0);
        broken.problem = Some("Signed out".into());
        broken.help = Some("Run `claude auth login` to restore authoritative usage.".into());
        assert!(matches!(
            blocks(&broken, false, 400.0)[1],
            Block::Problem(1..=3)
        ));
    }

    #[test]
    fn a_weekly_limit_gets_a_bar_for_the_week_gone_by() {
        let mut agent = agent(0, 0, 2);
        agent.limits[0].title = "Session".into();
        agent.limits[0].resets = Some(1_000);
        agent.limits[1].title = "Weekly".into();
        assert_eq!(
            blocks(&agent, false, 400.0)[1],
            Block::Limits(2, 0),
            "no reset, no week"
        );
        agent.limits[1].resets = Some(1_000);
        let with_week = blocks(&agent, false, 400.0)[1];
        assert_eq!(with_week, Block::Limits(2, 1));
        assert_eq!(
            with_week.height(),
            Block::Limits(2, 0).height() + WEEK_BAR_H
        );
    }

    #[test]
    fn the_week_runs_from_the_last_reset_not_from_monday() {
        let resets = 10 * WEEK_SECS;
        assert_eq!(week_gone(resets, resets - WEEK_SECS), 0.0);
        assert_eq!(week_gone(resets, resets - WEEK_SECS / 2), 0.5);
        assert_eq!(week_gone(resets, resets), 1.0);
        // A reset more than a week off, or already past, stays on the bar.
        assert_eq!(week_gone(resets, resets - 2 * WEEK_SECS), 0.0);
        assert_eq!(week_gone(resets, resets + 60), 1.0);
    }

    #[test]
    fn days_are_named_and_today_is_picked_out() {
        assert_eq!(day_name("2026-10-14"), "Wed");
        assert_eq!(day_name("2026-10-11"), "Sun");
        assert_eq!(day_name("whenever"), "whenever");
        let days = agent(7, 0, 0).days;
        let today = Some(Date {
            year: 2026,
            month: 10,
            day: 14,
        });
        assert!(is_today(&days, 6, today));
        assert!(!is_today(&days, 5, today));
        assert!(
            is_today(&days, 6, None),
            "the last, when the date isn't known"
        );
        let stale = Some(Date {
            year: 2026,
            month: 10,
            day: 20,
        });
        assert!(
            !is_today(&days, 6, stale),
            "a record from days ago has no today"
        );
    }

    #[test]
    fn long_help_wraps_and_is_cut_short() {
        let style = help_style();
        let text = "Set FIREWORKS_API_KEY, run firectl set-api-key, or sign in to Fireworks in opencode, \
                    then try again from a terminal where the key is exported for good measure.";
        let lines = wrap(text, &style, 200.0, 3);
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|line| text::width(line, &style) <= 200.0));
        assert!(lines[2].ends_with('…'));
        assert_eq!(wrap("short", &style, 200.0, 3), ["short"]);
        assert!(wrap("", &style, 200.0, 3).is_empty());
    }

    fn facts(agents: Vec<Agent>) -> Facts {
        Facts {
            agents,
            today: None,
            minute: 0,
            updating: false,
            reduced_motion: false,
            ring: 0,
        }
    }

    #[test]
    fn keys_switch_between_tools_and_wrap_round() {
        let mut other = agent(0, 0, 0);
        other.id = "codex".into();
        let facts = facts(vec![agent(0, 0, 0), other]);
        let mut panel = Agents::default();
        assert_eq!(panel.index(&facts), 0);
        assert_eq!(panel.key(Keysym::l, false, &facts), Request::Nothing);
        assert_eq!(panel.index(&facts), 1);
        panel.key(Keysym::Right, false, &facts);
        assert_eq!(panel.index(&facts), 0, "round to the first");
        panel.key(Keysym::Left, false, &facts);
        assert_eq!(panel.index(&facts), 1, "and back");
        panel.key(Keysym::Tab, true, &facts);
        assert_eq!(panel.index(&facts), 0);
        assert_eq!(panel.key(Keysym::r, false, &facts), Request::Refresh);
        assert_eq!(panel.key(Keysym::Escape, false, &facts), Request::Close);
    }

    #[test]
    fn the_card_paints_for_one_tool_and_for_none() {
        let mut panel = Agents::default();
        let mut shown = facts(vec![agent(7, 4, 3)]);
        for agents in [shown.agents.clone(), Vec::new()] {
            shown.agents = agents;
            let look = Look {
                facts: shown.clone(),
                chosen: 0,
                width: 1536,
                scale: 1.25,
            };
            assert!(panel.repaint(&look).is_some());
            assert!(panel.frame.size.w > 0.0 && panel.frame.size.h > 0.0);
        }
        assert_eq!(panel.click(1.0, 1.0, &shown), Request::Close, "outside");
        let (x, y) = (
            panel.frame.loc.x + panel.frame.size.w / 2.0,
            panel.frame.loc.y + panel.frame.size.h / 2.0,
        );
        assert_eq!(panel.click(x, y, &shown), Request::Nothing);
    }

    #[test]
    fn the_updater_is_asked_for_the_most_that_was_wanted() {
        // Asking leaves a note for the reader to act on, and a full refresh outranks the rest.
        *ASKED.lock().unwrap() = None;
        refresh(Refresh::Limits);
        assert_eq!(*ASKED.lock().unwrap(), Some(Refresh::Limits));
        refresh(Refresh::Full);
        refresh(Refresh::Limits);
        assert_eq!(*ASKED.lock().unwrap(), Some(Refresh::Full));
        *ASKED.lock().unwrap() = None;
    }
}
