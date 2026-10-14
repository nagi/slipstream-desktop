//! The agents panel's data: the usage records Omarchy's `omarchy-agent-usage-update` keeps in
//! `~/.local/state/omarchy/agents/usage/`, one JSON file for each AI coding tool (Claude Code,
//! Codex, Fireworks), and what the panel makes of them. The compositor reads them on a thread of
//! its own; nothing here touches the screen.
//!
//! A record that turns up in the directory is an agent, whoever wrote it. It is shown once it has
//! numbers to show: any prompts, sessions, active days, limits or a balance.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

/// How many models the panel lists, the busiest first.
pub const MODELS_SHOWN: usize = 4;

/// Where the records are, if the home is known.
pub fn usage_dir() -> Option<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".local/state")))?;
    Some(state.join("omarchy/agents/usage"))
}

/// One tool, ready to show.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Agent {
    pub id: String,
    pub name: String,
    /// The plan it runs on, e.g. `Pro` or `Max 20x`.
    pub plan: String,
    /// What is wrong with its numbers, if anything, and how to put it right.
    pub problem: Option<String>,
    pub help: Option<String>,
    pub limits: Vec<Limit>,
    pub balance: Option<Balance>,
    /// Tokens for each of the last days, oldest first; the last is today.
    pub days: Vec<Day>,
    pub today_prompts: u64,
    pub today_sessions: u64,
    /// Whether prompts are counted at all. A billing API's are not, and "0 prompts" would read
    /// as a quiet day.
    pub counts_prompts: bool,
    pub models: Vec<Model>,
}

/// One allowance: a rolling session, a week, a model's share.
#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    pub title: String,
    /// 0 to 1, or more when over.
    pub used: f64,
    /// Seconds since the Unix epoch.
    pub resets: Option<i64>,
}

/// A prepaid account's credit.
#[derive(Debug, Clone, PartialEq)]
pub struct Balance {
    pub remaining: f64,
    pub funded: f64,
    pub spent: f64,
    pub currency: String,
    pub estimated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Day {
    /// `2026-10-14`.
    pub date: String,
    pub tokens: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub name: String,
    pub total: u64,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

impl Agent {
    /// The fullest allowance, which is what stops the next prompt.
    pub fn binding(&self) -> Option<&Limit> {
        self.limits.iter().max_by(|a, b| a.used.total_cmp(&b.used))
    }

    /// Whether the bar's icon should warn: an allowance nearly used, or credit nearly gone.
    pub fn alarming(&self) -> bool {
        self.binding().is_some_and(|limit| limit.used >= 0.9)
            || self
                .balance
                .as_ref()
                .is_some_and(|b| b.funded > 0.0 && b.remaining / b.funded <= 0.1)
    }
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Record {
    id: String,
    name: String,
    tier_label: String,
    usage_status_text: String,
    auth_help_text: String,
    limits: Vec<RecordLimit>,
    balance: Option<RecordBalance>,
    recent_days: Vec<RecordDay>,
    today_prompts: f64,
    today_sessions: f64,
    total_prompts: f64,
    total_sessions: f64,
    active_days: f64,
    has_prompt_stats: Option<bool>,
    model_usage: HashMap<String, Tokens>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RecordLimit {
    label: String,
    title: String,
    percent: Option<f64>,
    resets_at: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RecordBalance {
    remaining: Option<f64>,
    funded: f64,
    spent: f64,
    currency: String,
    estimated: bool,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RecordDay {
    date: String,
    message_count: f64,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Tokens {
    input_tokens: f64,
    output_tokens: f64,
    cache_read_input_tokens: f64,
    cache_creation_input_tokens: f64,
}

/// A count out of a JSON number that might be anything.
fn count(value: f64) -> u64 {
    if value.is_finite() && value > 0.0 {
        value as u64
    } else {
        0
    }
}

impl Agent {
    /// The agent in one record's text, or why it isn't one.
    pub fn parse(text: &str) -> Result<Agent, String> {
        let record: Record = serde_json::from_str(text.trim()).map_err(|err| err.to_string())?;
        if record.id.is_empty() {
            return Err("no id".into());
        }
        let has_data = record.total_prompts > 0.0
            || record.total_sessions > 0.0
            || record.active_days > 0.0
            || record.today_prompts > 0.0
            || record.today_sessions > 0.0
            || !record.limits.is_empty()
            || record.balance.is_some();
        let limits: Vec<Limit> = record
            .limits
            .iter()
            .filter_map(|limit| {
                let used = limit.percent.filter(|p| p.is_finite() && *p >= 0.0)?;
                Some(Limit {
                    title: if limit.title.is_empty() {
                        window_title(&limit.label)
                    } else {
                        limit.title.clone()
                    },
                    used,
                    resets: parse_time(&limit.resets_at),
                })
            })
            .collect();
        let balance = record.balance.as_ref().and_then(|b| {
            let remaining = b.remaining.filter(|r| r.is_finite() && *r >= 0.0)?;
            Some(Balance {
                remaining,
                funded: if b.funded.is_finite() && b.funded > 0.0 {
                    b.funded
                } else {
                    0.0
                },
                spent: if b.spent.is_finite() {
                    b.spent.max(0.0)
                } else {
                    0.0
                },
                currency: if b.currency.is_empty() {
                    "USD".into()
                } else {
                    b.currency.clone()
                },
                estimated: b.estimated,
            })
        });
        let mut models: Vec<Model> = record
            .model_usage
            .iter()
            .map(|(id, tokens)| {
                let (input, output) = (count(tokens.input_tokens), count(tokens.output_tokens));
                let (cache_read, cache_write) = (
                    count(tokens.cache_read_input_tokens),
                    count(tokens.cache_creation_input_tokens),
                );
                Model {
                    name: model_name(id),
                    total: input + output + cache_read + cache_write,
                    input,
                    output,
                    cache_read,
                    cache_write,
                }
            })
            .collect();
        models.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.name.cmp(&b.name)));
        models.truncate(MODELS_SHOWN);
        let problem = Some(record.usage_status_text.clone()).filter(|text| !text.is_empty());
        let plan = match record.tier_label.as_str() {
            "" => "Subscription".to_string(),
            tier => {
                let mut chars = tier.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().chain(chars).collect())
                    .unwrap_or_default()
            }
        };
        let agent = Agent {
            name: if record.name.is_empty() {
                record.id.clone()
            } else {
                record.name.clone()
            },
            id: record.id,
            plan,
            help: problem
                .as_ref()
                .and(Some(record.auth_help_text).filter(|text| !text.is_empty())),
            problem,
            limits,
            balance,
            days: record
                .recent_days
                .iter()
                .map(|day| Day {
                    date: day.date.clone(),
                    tokens: count(day.message_count),
                })
                .collect(),
            today_prompts: count(record.today_prompts),
            today_sessions: count(record.today_sessions),
            counts_prompts: record.has_prompt_stats != Some(false),
            models,
        };
        if has_data {
            Ok(agent)
        } else {
            Err("nothing recorded yet".into())
        }
    }
}

/// Every agent with something to show in the records directory, by name.
pub fn load(dir: &Path) -> Vec<Agent> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut agents: Vec<Agent> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| Agent::parse(&fs::read_to_string(path).ok()?).ok())
        .collect();
    agents.sort_by_key(|agent| agent.name.to_lowercase());
    agents
}

/// `Session (5-hour)`, `5h window` and the like, as the one word for what it is.
pub fn window_title(label: &str) -> String {
    let text = label.to_lowercase();
    let long = ["week", "7-day", "seven", "month", "30-day"]
        .iter()
        .any(|word| text.contains(word));
    if text.contains("month") || text.contains("30-day") {
        "Monthly".into()
    } else if long {
        "Weekly".into()
    } else if text.contains("session") || text.contains("hour") || short_span(&text) {
        "Session".into()
    } else {
        let plain = label.split('(').next().unwrap_or("").trim();
        if plain.is_empty() {
            "Limit".into()
        } else {
            plain.into()
        }
    }
}

/// Whether `text` has a span like `5h` or `30m` in it.
fn short_span(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(2).enumerate().any(|(at, pair)| {
        pair[0].is_ascii_digit()
            && matches!(pair[1], b'h' | b'm')
            && !bytes.get(at + 2).is_some_and(u8::is_ascii_alphabetic)
    })
}

/// `claude-opus-4-8` as `Opus 4.8`, `gpt-5.6-sol` as `GPT 5.6 Sol`.
pub fn model_name(id: &str) -> String {
    if id.is_empty() {
        return "Unknown".into();
    }
    // Some providers name a model by its path: `accounts/fireworks/models/deepseek-v3p2`.
    let id = id.rsplit('/').next().unwrap_or(id);
    let name = id.strip_prefix("claude-").unwrap_or(id);
    let name = match name.rsplit_once('-') {
        Some((rest, date)) if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) => rest,
        _ => name,
    };
    let mut words: Vec<String> = Vec::new();
    let mut version: Vec<String> = Vec::new();
    let flush = |version: &mut Vec<String>, words: &mut Vec<String>| {
        if !version.is_empty() {
            words.push(version.join("."));
            version.clear();
        }
    };
    for part in name.split('-').filter(|part| !part.is_empty()) {
        // `v3p2` is version 3.2, and the `v` is no word of its own.
        let dotted = part.replace('p', ".");
        let part = match part.strip_prefix('v') {
            Some(rest)
                if rest.starts_with(|c: char| c.is_ascii_digit())
                    && rest
                        .split('p')
                        .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) =>
            {
                dotted[1..].to_string()
            }
            _ => part.to_string(),
        };
        let part = part.as_str();
        if part.starts_with(|c: char| c.is_ascii_digit()) {
            version.push(part.to_string());
            continue;
        }
        flush(&mut version, &mut words);
        words.push(match part {
            "gpt" => "GPT".into(),
            "deepseek" => "DeepSeek".into(),
            _ => {
                let mut chars = part.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().chain(chars).collect())
                    .unwrap_or_default()
            }
        });
    }
    flush(&mut version, &mut words);
    if words.is_empty() {
        "Unknown".into()
    } else {
        words.join(" ")
    }
}

/// `578.8M`, `1.5K`, `42`.
pub fn tokens(n: u64) -> String {
    let n = n as f64;
    if n >= 1e9 {
        format!("{:.1}B", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.1}M", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.1}K", n / 1e3)
    } else {
        format!("{n}")
    }
}

/// `$4.20`, `€3.00`, `CHF 3.00`.
pub fn money(amount: f64, currency: &str) -> String {
    let prefix = match currency.to_uppercase().as_str() {
        "USD" => "$".to_string(),
        "EUR" => "€".to_string(),
        "GBP" => "£".to_string(),
        other => format!("{other} "),
    };
    format!(
        "{prefix}{:.2}",
        if amount.is_finite() { amount } else { 0.0 }
    )
}

/// `2d 3h`, `1h 20m`, `5m`: the time left, never less than a minute.
pub fn time_left(secs: i64) -> String {
    if secs <= 0 {
        return "now".into();
    }
    let minutes = secs / 60;
    let (hours, days) = (minutes / 60, minutes / 1440);
    if days > 0 {
        format!("{days}d {}h", hours % 24)
    } else if hours > 0 {
        format!("{hours}h {}m", minutes % 60)
    } else {
        format!("{}m", minutes.max(1))
    }
}

/// An RFC 3339 time such as `2026-10-14T16:00:00.265166+00:00` as seconds since the Unix epoch.
pub fn parse_time(text: &str) -> Option<i64> {
    let (date, time) = text.split_once('T')?;
    let mut ymd = date.split('-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (ymd.next()??, ymd.next()??, ymd.next()??);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let split = time.find(['Z', '+', '-']).unwrap_or(time.len());
    let (clock, zone) = time.split_at(split);
    let mut hms = clock.split(':');
    let hour: i64 = hms.next()?.parse().ok()?;
    let minute: i64 = hms.next()?.parse().ok()?;
    let second: f64 = hms.next().unwrap_or("0").parse().ok()?;
    let offset = match zone.as_bytes().first() {
        None | Some(b'Z') => 0,
        Some(sign) => {
            let (h, m) = zone[1..].split_once(':')?;
            let secs = h.parse::<i64>().ok()? * 3600 + m.parse::<i64>().ok()? * 60;
            if *sign == b'-' { -secs } else { secs }
        }
    };
    // Days since 1970-01-01, by Howard Hinnant's civil-date arithmetic.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hour * 3600 + minute * 60 + second as i64 - offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLAUDE: &str = r#"{"id":"claude","name":"Claude Code","tierLabel":"pro",
        "limits":[{"label":"Session (5-hour)","percent":0.07,"resetsAt":"2026-10-14T16:00:00.265166+00:00"},
                  {"label":"Weekly (7-day)","percent":0.93,"resetsAt":"2026-10-18T10:00:00+00:00"},
                  {"label":"Opus 5 (1M context)","title":"Opus Weekly","percent":0.1}],
        "recentDays":[{"date":"2026-10-13","messageCount":213169856},{"date":"2026-10-14","messageCount":578849565}],
        "modelUsage":{"claude-opus-5-5":{"inputTokens":10,"outputTokens":20,"cacheReadInputTokens":300,"cacheCreationInputTokens":4},
                      "claude-haiku-4-5-20251001":{"inputTokens":1,"outputTokens":1}},
        "todayPrompts":3055,"todaySessions":5,"totalPrompts":8051}"#;

    #[test]
    fn a_record_becomes_an_agent() {
        let agent = Agent::parse(CLAUDE).unwrap();
        assert_eq!(
            (agent.name.as_str(), agent.plan.as_str()),
            ("Claude Code", "Pro")
        );
        let titles: Vec<_> = agent.limits.iter().map(|l| l.title.as_str()).collect();
        assert_eq!(titles, ["Session", "Weekly", "Opus Weekly"]);
        assert_eq!(agent.binding().unwrap().title, "Weekly");
        assert!(agent.alarming());
        assert_eq!(agent.days.last().unwrap().tokens, 578_849_565);
        assert_eq!(agent.models[0].name, "Opus 5.5");
        assert_eq!(agent.models[0].total, 334);
        assert_eq!(agent.models[1].name, "Haiku 4.5");
        assert!(agent.counts_prompts);
        assert_eq!((agent.today_prompts, agent.today_sessions), (3055, 5));
    }

    #[test]
    fn a_record_with_nothing_in_it_is_no_agent() {
        let empty = r#"{"id":"codex","name":"Codex","recentDays":[{"date":"2026-10-14","messageCount":0}],
            "totalPrompts":0,"activeDays":0,"limits":[],"authHelpText":"Run `codex login`."}"#;
        assert!(Agent::parse(empty).is_err());
        assert!(Agent::parse("{").is_err());
        assert!(Agent::parse(r#"{"name":"x","totalPrompts":3}"#).is_err());
    }

    #[test]
    fn a_balance_runs_low_like_a_limit_fills() {
        let record = r#"{"id":"fireworks","name":"Fireworks","hasPromptStats":false,
            "balance":{"remaining":1.5,"funded":20,"spent":18.5,"estimated":true}}"#;
        let agent = Agent::parse(record).unwrap();
        let balance = agent.balance.as_ref().unwrap();
        assert_eq!(money(balance.remaining, &balance.currency), "$1.50");
        assert!(balance.estimated && !agent.counts_prompts);
        assert!(agent.alarming());
    }

    #[test]
    fn a_problem_comes_with_its_remedy() {
        let record = r#"{"id":"claude","totalPrompts":1,"usageStatusText":"Signed out",
            "authHelpText":"Run `claude auth login`."}"#;
        let agent = Agent::parse(record).unwrap();
        assert_eq!(agent.problem.as_deref(), Some("Signed out"));
        assert_eq!(agent.help.as_deref(), Some("Run `claude auth login`."));
    }

    #[test]
    fn windows_and_models_are_named_plainly() {
        assert_eq!(window_title("Session (5-hour)"), "Session");
        assert_eq!(window_title("5h window"), "Session");
        assert_eq!(window_title("Weekly (7-day)"), "Weekly");
        assert_eq!(window_title("30-day"), "Monthly");
        assert_eq!(window_title("Credits (x)"), "Credits");
        assert_eq!(window_title(""), "Limit");
        assert_eq!(model_name("claude-opus-4-8"), "Opus 4.8");
        assert_eq!(model_name("gpt-5.6-sol"), "GPT 5.6 Sol");
        assert_eq!(model_name("claude-haiku-4-5-20251001"), "Haiku 4.5");
        assert_eq!(
            model_name("accounts/fireworks/models/deepseek-v3p2"),
            "DeepSeek 3.2"
        );
        assert_eq!(model_name(""), "Unknown");
    }

    #[test]
    fn numbers_read_short() {
        assert_eq!(tokens(42), "42");
        assert_eq!(tokens(1_500), "1.5K");
        assert_eq!(tokens(578_849_565), "578.8M");
        assert_eq!(tokens(2_300_000_000), "2.3B");
        assert_eq!(time_left(0), "now");
        assert_eq!(time_left(20), "1m");
        assert_eq!(time_left(3 * 3600 + 5 * 60), "3h 5m");
        assert_eq!(time_left(2 * 86_400 + 3 * 3600), "2d 3h");
        assert_eq!(money(3.0, "EUR"), "€3.00");
        assert_eq!(money(3.0, "chf"), "CHF 3.00");
    }

    #[test]
    fn times_are_read_in_any_zone() {
        assert_eq!(parse_time("1970-01-02T00:00:00Z"), Some(86_400));
        assert_eq!(
            parse_time("2026-10-14T16:00:00.265166+00:00"),
            Some(1_791_993_600)
        );
        assert_eq!(
            parse_time("2026-10-14T18:00:00+02:00"),
            parse_time("2026-10-14T16:00:00Z")
        );
        assert_eq!(
            parse_time("2026-10-14T11:00:00-05:00"),
            parse_time("2026-10-14T16:00:00Z")
        );
        assert_eq!(parse_time("soon"), None);
        assert_eq!(parse_time(""), None);
    }
}
