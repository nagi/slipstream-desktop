//! The colours Slipstream is drawn in: its own, or a followed Omarchy theme's.
//!
//! Every colour in the code is still written as Slipstream's own, `0xrrggbbaa`, and says what
//! kind it is by the function it goes through. A neutral, anything from the bar's near-black to
//! white text, goes through `tone`. A colour that means something goes through the function
//! named for what it means: `accent` for Slipstream's cyan, `warm` for its amber, `green`, `red`
//! and the rest for those. With no theme each hands back the colour it was given, so the
//! desktop without Omarchy is the desktop as it was drawn. With one, each hands back the theme's
//! colour for it, at the same opacity (`slipstream_config::theme::Palette` says which).
//!
//! The theme is read when Slipstream starts and again whenever Omarchy changes it
//! (`watch::watch_theme`), and `Slipstream::apply_theme` has everything painted in the old
//! colours painted again.

use std::sync::{
    RwLock,
    atomic::{AtomicU64, Ordering},
};

use slipstream_config::theme::{self, Palette, Rgb};
use smithay::backend::renderer::Color32F;

/// The followed theme, or `None` for Slipstream's own colours. Read wherever something is
/// painted, on whichever thread paints it, and written by the main loop.
static THEME: RwLock<Option<Palette>> = RwLock::new(None);

/// Counts the changes of colours, so whatever keeps a picture it painted can tell that the
/// colours it was painted in have gone.
static CHANGES: AtomicU64 = AtomicU64::new(0);

/// Follows `palette` from now on, or goes back to Slipstream's own colours. Returns whether
/// that changed anything.
pub fn set(palette: Option<Palette>) -> bool {
    let mut held = THEME
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if *held == palette {
        return false;
    }
    *held = palette;
    CHANGES.fetch_add(1, Ordering::Relaxed);
    true
}

/// How many times the colours have changed since Slipstream started.
pub fn changes() -> u64 {
    CHANGES.load(Ordering::Relaxed)
}

/// The followed theme's palette, for what recolours a great many colours at once and can't ask
/// for each.
pub fn current() -> Option<Palette> {
    with(|palette| palette.cloned())
}

/// What `with` hands a followed theme's palette to.
fn with<T>(read: impl FnOnce(Option<&Palette>) -> T) -> T {
    read(
        THEME
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref(),
    )
}

fn unpack(rgba: u32) -> (Rgb, u8) {
    let [r, g, b, a] = rgba.to_be_bytes();
    ([r, g, b], a)
}

fn pack(rgb: Rgb, alpha: u8) -> u32 {
    u32::from_be_bytes([rgb[0], rgb[1], rgb[2], alpha])
}

/// `own` as the theme has it, at `own`'s opacity: `pick` chooses the theme's colour for it.
fn themed(palette: Option<&Palette>, own: u32, pick: impl FnOnce(&Palette, Rgb) -> Rgb) -> u32 {
    match palette {
        Some(palette) => {
            let (rgb, alpha) = unpack(own);
            pack(pick(palette, rgb), alpha)
        }
        None => own,
    }
}

fn tone_in(palette: Option<&Palette>, own: u32) -> u32 {
    themed(palette, own, |palette, rgb| palette.tone(rgb))
}

/// One of Slipstream's neutrals: a surface, a line, or text from white down to the faintest.
pub fn tone(own: u32) -> u32 {
    with(|palette| tone_in(palette, own))
}

/// A neutral on the ground that light is drawn over: the streams of rain and the bar's strip,
/// which stay dark under a light theme, since light only shows on dark.
pub fn night(own: u32) -> u32 {
    with(|palette| themed(palette, own, |palette, rgb| palette.night_tone(rgb)))
}

/// Slipstream's cyan: what is live, and where the keyboard is.
pub fn accent(own: u32) -> u32 {
    with(|palette| themed(palette, own, |palette, _| palette.accent))
}

/// Slipstream's amber: what is chosen, what wants attention, and bullet time.
pub fn warm(own: u32) -> u32 {
    with(|palette| themed(palette, own, |palette, _| palette.warm))
}

/// Slipstream's mint and the rain's green: what is working, and what is well.
pub fn green(own: u32) -> u32 {
    with(|palette| themed(palette, own, |palette, _| palette.green))
}

/// What is wrong, and what is recording.
pub fn red(own: u32) -> u32 {
    with(|palette| themed(palette, own, |palette, _| palette.red))
}

/// What is running hot.
pub fn orange(own: u32) -> u32 {
    with(|palette| themed(palette, own, |palette, _| palette.orange))
}

pub fn magenta(own: u32) -> u32 {
    with(|palette| themed(palette, own, |palette, _| palette.magenta))
}

/// A second, cooler blue-green beside the accent.
pub fn cyan(own: u32) -> u32 {
    with(|palette| themed(palette, own, |palette, _| palette.cyan))
}

/// A lighter or darker shade of a signal colour that Slipstream writes as its own colour, as
/// `own`: the theme's `colour` moved `towards_ink` of the way to its ink, or when that is
/// negative, that far to its ground.
fn shade(own: u32, towards_ink: f32, colour: impl FnOnce(&Palette) -> Rgb) -> u32 {
    with(|palette| {
        themed(palette, own, |palette, _| {
            let end = if towards_ink < 0.0 {
                palette.ground
            } else {
                palette.ink
            };
            theme::mix(colour(palette), end, towards_ink.abs())
        })
    })
}

/// A shade of the accent: `own` is how Slipstream writes it.
pub fn accent_shade(own: u32, towards_ink: f32) -> u32 {
    shade(own, towards_ink, |palette| palette.accent)
}

/// A shade of the warm colour: `own` is how Slipstream writes it.
pub fn warm_shade(own: u32, towards_ink: f32) -> u32 {
    shade(own, towards_ink, |palette| palette.warm)
}

/// The rain's green lit towards white, `own` as Slipstream writes it, on the ground that light
/// is drawn over.
pub fn green_lit(own: u32, towards_white: f32) -> u32 {
    with(|palette| {
        themed(palette, own, |palette, _| {
            theme::mix(palette.green, palette.night_ink, towards_white)
        })
    })
}

/// The ink for writing on `fill`, a colour already as the theme has it: `own` is the ink
/// Slipstream uses there, and under a theme it is whichever end of the theme stands out more.
pub fn on(fill: u32, own: u32) -> u32 {
    with(|palette| themed(palette, own, |palette, _| palette.on(unpack(fill).0)))
}

/// Quieter writing on the warm colour, `own` as Slipstream writes it: the warm colour taken
/// `towards_ink` of the way to the ink that goes on it.
pub fn on_warm(own: u32, towards_ink: f32) -> u32 {
    with(|palette| {
        themed(palette, own, |palette, _| {
            theme::mix(palette.warm, palette.on(palette.warm), towards_ink)
        })
    })
}

/// `own`, red, green and blue from 0 to 1, as the theme has it: `pick` chooses the theme's
/// colour for it. The same choices as the packed colours above, for what is drawn by a shader or
/// worked out in floats, where Slipstream's own colour has to come back exactly as written.
fn themed_rgb(own: [f32; 3], pick: impl FnOnce(&Palette, Rgb) -> Rgb) -> [f32; 3] {
    with(|palette| match palette {
        Some(palette) => {
            let bytes = own.map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8);
            pick(palette, bytes).map(|channel| channel as f32 / 255.0)
        }
        None => own,
    })
}

pub fn tone_rgb(own: [f32; 3]) -> [f32; 3] {
    themed_rgb(own, |palette, rgb| palette.tone(rgb))
}

pub fn night_rgb(own: [f32; 3]) -> [f32; 3] {
    themed_rgb(own, |palette, rgb| palette.night_tone(rgb))
}

pub fn green_rgb(own: [f32; 3]) -> [f32; 3] {
    themed_rgb(own, |palette, _| palette.green)
}

/// A packed colour's red, green and blue, each from 0 to 1.
pub fn rgb(rgba: u32) -> [f32; 3] {
    let (rgb, _) = unpack(rgba);
    rgb.map(|channel| channel as f32 / 255.0)
}

/// A packed colour as the renderer takes one: premultiplied by its opacity.
pub fn colour32(rgba: u32) -> Color32F {
    let [r, g, b] = rgb(rgba);
    let alpha = (rgba & 0xff) as f32 / 255.0;
    Color32F::new(r * alpha, g * alpha, b * alpha, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokyo_night() -> Palette {
        let text = "background = \"#1a1b26\"\nforeground = \"#a9b1d6\"\n\
                    bright_foreground = \"#c0caf5\"\naccent = \"#7aa2f7\"\nyellow = \"#e0af68\"\n";
        Palette::of(&theme::parse(text, false).unwrap())
    }

    #[test]
    fn with_no_theme_every_colour_is_slipstreams_own() {
        assert_eq!(tone_in(None, 0xe7ebf1ff), 0xe7ebf1ff);
        assert_eq!(
            themed(None, 0x42d3ffcc, |palette, _| palette.accent),
            0x42d3ffcc
        );
    }

    #[test]
    fn a_themed_colour_keeps_its_opacity() {
        let palette = tokyo_night();
        assert_eq!(
            themed(Some(&palette), 0x42d3ffcc, |palette, _| palette.accent),
            0x7aa2f7cc
        );
        assert_eq!(tone_in(Some(&palette), 0xffffff12), 0xa9b1d612);
        assert_eq!(tone_in(Some(&palette), 0x0b0d1200), 0x1a1b2600);
    }

    #[test]
    fn writing_on_a_fill_takes_whichever_end_of_the_theme_stands_out() {
        let palette = tokyo_night();
        let on = |fill: u32| {
            themed(Some(&palette), 0x1a1206ff, |palette, _| {
                palette.on(unpack(fill).0)
            })
        };
        assert_eq!(on(0xe0af68ff), 0x1a1b26ff, "the ground on a pale colour");
        assert_eq!(on(0x3d59a1ff), 0xa9b1d6ff, "the ink on a deep one");
    }

    #[test]
    fn colours_pack_and_unpack() {
        assert_eq!(rgb(0xff8000cc), [1.0, 128.0 / 255.0, 0.0]);
        let half = colour32(0xff000080);
        assert!((half.r() - 0.502).abs() < 0.01 && (half.a() - 0.502).abs() < 0.01);
        assert_eq!(half.g(), 0.0);
    }
}
