#!/usr/bin/env python3
"""Agent usage: a Slipstream plugin showing how much of each AI coding tool's allowance is used.

Omarchy's `omarchy-agent-usage-update` keeps one JSON record per tool (Claude Code, Codex, ...)
in ~/.local/state/omarchy/agents/usage/. This keeps those records current and prints them as a
card, one section per tool with a gauge per allowance, each with when it starts over.

- `agent-usage.py` keeps running: it reads the records every minute, runs the updater every
  quarter of an hour, and prints a card whenever what it shows changes.
- `agent-usage.py press refresh` runs beside it when Refresh is pressed, and only leaves the
  press in the plugin's own folder for the running one, which updates everything at once.
"""

import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

EVERY = 15 * 60  # how often the records are made again, in seconds
RESCAN = 60  # how often they are read again, to pick up whatever else wrote them
UPDATE_LIMIT = 180  # an update still running after this long is stopped
LOOK = 0.25  # how often the running program looks for presses and at its update
MOST_SECTIONS = 4  # a card shows this many sections at most

# How long each kind of allowance lasts, by the words its label uses, in seconds.
WINDOWS = [
    (("month", "30-day"), "Monthly", 30 * 86400),
    (("week", "7-day", "seven"), "Weekly", 7 * 86400),
]
SESSION = ("Session", 5 * 3600)


def usage_dir():
    state = os.environ.get("XDG_STATE_HOME") or os.path.expanduser("~/.local/state")
    return Path(state) / "omarchy" / "agents" / "usage"


def asked_dir():
    state = os.environ.get("SLIPSTREAM_PLUGIN_STATE")
    if not state:
        sys.exit("agent-usage: SLIPSTREAM_PLUGIN_STATE isn't set: Slipstream sets it for a plugin")
    return Path(state) / "asked"


def window_of(label):
    """`Session (5-hour)`, `Weekly (7-day)` and the like: its plain name, and how long it lasts."""
    text = label.lower()
    for words, name, seconds in WINDOWS:
        if any(word in text for word in words):
            return name, seconds
    hours = re.search(r"(\d+)\s*-?\s*h(?:our)?", text)
    if "session" in text or hours:
        return SESSION[0], int(hours.group(1)) * 3600 if hours else SESSION[1]
    plain = label.split("(")[0].strip()
    return plain or "Limit", None


def epoch(text):
    try:
        return datetime.fromisoformat(text.replace("Z", "+00:00")).timestamp()
    except (TypeError, ValueError):
        return None


def section(record):
    """One tool's section of the card, or None when it has nothing to show."""
    meters = []
    for limit in record.get("limits") or []:
        used = limit.get("percent")
        if not isinstance(used, (int, float)) or used < 0:
            continue
        name, window = window_of(limit.get("label", ""))
        meter = {"label": limit.get("title") or name, "percent": round(used * 100)}
        resets = epoch(limit.get("resetsAt"))
        if resets:
            meter["resets"] = resets
            if window:
                meter["window"] = window
        meters.append(meter)
    notes = []
    balance = record.get("balance") or {}
    remaining = balance.get("remaining")
    if isinstance(remaining, (int, float)):
        currency = balance.get("currency") or "$"
        notes.append(f"{currency}{remaining:,.2f} left")
    prompts, sessions = record.get("todayPrompts") or 0, record.get("todaySessions") or 0
    if prompts or sessions:
        notes.append(f"Today {int(prompts)} prompts, {int(sessions)} sessions")
    if not meters and not notes:
        return None
    shown = {"name": record.get("name") or record.get("id") or "Agent", "meters": meters}
    if record.get("tierLabel"):
        shown["detail"] = record["tierLabel"]
    if notes:
        shown["note"] = " · ".join(notes)
    if record.get("limitsStale"):
        shown["stale"] = True
    return shown


def read_sections(folder):
    """Every tool with something to show, nearest a limit first, as only a few fit."""
    records = []
    for path in sorted(folder.glob("*.json")) if folder.is_dir() else []:
        try:
            records.append(json.loads(path.read_text()))
        except (OSError, ValueError):
            continue
    sections = [s for s in map(section, records) if s]
    sections.sort(key=lambda s: -max((m["percent"] for m in s["meters"]), default=-1))
    return sections


def card(sections, updating):
    shown = {
        "sections": sections[:MOST_SECTIONS],
        "buttons": [{"id": "refresh", "label": "Updating…" if updating else "Refresh"}],
    }
    if not sections:
        shown["lines"] = ["No usage recorded yet"]
    return shown


def press(button):
    if button != "refresh":
        sys.exit(f"agent-usage: nothing is pressed as {button!r}")
    asked = asked_dir()
    asked.mkdir(parents=True, exist_ok=True)
    # Written under a hidden name first, so a half-written one is never read.
    name = f"{time.time_ns()}-{os.getpid()}"
    (asked / f".{name}").write_text(button)
    (asked / f".{name}").rename(asked / name)


def take_asked(asked):
    found = False
    for entry in asked.glob("[!.]*") if asked.is_dir() else []:
        entry.unlink(missing_ok=True)
        found = True
    return found


def start_update(full):
    env = dict(os.environ)
    # The updater finds its collectors from here; a session started outside Omarchy's lacks it.
    env.setdefault("OMARCHY_PATH", "/usr/share/omarchy")
    command = ["omarchy-agent-usage-update"] + (["--force"] if full else [])
    try:
        return subprocess.Popen(
            command, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE, text=True,
        )
    except OSError as err:
        print(f"agent-usage: couldn't start {command[0]}: {err}", file=sys.stderr)
        return None


def keep_running():
    asked, folder = asked_dir(), usage_dir()
    printed = None
    next_update = next_rescan = 0.0
    update, started = None, 0.0
    sections = []
    wanted = False
    while True:
        now = time.monotonic()
        # A press while an update runs waits for it, and then asks for another.
        wanted = take_asked(asked) or wanted
        if update is not None:
            status = update.poll()
            if status is None and now - started > UPDATE_LIMIT:
                update.kill()
                status = update.wait()
            if status is not None:
                if status != 0:
                    said = (update.stderr.read() or "").strip().splitlines()
                    print(f"agent-usage: the update failed: {said[-1] if said else status}",
                          file=sys.stderr)
                update, next_update, next_rescan = None, now + EVERY, now
        if update is None and (wanted or now >= next_update):
            update, started = start_update(full=wanted), now
            wanted = False
            if update is None:
                next_update = now + EVERY
        if now >= next_rescan:
            sections = read_sections(folder)
            next_rescan = now + RESCAN
        line = json.dumps(card(sections, update is not None), ensure_ascii=False,
                          separators=(",", ":"))
        if line != printed:
            print(line, flush=True)
            printed = line
        time.sleep(LOOK)


if __name__ == "__main__":
    args = sys.argv[1:]
    if not args:
        keep_running()
    elif len(args) == 2 and args[0] == "press":
        press(args[1])
    else:
        sys.exit("usage: agent-usage.py [press refresh]")
