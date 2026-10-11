//! The Settings window in the colours the desktop is drawn in: Slipstream's own, as `style.css`
//! writes them, or a followed Omarchy theme's.
//!
//! The stylesheet stays as it is written. Under a theme each colour in it is carried into the
//! theme the way the compositor carries its own (`slipstream_config::theme::Palette`): the
//! signal colours listed here become the theme's, and every other colour is a neutral, a step
//! between the theme's ground and its ink.

use slipstream_config::{
    Settings,
    theme::{self, Palette, Rgb},
};

/// The colours in `style.css` that mean something, and what each is in a theme: the theme's
/// colour for it, and how far that is taken towards the theme's ink.
#[derive(Clone, Copy)]
enum Signal {
    Accent,
    Warm,
    Green,
    Orange,
    Red,
}

const SIGNALS: [(Rgb, Signal, f32); 9] = [
    // What's ticked, switched on or chosen.
    ([0x42, 0xd3, 0xff], Signal::Accent, 0.0),
    // The sidebar's page, and the keyboard's outline.
    ([0xff, 0xb5, 0x47], Signal::Warm, 0.0),
    ([0xff, 0xd0, 0x8a], Signal::Warm, 0.35),
    // The battery's charge, and the light that runs along it.
    ([0x7f, 0xe0, 0xa8], Signal::Green, 0.0),
    ([0xb6, 0xf0, 0xc8], Signal::Green, 0.3),
    ([0xe4, 0xff, 0xf0], Signal::Green, 0.7),
    // The meter, nearly used up.
    ([0xff, 0x7a, 0x45], Signal::Orange, 0.0),
    // What went wrong.
    ([0xff, 0x8a, 0x80], Signal::Red, 0.0),
    ([0xff, 0x5a, 0x5a], Signal::Red, 0.0),
];

/// One of the stylesheet's colours in the theme. Black is left alone: it is only ever a shadow
/// or a line drawn at a few percent.
fn themed(palette: &Palette, own: Rgb) -> Rgb {
    if own == [0; 3] {
        return own;
    }
    match SIGNALS.iter().find(|(colour, ..)| *colour == own) {
        Some((_, signal, towards_ink)) => {
            let colour = match signal {
                Signal::Accent => palette.accent,
                Signal::Warm => palette.warm,
                Signal::Green => palette.green,
                Signal::Orange => palette.orange,
                Signal::Red => palette.red,
            };
            theme::mix(colour, palette.ink, *towards_ink)
        }
        None => palette.tone(own),
    }
}

/// `css` with every colour in it, `#rrggbb` or `rgba(r, g, b, a)`, as the theme has it.
pub fn in_theme(css: &str, palette: &Palette) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    loop {
        let (hex, rgba) = (rest.find('#'), rest.find("rgba("));
        let at = match (hex, rgba) {
            (Some(hex), Some(rgba)) => hex.min(rgba),
            (Some(at), None) | (None, Some(at)) => at,
            (None, None) => break,
        };
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some(digits) = rest.strip_prefix('#') {
            let colour = digits
                .get(..6)
                .filter(|_| !digits[6..].starts_with(|c: char| c.is_ascii_hexdigit()))
                .and_then(theme::rgb);
            match colour {
                Some(own) => {
                    out.push_str(&theme::hex(themed(palette, own)));
                    rest = &digits[6..];
                }
                // An id selector, or a colour in a form this doesn't know.
                None => {
                    out.push('#');
                    rest = digits;
                }
            }
        } else {
            let inside = &rest["rgba(".len()..];
            let end = inside.find(')').unwrap_or(inside.len());
            let parts: Vec<&str> = inside[..end].split(',').map(str::trim).collect();
            let channels: Vec<u8> = parts
                .iter()
                .take(3)
                .filter_map(|part| part.parse().ok())
                .collect();
            match (channels.as_slice(), parts.get(3)) {
                (&[r, g, b], Some(alpha)) if parts.len() == 4 => {
                    let [r, g, b] = themed(palette, [r, g, b]);
                    out.push_str(&format!("rgba({r}, {g}, {b}, {alpha}"));
                    rest = &inside[end..];
                }
                _ => {
                    out.push_str("rgba(");
                    rest = inside;
                }
            }
        }
    }
    out.push_str(rest);
    out
}

/// The palette the desktop is drawn in under `settings`: the followed Omarchy theme's, or `None`
/// for Slipstream's own.
pub fn palette(settings: &Settings) -> Option<Palette> {
    settings.palette(settings.appearance.theme().as_ref())
}

/// A name for the colours the desktop is drawn in, which changes when they do: for anything kept
/// that was drawn in them.
pub fn name(settings: &Settings) -> String {
    match palette(settings) {
        Some(palette) => [
            palette.ground,
            palette.ink,
            palette.accent,
            palette.warm,
            palette.green,
            palette.magenta,
            palette.red,
        ]
        .iter()
        .map(|colour| theme::hex(*colour)[1..].to_string())
        .collect(),
        None => "own".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokyo_night() -> Palette {
        let text = "background = \"#1a1b26\"\nforeground = \"#a9b1d6\"\n\
                    bright_foreground = \"#c0caf5\"\naccent = \"#7aa2f7\"\nyellow = \"#e0af68\"\n\
                    green = \"#9ece6a\"\n";
        Palette::of(&theme::parse(text, false).unwrap())
    }

    #[test]
    fn signal_colours_become_the_themes_and_the_rest_its_neutrals() {
        let palette = tokyo_night();
        let css = ".row switch:checked { border-color: #42d3ff; background: #42d3ff; }\n\
                   window { background-color: #0b0d12; color: #ffffff; }\n\
                   .side > row:selected { background: rgba(255, 181, 71, 0.12); }";
        let themed = in_theme(css, &palette);
        assert_eq!(
            themed,
            ".row switch:checked { border-color: #7aa2f7; background: #7aa2f7; }\n\
             window { background-color: #1a1b26; color: #a9b1d6; }\n\
             .side > row:selected { background: rgba(224, 175, 104, 0.12); }"
        );
    }

    #[test]
    fn white_at_a_few_percent_is_the_themes_ink_and_black_is_left_alone() {
        let palette = tokyo_night();
        assert_eq!(
            in_theme("border: 1px solid rgba(255, 255, 255, 0.05);", &palette),
            "border: 1px solid rgba(169, 177, 214, 0.05);"
        );
        let shadow = "border-bottom: 1px solid rgba(0, 0, 0, 0.3);";
        assert_eq!(in_theme(shadow, &palette), shadow);
    }

    #[test]
    fn what_isnt_a_colour_is_left_as_written() {
        let palette = tokyo_night();
        for css in [
            "#sidebar { margin: 0 }",
            "/* 50% */ @keyframes batt-charging { from { background-position: 100% 0; } }",
            "color: #fff;",
            "background: rgba(var(--r), 0, 0, 1);",
            "trailing #",
            "an unclosed rgba(1, 2",
        ] {
            assert_eq!(in_theme(css, &palette), css);
        }
    }

    #[test]
    fn every_colour_in_the_stylesheet_is_one_this_reads() {
        // A colour written some other way would be left in Slipstream's own under a theme.
        let css = include_str!("style.css");
        let themed = in_theme(css, &tokyo_night());
        assert_eq!(themed.lines().count(), css.lines().count());
        for (line, was) in themed.lines().zip(css.lines()) {
            let coloured = was.contains('#') || was.contains("rgb");
            let comment = was.trim_start().starts_with("/*") || was.trim_start().starts_with('*');
            let black = was.contains("rgba(0, 0, 0,");
            if coloured && !comment && !black {
                assert_ne!(line, was, "not carried into the theme");
            }
        }
    }
}
