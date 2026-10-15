//! Power: the battery and the charger.

use super::*;
use crate::power as supplies;

/// How often the Power page reads the battery again.
const POWER_EVERY: Duration = Duration::from_secs(2);

/// The battery's charge and what it's doing, then the details: power flowing, the charger, and
/// how the battery has worn. Read again every couple of seconds while the window is open.
pub(super) fn build(_: &Store) -> gtk::Widget {
    let page = page("Power", "Battery and charging");

    let card = gtk::Box::builder()
        .spacing(18)
        .css_classes(["card", "batt"])
        .build();
    let percent = label("", "batt-percent");
    percent.set_wrap(false);
    let bar = gtk::ProgressBar::builder()
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    let summary = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Center)
        .build();
    let state = label("", "batt-state");
    let time = label("", "row-sub");
    summary.append(&state);
    summary.append(&time);
    card.append(&percent);
    card.append(&bar);
    card.append(&summary);
    page.append(&card);

    let details = group(&page, "Details");
    let value = || {
        let value = label("", "kv");
        value.set_xalign(1.0);
        value
    };
    let add = |title: &str, sub: Option<&str>| {
        let value = value();
        let (row, sub) = row_box(title, sub, &value);
        details.append(&row);
        (row, value, sub)
    };
    let (_, flow, flow_sub) = add("Power", Some(""));
    let (_, charger, _) = add("Charger", None);
    let (limit_row, limit, _) = add(
        "Charge limit",
        Some("Set in the firmware, to spare the battery"),
    );
    let (health_row, health, health_sub) = add("Health", Some(""));
    let (cycles_row, cycles, _) = add("Charge cycles", None);

    let mut estimate = slipstream_config::battery::Estimate::default();
    let opened = std::time::Instant::now();
    let mut update = move || {
        let reading = supplies::read(&mut estimate, opened.elapsed().as_secs_f64());
        let Some(battery) = &reading.battery else {
            percent.set_label("—");
            bar.set_fraction(0.0);
            bar.remove_css_class("charging");
            state.set_label(if reading.plugged {
                "No battery"
            } else {
                "No battery found"
            });
            time.set_label("Running on mains power");
            flow.set_label("—");
            if let Some(sub) = &flow_sub {
                sub.set_label("");
            }
            charger.set_label(&charger_text(&reading));
            for row in [&limit_row, &health_row, &cycles_row] {
                row.set_visible(false);
            }
            return;
        };
        percent.set_label(&format!("{}%", battery.percent));
        bar.set_fraction(battery.percent as f64 / 100.0);
        if battery.charging() {
            bar.add_css_class("charging");
        } else {
            bar.remove_css_class("charging");
        }
        state.set_label(match battery.status.as_str() {
            "Charging" => "Charging",
            "Full" => "Fully charged",
            "Not charging" if battery.limit.is_some_and(|limit| battery.percent >= limit) => {
                "Held at its limit"
            }
            "Not charging" => "Plugged in, not charging",
            _ if reading.plugged => "Plugged in",
            _ => "On battery",
        });
        let duration = slipstream_config::battery::duration;
        time.set_label(&match (battery.hours, battery.status.as_str()) {
            (Some(hours), "Charging") => format!("{} until full", duration(hours)),
            (Some(hours), _) => format!("{} left", duration(hours)),
            // Just after a charger is plugged in or pulled out, while the rate settles.
            (None, "Charging" | "Discharging") => "Working out the time".to_string(),
            (None, _) => String::new(),
        });
        time.set_visible(!time.label().is_empty());

        flow.set_label(
            &battery
                .watts
                .map_or("—".to_string(), |watts| format!("{watts:.1} W")),
        );
        if let Some(sub) = &flow_sub {
            sub.set_label(match (battery.watts, battery.status.as_str()) {
                (None, _) => "Not reported by this battery",
                (_, "Charging") => "Going into the battery",
                (_, "Discharging") => "Being drawn from the battery",
                _ => "Through the battery",
            });
        }
        charger.set_label(&charger_text(&reading));
        limit_row.set_visible(battery.limit.is_some());
        limit.set_label(
            &battery
                .limit
                .map_or(String::new(), |limit| format!("{limit}%")),
        );
        health_row.set_visible(battery.health.is_some());
        health.set_label(
            &battery
                .health
                .map_or(String::new(), |health| format!("{health}%")),
        );
        if let Some(sub) = &health_sub {
            sub.set_label(&match battery.capacity_wh {
                Some((now, new)) => {
                    format!("A full charge holds {now:.1} Wh; it held {new:.1} Wh new")
                }
                None => "What a full charge holds now, against when it was new".to_string(),
            });
        }
        cycles_row.set_visible(battery.cycles.is_some());
        cycles.set_label(
            &battery
                .cycles
                .map_or(String::new(), |cycles| cycles.to_string()),
        );
    };
    update();
    // Every page is built when the window opens, so the battery is only read while this one
    // shows, straight away when it comes into view. Reading stops once the page is gone.
    let update = Rc::new(RefCell::new(update));
    page.connect_map({
        let update = update.clone();
        move |_| (update.borrow_mut())()
    });
    let weak = page.downgrade();
    gtk::glib::timeout_add_local(POWER_EVERY, move || {
        let Some(page) = weak.upgrade() else {
            return gtk::glib::ControlFlow::Break;
        };
        if page.is_mapped() {
            (update.borrow_mut())();
        }
        gtk::glib::ControlFlow::Continue
    });
    page.upcast()
}

/// The charger row: its rating when the kernel knows it.
fn charger_text(reading: &supplies::Reading) -> String {
    match (reading.plugged, reading.charger_watts) {
        (true, Some(watts)) => format!("{watts:.0} W, plugged in"),
        (true, None) => "Plugged in".to_string(),
        (false, _) => "Not plugged in".to_string(),
    }
}
