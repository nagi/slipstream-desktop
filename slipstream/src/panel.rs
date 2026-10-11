//! What Slipstream's panels share, quick settings and the notification centre: a glass card at the
//! top right, just below the bar, that slides down into place as it opens. Only one panel, or the
//! explorer, is open at a time.

use crate::motion::REDUCED_FADE;
use crate::{
    Slipstream,
    motion::HYPR,
    paint::Painter,
    palette::{self, tone},
};

/// Logical pixels per design pixel. Every surface is laid out in design pixels, sized for a 1.25×
/// screen: each is 0.8 logical pixels, which there is exactly one physical pixel.
pub const DESIGN_PX: f32 = 0.8;
/// The card's top right corner, in design pixels: 10 in from the screen's edge, and 48 down, which
/// is 8 below the bar.
pub const RIGHT: f32 = 10.0;
pub const TOP: f32 = 48.0;
/// Room around the card for its shadow, and more below it, where the shadow falls.
pub const MARGIN: f32 = 64.0;
pub const BELOW: f32 = 32.0;
pub const PADDING: f32 = 18.0;
/// Between the card's rows.
pub const GAP: f32 = 14.0;

/// Text, from the brightest down: what's typed or chosen, headings and names, prose, then two
/// steps of secondary text above `hint` and `placeholder`.
pub fn bright() -> u32 {
    tone(0xf2f4f8ff)
}
pub fn ink() -> u32 {
    tone(0xe7ebf1ff)
}
pub fn prose() -> u32 {
    tone(0xdfe5eeff)
}
pub fn secondary() -> u32 {
    tone(0xaab2c0ff)
}
pub fn tertiary() -> u32 {
    tone(0x9aa4b6ff)
}
pub fn amber() -> u32 {
    palette::warm(0xffb547ff)
}
/// Text on amber.
pub fn dark() -> u32 {
    palette::on(amber(), 0x1a1206ff)
}
/// The code rain's mint, and the charging battery's.
pub fn mint() -> u32 {
    palette::green(0x3cf0c0ff)
}
/// A hairline between a panel's parts.
pub fn divider() -> u32 {
    tone(0xffffff12)
}
/// A quiet button or keycap, the same under the pointer, and its 1 px edge.
pub fn quiet() -> u32 {
    tone(0xffffff12)
}
pub fn quiet_lit() -> u32 {
    tone(0xffffff26)
}
pub fn edge() -> u32 {
    tone(0xffffff24)
}
/// A tile that isn't lit: a switcher window, a notification, a quick settings toggle.
pub fn tile() -> u32 {
    tone(0xffffff0a)
}
/// Corner radii: tiles, and selectable rows and rectangular buttons.
pub const TILE_RADIUS: f32 = 12.0;
pub const ROW_RADIUS: f32 = 9.0;
/// The near-black of dark chips, the bar and wells, without its alpha: each surface picks its
/// own. The same as the code rain's cards, so they sit on the default wallpaper as one.
pub fn chip() -> u32 {
    tone(0x0b0d1200)
}
/// A level's empty track on a notice.
pub fn track() -> u32 {
    tone(0xffffff1a)
}
/// How far in from the frame's left and right edges a panel's head row sits.
pub const HEAD_INSET: f32 = 26.0;
/// Where the keyboard is, until the settings are read: the focused window's ring colour at the
/// same four fifths' opacity the window ring is drawn with. `Slipstream::panel_ring` hands the
/// chosen one to each panel.
pub fn focus() -> u32 {
    palette::accent(0x42d3ffcc)
}

/// Opening: 200 ms from 10 px higher and transparent. Closing is instant.
const OPEN: f64 = 0.2;

/// The card, `w` × `h` at (`x`, `y`), with its shadow. With nothing blurred behind it, the glass is
/// nearly solid, as the explorer's is.
pub fn glass(p: &mut Painter, x: f32, y: f32, w: f32, h: f32) {
    p.card(
        x,
        y,
        w,
        h,
        18.0,
        (30.0, 90.0, 0x000000b0),
        glass_fill(),
        glass_edge(),
    );
}

/// Large panels' glass: the explorer, quick settings, the centre, the cards and the sheet.
pub fn glass_fill() -> u32 {
    tone(0x10131af8)
}
pub fn glass_edge() -> u32 {
    tone(0xffffff1c)
}

/// The small surfaces that come and go by themselves: the toast, the slim card, notification
/// pop-ups and the volume display. Solid, so nothing behind shows through.
pub fn notice_fill() -> u32 {
    tone(0x141925ff)
}
pub fn notice_edge() -> u32 {
    tone(0xffffff18)
}
pub const NOTICE_RADIUS: f32 = 12.0;
/// How far a notice's shadow reaches past its card: down by its offset and out by half its blur.
pub const NOTICE_SHADOW: (f32, f32, u32) = (20.0, 50.0, 0x000000aa);
pub const NOTICE_MARGIN: f32 = 48.0;

/// Text that teaches a key or says something quietly: 6.4:1 on the glass, 5.4:1 in a quiet
/// button.
pub fn hint() -> u32 {
    tone(0x8f98a8ff)
}
/// What an empty search box says.
pub fn placeholder() -> u32 {
    tone(0x7d8697ff)
}

/// Dark ink on a light colour, light ink on a dark one.
pub fn ink_on(rgba: u32) -> u32 {
    let [r, g, b, _] = rgba.to_be_bytes().map(|c| c as f32 / 255.0);
    let own = if 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.45 {
        0x1a1206ff
    } else {
        0xf2f4f8ff
    };
    palette::on(rgba, own)
}

/// A selection's fill: the ring colour, faint.
pub fn selection_fill(ring: u32) -> u32 {
    (ring & 0xffffff00) | 0x1c
}

/// The chosen row of a list: its box filled faintly in the ring colour, with a 2 px edge in it.
pub fn selected_row(p: &mut Painter, x: f32, y: f32, w: f32, h: f32, ring: u32) {
    p.fill(x, y, w, h, ROW_RADIUS, selection_fill(ring));
    p.border(x, y, w, h, ROW_RADIUS, 2.0, ring);
}

/// A quiet button's box: faint, lighter under the pointer (`lit`), with a 1 px edge.
pub fn quiet_button(p: &mut Painter, x: f32, y: f32, w: f32, h: f32, radius: f32, lit: bool) {
    p.fill(x, y, w, h, radius, if lit { quiet_lit() } else { quiet() });
    p.border(x, y, w, h, radius, 1.0, edge());
}

/// A section's title in small capitals.
pub fn section_style() -> crate::text::Style {
    crate::text::Style {
        tracking: 0.12,
        ..crate::text::Style::new(crate::text::Face::Body, 12.0, placeholder())
    }
}

/// An app with no icon: a `side`-pixel square at (`x`, `y`), coloured by its name, with its
/// initial in capitals `px` pixels tall.
pub fn app_placeholder(
    p: &mut Painter,
    name: &str,
    x: f32,
    y: f32,
    side: f32,
    radius: f32,
    px: f32,
) {
    let colours = [
        mint(),
        palette::accent(0x33ccffff),
        amber(),
        palette::red(0xff7a93ff),
        palette::magenta(0xa78bfaff),
        palette::cyan(0x7fe3ffff),
    ];
    let hash = name.bytes().fold(0u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as u32)
    });
    let fill = colours[hash as usize % colours.len()];
    p.fill(x, y, side, side, radius, fill);
    let initial: String = name
        .chars()
        .next()
        .map(|ch| ch.to_uppercase().collect())
        .unwrap_or_default();
    let style = crate::text::Style::new(
        crate::text::Face::MonoBold,
        px,
        palette::on(fill, 0x10131aff),
    );
    let w = crate::text::width(&initial, &style);
    p.text(&initial, x + (side - w) / 2.0, y + side / 2.0, &style);
}

/// A notice's card, `w` × `h` at (`x`, `y`), with its shadow.
pub fn notice(p: &mut Painter, x: f32, y: f32, w: f32, h: f32) {
    let (dy, blur, shadow) = NOTICE_SHADOW;
    p.shadow(x, y, w, h, NOTICE_RADIUS, dy, blur, shadow);
    p.fill(x, y, w, h, NOTICE_RADIUS, notice_fill());
    p.border(x, y, w, h, NOTICE_RADIUS, 1.0, notice_edge());
}

/// Between a footer's hints.
pub const HINT_GAP: f32 = 20.0;
/// Between a hint's keycaps, and before its words.
const KEY_GAP: f32 = 4.0;

fn hint_style() -> crate::text::Style {
    crate::text::Style::new(crate::text::Face::Body, 13.0, hint())
}

/// How wide `key_hint` draws `keys` and `label`.
pub fn key_hint_width(keys: &[&str], label: &str) -> f32 {
    let caps: f32 = keys
        .iter()
        .map(|key| crate::paint::keycap_width(key) + KEY_GAP)
        .sum();
    let label_w = if label.is_empty() {
        0.0
    } else {
        KEY_GAP + crate::text::width(label, &hint_style())
    };
    caps - KEY_GAP + label_w
}

/// A footer's hint ending at `right`, centred on `centre`: its keycaps, then what they do.
/// Returns where it starts.
pub fn key_hint(p: &mut Painter, right: f32, centre: f32, keys: &[&str], label: &str) -> f32 {
    let style = hint_style();
    let start = right - key_hint_width(keys, label);
    let mut x = start;
    for key in keys {
        x += p.keycap(key, x, centre) + KEY_GAP;
    }
    if !label.is_empty() {
        p.text(label, x, centre, &style);
    }
    start
}

/// The keyboard's ring, just outside a control's box.
pub fn focus_ring(p: &mut Painter, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: u32) {
    p.border(
        x - 4.0,
        y - 4.0,
        w + 8.0,
        h + 8.0,
        radius + 4.0,
        2.0,
        colour,
    );
}

/// A panel `since` seconds after it opened: its opacity, and how far above its place it is, in
/// design pixels.
pub fn opening(since: f64, reduced_motion: bool) -> (f32, f64) {
    if reduced_motion {
        ((since / REDUCED_FADE).clamp(0.0, 1.0) as f32, 0.0)
    } else {
        let eased = HYPR.at((since / OPEN).clamp(0.0, 1.0));
        (eased.clamp(0.0, 1.0) as f32, -10.0 * (1.0 - eased))
    }
}

impl Slipstream {
    /// Closes the explorer, quick settings, the notification centre and the shortcut sheet, before
    /// another opens.
    pub fn close_panels(&mut self) {
        if self.explorer.is_open() {
            self.explorer.close();
        }
        self.quick.close();
        self.centre.close();
        self.sheet.close();
        self.history.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG's contrast ratio between two colours, 0xRRGGBBAA, the alpha ignored.
    fn contrast(a: u32, b: u32) -> f64 {
        let luminance = |rgba: u32| {
            let [r, g, b, _] = rgba.to_be_bytes().map(|c| {
                let c = c as f64 / 255.0;
                if c <= 0.03928 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            });
            0.2126 * r + 0.7152 * g + 0.0722 * b
        };
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn hint_ink_is_at_least_4_5_to_1() {
        let glass = 0x10131aff;
        // A quiet button: white at 7% over the glass.
        let quiet = 0x21232aff;
        assert!(
            contrast(hint(), glass) >= 4.5,
            "{}",
            contrast(hint(), glass)
        );
        assert!(
            contrast(hint(), quiet) >= 4.5,
            "{}",
            contrast(hint(), quiet)
        );
        assert!(contrast(placeholder(), glass) >= 4.5);
    }
}
