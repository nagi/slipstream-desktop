//! An Omarchy theme's colours, for a desktop that follows one.
//!
//! Omarchy keeps the theme it is showing in `~/.local/state/omarchy/current`: `theme.name` holds
//! its name and `theme/colors.toml` its palette. `omarchy theme set` builds the next theme beside
//! the current one and renames it into place, so a reader only ever sees a whole theme.
//!
//! `colors.toml` is read the way Omarchy's own `omarchy-theme-color` reads it, line by line
//! rather than as TOML, so a theme Omarchy accepts is one Slipstream accepts: the same older
//! names (`bg`, `color0` to `color15`, `theme_type`) stand in for the ones a theme leaves out.
//!
//! A `Theme` is what the file says. A `Palette` is what Slipstream makes of it: where each of its
//! own colours lands in the theme, so the bar, the panels, the rings and the wallpapers are drawn
//! from one place.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

/// Red, green and blue, 0 to 255.
pub type Rgb = [u8; 3];

/// Slipstream's own colours, which stand in for whatever a theme doesn't name.
pub const OWN_GROUND: Rgb = [0x0b, 0x0d, 0x12];
/// Its ordinary text.
pub const OWN_INK: Rgb = [0xe7, 0xeb, 0xf1];
pub const OWN_CYAN: Rgb = [0x42, 0xd3, 0xff];
pub const OWN_AMBER: Rgb = [0xff, 0xb5, 0x47];
pub const OWN_MINT: Rgb = [0x3c, 0xf0, 0xc0];
pub const OWN_RED: Rgb = [0xff, 0x5a, 0x5a];
pub const OWN_ORANGE: Rgb = [0xff, 0x7a, 0x45];
pub const OWN_VIOLET: Rgb = [0xa7, 0x8b, 0xfa];
pub const OWN_BLUE: Rgb = [0x62, 0xa0, 0xea];

/// The colours an Omarchy theme names, with its older names resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// As Omarchy stores it, `tokyo-night`. Empty when it didn't say.
    pub name: String,
    /// A light theme: dark ink on a light ground.
    pub light: bool,
    pub background: Rgb,
    pub foreground: Rgb,
    pub bright_foreground: Rgb,
    pub accent: Rgb,
    pub red: Rgb,
    pub green: Rgb,
    pub yellow: Rgb,
    pub orange: Rgb,
    pub blue: Rgb,
    pub magenta: Rgb,
    pub cyan: Rgb,
    /// The icon theme that goes with it, from `icons.theme` beside the colours.
    pub icons: Option<String>,
}

/// The colours of a theme that can be chosen for Slipstream's ring and its highlight: the name
/// the settings file uses, and what the Settings app calls it.
pub const PICKS: [(&str, &str); 9] = [
    ("accent", "Accent"),
    ("foreground", "Foreground"),
    ("red", "Red"),
    ("orange", "Orange"),
    ("yellow", "Yellow"),
    ("green", "Green"),
    ("cyan", "Cyan"),
    ("blue", "Blue"),
    ("magenta", "Magenta"),
];

/// A chosen colour: one of `theme`'s by name, or Slipstream's own of that name without a theme,
/// or `#rrggbb` as it stands. `None` for anything else.
pub fn pick(chosen: &str, theme: Option<&Theme>) -> Option<Rgb> {
    let chosen = chosen.trim();
    rgb(chosen).or_else(|| match theme {
        Some(theme) => theme.colour(chosen),
        None => Theme::own().colour(chosen),
    })
}

/// Where Omarchy keeps the theme it is showing. Omarchy writes under `~/.local/state` whatever
/// `XDG_STATE_HOME` says, so this does too.
pub fn current_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state/omarchy/current")
}

/// The theme Omarchy is showing, or `None` where there is no Omarchy, no theme set, or a theme
/// whose colours can't be read.
pub fn current() -> Option<Theme> {
    read(&current_dir())
}

/// The theme in `current`, Omarchy's folder for the one it is showing.
pub fn read(current: &Path) -> Option<Theme> {
    let dir = current.join("theme");
    let text = fs::read_to_string(dir.join("colors.toml")).ok()?;
    let mut theme = parse(&text, dir.join("light.mode").exists())?;
    theme.name = fs::read_to_string(current.join("theme.name"))
        .map(|name| name.trim().to_string())
        .unwrap_or_default();
    theme.icons = fs::read_to_string(dir.join("icons.theme"))
        .ok()
        .map(|icons| icons.trim().to_string())
        .filter(|icons| !icons.is_empty() && !icons.contains('/'));
    Some(theme)
}

/// A `colors.toml`'s colours. `light_file` says the theme has a `light.mode` file beside it,
/// which is how an older theme says it is light. `None` when the file names neither a
/// background nor a foreground: there is nothing to draw a desktop from.
pub fn parse(text: &str, light_file: bool) -> Option<Theme> {
    let raw = raw_values(text);
    let colour = |names: &[&str]| names.iter().find_map(|name| rgb(raw.get(*name)?));
    let background = colour(&["background", "bg", "color0"])?;
    let foreground = colour(&["foreground", "fg", "color7"])?;
    let mode = ["mode", "theme_type"]
        .iter()
        .find_map(|name| raw.get(*name))
        .map(|mode| mode.to_ascii_lowercase());
    let light = match mode.as_deref() {
        Some(mode) => mode == "light",
        // Omarchy's own test: a ground whose channels sum past half way is a light one.
        None => light_file || background.iter().map(|c| *c as u32).sum::<u32>() > 382,
    };
    let blue = colour(&["blue", "color4"]);
    let yellow = colour(&["yellow", "color3"]);
    Some(Theme {
        name: String::new(),
        light,
        background,
        foreground,
        bright_foreground: colour(&["bright_foreground", "bright_fg", "color15"])
            .unwrap_or(foreground),
        accent: colour(&["accent"]).or(blue).unwrap_or(OWN_CYAN),
        red: colour(&["red", "color1"]).unwrap_or(OWN_RED),
        green: colour(&["green", "color2"]).unwrap_or(OWN_MINT),
        yellow: yellow.unwrap_or(OWN_AMBER),
        orange: colour(&["orange"]).or(yellow).unwrap_or(OWN_ORANGE),
        blue: blue.unwrap_or(OWN_BLUE),
        magenta: colour(&["magenta", "color5", "purple"]).unwrap_or(OWN_VIOLET),
        cyan: colour(&["cyan", "color6"]).unwrap_or(OWN_CYAN),
        icons: None,
    })
}

impl Theme {
    /// Slipstream's own colours under a theme's names, for a colour chosen by name where no
    /// theme is followed.
    pub fn own() -> Self {
        Self {
            name: String::new(),
            light: false,
            background: OWN_GROUND,
            foreground: OWN_INK,
            bright_foreground: [0xf2, 0xf4, 0xf8],
            accent: OWN_CYAN,
            red: OWN_RED,
            green: OWN_MINT,
            yellow: OWN_AMBER,
            orange: OWN_ORANGE,
            blue: OWN_BLUE,
            magenta: OWN_VIOLET,
            cyan: [0x7f, 0xe3, 0xff],
            icons: None,
        }
    }

    /// One of the theme's colours by its name in `PICKS`, in either case.
    pub fn colour(&self, name: &str) -> Option<Rgb> {
        Some(match name.trim().to_ascii_lowercase().as_str() {
            "accent" => self.accent,
            "foreground" => self.foreground,
            "red" => self.red,
            "orange" => self.orange,
            "yellow" => self.yellow,
            "green" => self.green,
            "cyan" => self.cyan,
            "blue" => self.blue,
            "magenta" => self.magenta,
            _ => return None,
        })
    }

    /// The name as Omarchy shows it: `tokyo-night` is Tokyo Night.
    pub fn title(&self) -> String {
        self.name
            .split('-')
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut letters = word.chars();
                letters
                    .next()
                    .map(|first| first.to_uppercase().chain(letters).collect::<String>())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Every `key = value` line, as `omarchy-theme-color` reads them: the key up to the first `=`
/// without quotes or spaces, and the value between its first pair of quotes, or trimmed when it
/// has none. Comments, section headers and anything else that isn't a plain key are skipped, and
/// a key given twice keeps its last value.
fn raw_values(text: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key: String = key
            .chars()
            .filter(|c| !matches!(c, '"' | '\'' | ' ' | '\t'))
            .collect();
        let plain = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
        if key.is_empty() || !key.chars().all(plain) {
            continue;
        }
        let quote = |c: char| c == '"' || c == '\'';
        let value = match value.find(quote) {
            Some(open) => {
                let rest = &value[open + 1..];
                &rest[..rest.find(quote).unwrap_or(rest.len())]
            }
            None => value.trim(),
        };
        values.insert(key, value.to_string());
    }
    values
}

/// `#rrggbb`, with or without the `#` and either case. A colour with an alpha after it,
/// `#rrggbbaa`, is read without it. Anything else is `None`.
pub fn rgb(text: &str) -> Option<Rgb> {
    let digits = text.trim().strip_prefix('#').unwrap_or(text.trim());
    if !matches!(digits.len(), 6 | 8) || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// `#rrggbb`.
pub fn hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// `a` with `t` of the way to `b` mixed in.
pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8)
}

/// WCAG's relative luminance, 0 for black to 1 for white.
pub fn luminance(rgb: Rgb) -> f64 {
    let [r, g, b] = rgb.map(|c| {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG's contrast ratio between two colours, from 1 for the same to 21 for black on white.
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// How far apart two colours are, 0 for the same to 441 for black and white.
#[cfg(test)]
fn distance(a: Rgb, b: Rgb) -> f32 {
    (0..3)
        .map(|i| (a[i] as f32 - b[i] as f32).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// The desktop's backdrop, which the living wallpaper is drawn over: a shade off `OWN_GROUND`.
pub const OWN_BACKDROP: Rgb = [11, 14, 19];
/// The hues of Slipstream's amber, mint, cyan, violet and rose, in degrees round the wheel.
const OWN_HUES: [f32; 5] = [36.0, 163.0, 195.0, 262.0, 335.0];
/// How much of the light of Slipstream's least saturated signal colour is colour, not white.
/// Anything this saturated or more is the theme's colour outright.
const OWN_SATURATION: f32 = 0.7;

/// A colour's hue in degrees, red at 0. A grey has none, and is given 0.
fn hue([r, g, b]: [f32; 3]) -> f32 {
    let most = r.max(g).max(b);
    let spread = most - r.min(g).min(b);
    if spread <= 0.0 {
        return 0.0;
    }
    let sixth = if most == r {
        ((g - b) / spread).rem_euclid(6.0)
    } else if most == g {
        (b - r) / spread + 2.0
    } else {
        (r - g) / spread + 4.0
    };
    sixth * 60.0
}

/// How far a light theme's ink is taken towards black for the ground that light is drawn over.
const NIGHT: f32 = 0.55;

/// Where Slipstream's colours land in a theme.
///
/// Slipstream draws with three kinds of colour. Its neutrals, from the near-black of the bar up
/// to white text, become steps between the theme's background and its foreground (`tone`): the
/// whole of the desktop's chrome comes from those two colours. Its two signal colours are the
/// two chosen in the settings from the theme's own: cyan, for the ring and what is live, and
/// amber, for what is switched on or chosen. The rest are the theme's red, green and so on.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub light: bool,
    /// The theme's background: the desktop behind the windows, and the bar.
    pub ground: Rgb,
    /// The theme's foreground: what Slipstream's white becomes.
    pub ink: Rgb,
    /// For Slipstream's cyan: the ring's colour.
    pub accent: Rgb,
    /// For Slipstream's amber: the highlight's colour.
    pub warm: Rgb,
    pub red: Rgb,
    pub green: Rgb,
    pub orange: Rgb,
    pub blue: Rgb,
    pub magenta: Rgb,
    pub cyan: Rgb,
    /// The ground under anything drawn in light, which only shows on dark: the streams of
    /// rain, and the strip a docked window's runs along. A dark theme's own ground. A light
    /// theme's ink, darkened, with its ground for the ink.
    pub night_ground: Rgb,
    pub night_ink: Rgb,
    /// How far from `ground` to `ink` a neutral of each brightness goes, by its luma from 0 to
    /// 255, and the same from `night_ground` to `night_ink`.
    tones: [f32; 256],
    night_tones: [f32; 256],
}

impl Palette {
    /// The palette with the ring and the highlight left as they come: the theme's accent and
    /// its yellow.
    pub fn of(theme: &Theme) -> Self {
        Self::with(theme, theme.accent, theme.yellow)
    }

    /// The palette with `ring` for Slipstream's cyan and `highlight` for its amber.
    pub fn with(theme: &Theme, ring: Rgb, highlight: Rgb) -> Self {
        let (ground, ink) = (theme.background, theme.foreground);
        let (night_ground, night_ink) = if theme.light {
            (mix(ink, [0; 3], NIGHT), ground)
        } else {
            (ground, ink)
        };
        Self {
            light: theme.light,
            ground,
            ink,
            night_ground,
            night_ink,
            night_tones: tones(night_ground, night_ink),
            accent: ring,
            warm: highlight,
            red: theme.red,
            green: theme.green,
            orange: theme.orange,
            blue: theme.blue,
            magenta: theme.magenta,
            cyan: theme.cyan,
            tones: tones(ground, ink),
        }
    }

    /// One of Slipstream's neutrals in the theme: as far up the theme's contrast, from its
    /// background to its foreground, as the neutral sits up Slipstream's own, from the bar's
    /// near-black to its ordinary text. Text that was comfortably readable stays as readable as the
    /// theme allows, and a surface a shade off the ground stays a shade off it.
    pub fn tone(&self, own: Rgb) -> Rgb {
        mix(self.ground, self.ink, self.tones[luma(own)])
    }

    /// A neutral on the ground that light is drawn over, as `tone` is on the theme's own.
    pub fn night_tone(&self, own: Rgb) -> Rgb {
        mix(
            self.night_ground,
            self.night_ink,
            self.night_tones[luma(own)],
        )
    }

    /// One of the living wallpaper's colours in the theme. The wallpaper is drawn in light over
    /// Slipstream's backdrop, in mixtures of its white, cyan, amber and mint, so a colour is
    /// taken apart the way it was put together: how much light it adds to the backdrop, how
    /// much of that is colour rather than white, and which colour. It is put back together
    /// over the theme's ground, from the theme's ink and the theme's colour for that hue.
    ///
    /// Under a light theme that makes it ink on paper: the more light a cell had, the more ink
    /// it gets.
    pub fn lit(&self, own: Rgb) -> Rgb {
        let light: [f32; 3] = std::array::from_fn(|i| {
            let ground = OWN_BACKDROP[i] as f32;
            ((own[i] as f32 - ground) / (255.0 - ground)).clamp(0.0, 1.0)
        });
        let most = light[0].max(light[1]).max(light[2]);
        let least = light[0].min(light[1]).min(light[2]);
        if most <= 0.0 {
            return self.ground;
        }
        let colour = ((most - least) / most / OWN_SATURATION).min(1.0);
        let ink = mix(self.ink, self.hue(hue(light)), colour);
        mix(self.ground, ink, most)
    }

    /// The theme's colour for a hue of Slipstream's, in degrees: its warm colour where
    /// Slipstream's amber is, its green at the mint, its accent at the cyan, its magenta at the
    /// violet and its red at the rose, and a mixture of the two either side in between.
    fn hue(&self, degrees: f32) -> Rgb {
        let ring = [
            (OWN_HUES[0], self.warm),
            (OWN_HUES[1], self.green),
            (OWN_HUES[2], self.accent),
            (OWN_HUES[3], self.magenta),
            (OWN_HUES[4], self.red),
        ];
        let degrees = degrees.rem_euclid(360.0);
        for (i, (at, colour)) in ring.iter().enumerate() {
            let (next_at, next) = ring[(i + 1) % ring.len()];
            let span = (next_at - at).rem_euclid(360.0);
            let along = (degrees - at).rem_euclid(360.0);
            if along <= span {
                return mix(*colour, next, along / span);
            }
        }
        self.warm
    }

    /// The ink for writing on `fill`: whichever end of the theme, its ground or its ink, stands
    /// out from it more. Themes put their colours anywhere from pale to deep, so neither end is
    /// right for all of them.
    pub fn on(&self, fill: Rgb) -> Rgb {
        if contrast(self.ground, fill) >= contrast(self.ink, fill) {
            self.ground
        } else {
            self.ink
        }
    }
}

/// A colour's brightness as a grey would have it, 0 to 255.
fn luma(rgb: Rgb) -> usize {
    let luma = 0.2126 * rgb[0] as f32 + 0.7152 * rgb[1] as f32 + 0.0722 * rgb[2] as f32;
    luma.round().clamp(0.0, 255.0) as usize
}

/// For each luma a neutral of Slipstream's can have, how far from `ground` to `ink` gives the
/// same share of the theme's contrast as the neutral has of Slipstream's.
fn tones(ground: Rgb, ink: Rgb) -> [f32; 256] {
    let own_ground = luma(OWN_GROUND);
    // Slipstream's ordinary text is the theme's foreground, and its brighter text with it: a
    // theme has nothing brighter to write in.
    let own_span = contrast(OWN_INK, OWN_GROUND).ln();
    let span = contrast(ink, ground).ln();
    std::array::from_fn(|luma| {
        // Slipstream's ground and anything darker is the theme's ground, and white its ink.
        if luma <= own_ground || span <= 0.0 {
            return 0.0;
        }
        if luma >= self::luma(OWN_INK) {
            return 1.0;
        }
        let own = contrast([luma as u8; 3], OWN_GROUND).ln() / own_span;
        let wanted = (own.clamp(0.0, 1.0) * span).exp();
        // Contrast against the ground rises along the way from it to the ink.
        let (mut low, mut high) = (0.0f32, 1.0f32);
        for _ in 0..16 {
            let middle = (low + high) / 2.0;
            if contrast(mix(ground, ink, middle), ground) < wanted {
                low = middle;
            } else {
                high = middle;
            }
        }
        (low + high) / 2.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKYO_NIGHT: &str = r##"
mode = "dark"

accent = "#7aa2f7"
selection = "#292e42"

background = "#1a1b26"
foreground = "#a9b1d6"
bright_foreground = "#c0caf5"

red = "#f7768e"
yellow = "#e0af68"
orange = "#eb927b"
green = "#9ece6a"
cyan = "#449dab"
blue = "#7aa2f7"
magenta = "#ad8ee6"
"##;

    #[test]
    fn a_theme_is_read_by_its_names() {
        let theme = parse(TOKYO_NIGHT, false).unwrap();
        assert!(!theme.light);
        assert_eq!(theme.background, [0x1a, 0x1b, 0x26]);
        assert_eq!(theme.bright_foreground, [0xc0, 0xca, 0xf5]);
        assert_eq!(theme.accent, [0x7a, 0xa2, 0xf7]);
        assert_eq!(theme.yellow, [0xe0, 0xaf, 0x68]);
    }

    #[test]
    fn an_older_theme_is_read_by_its_terminal_colours() {
        let text = r##"
            accent = "#62e2a4"
            foreground = "#fdfffd"
            background = "#010401"
            color1 = "#bf5a7c"
            color2 = "#70cf6c"
            color3 = "#dfec63"
            color4 = "#62e2a4"
            color5 = "#e0eb7a"
            color6 = "#9ed8dd"
            color15 = "#fdfffd"
        "##;
        let theme = parse(text, false).unwrap();
        assert!(!theme.light);
        assert_eq!(theme.red, [0xbf, 0x5a, 0x7c]);
        assert_eq!(theme.green, [0x70, 0xcf, 0x6c]);
        // No orange of its own: the yellow stands in, as it does in Omarchy.
        assert_eq!(theme.orange, theme.yellow);
        assert_eq!(theme.bright_foreground, [0xfd, 0xff, 0xfd]);
    }

    #[test]
    fn the_shortest_names_and_the_terminal_ground_stand_in() {
        let theme = parse("bg = '#101010' # the ground\nfg = \"#eeeeee\"\n", false).unwrap();
        assert_eq!(theme.background, [0x10; 3]);
        assert_eq!(theme.foreground, [0xee; 3]);
        // Nothing else named: Slipstream's own colours.
        assert_eq!(theme.accent, OWN_CYAN);
        assert_eq!(theme.yellow, OWN_AMBER);
        let theme = parse(
            "color0 = #202020\ncolor7 = #dddddd\ncolor4 = #3366ff",
            false,
        )
        .unwrap();
        assert_eq!(theme.background, [0x20; 3]);
        assert_eq!(theme.accent, [0x33, 0x66, 0xff], "blue is the accent");
    }

    #[test]
    fn a_theme_says_it_is_light_in_any_of_omarchys_ways() {
        let colours = "background = \"#101010\"\nforeground = \"#eeeeee\"\n";
        assert!(!parse(colours, false).unwrap().light);
        assert!(parse(colours, true).unwrap().light, "a light.mode file");
        assert!(
            parse(&format!("mode = \"light\"\n{colours}"), false)
                .unwrap()
                .light
        );
        assert!(
            parse(&format!("theme_type = \"light\"\n{colours}"), false)
                .unwrap()
                .light
        );
        assert!(
            !parse(&format!("mode = \"dark\"\n{colours}"), true)
                .unwrap()
                .light,
            "what the file says wins over light.mode"
        );
        let pale = "background = \"#faf4ed\"\nforeground = \"#575279\"\n";
        assert!(parse(pale, false).unwrap().light, "a pale ground");
    }

    #[test]
    fn a_file_with_no_ground_or_no_ink_is_no_theme() {
        assert_eq!(parse("", false), None);
        assert_eq!(parse("accent = \"#7aa2f7\"\n", false), None);
        assert_eq!(parse("background = \"#1a1b26\"\n", false), None);
        assert_eq!(
            parse("background = \"night\"\nforeground = \"#fff\"\n", false),
            None,
            "neither is a colour"
        );
    }

    #[test]
    fn section_headers_comments_and_other_keys_are_skipped() {
        let text = "# colours\n[colors]\nbackground = \"#000000\" # black\n\
                    foreground=\"#FFFFFF\"\nhyprland_active_border = \"rgba(1,2,3,0.5) 45deg\"\n";
        let theme = parse(text, false).unwrap();
        assert_eq!(theme.background, [0; 3]);
        assert_eq!(theme.foreground, [255; 3]);
    }

    #[test]
    fn a_colour_is_six_digits_or_eight_with_its_alpha_dropped() {
        assert_eq!(rgb("#7AA2f7"), Some([0x7a, 0xa2, 0xf7]));
        assert_eq!(rgb("7aa2f7"), Some([0x7a, 0xa2, 0xf7]));
        assert_eq!(rgb(" #7aa2f7cc "), Some([0x7a, 0xa2, 0xf7]));
        assert_eq!(rgb("#fff"), None);
        assert_eq!(rgb("rgba(1,2,3,1)"), None);
        assert_eq!(hex([0x7a, 0xa2, 0xf7]), "#7aa2f7");
    }

    #[test]
    fn a_name_is_shown_as_omarchy_shows_it() {
        let mut theme = parse(TOKYO_NIGHT, false).unwrap();
        theme.name = "tokyo-night".into();
        assert_eq!(theme.title(), "Tokyo Night");
        theme.name = "nord".into();
        assert_eq!(theme.title(), "Nord");
        theme.name.clear();
        assert_eq!(theme.title(), "");
    }

    #[test]
    fn the_theme_in_omarchys_folder_is_read_with_its_name() {
        let current = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../target/test-scratch")
            .join(format!("theme-{}", std::process::id()));
        let _ = fs::remove_dir_all(&current);
        assert_eq!(read(&current), None, "no Omarchy here");
        fs::create_dir_all(current.join("theme")).unwrap();
        fs::write(current.join("theme/colors.toml"), TOKYO_NIGHT).unwrap();
        assert_eq!(read(&current).unwrap().name, "", "a theme with no name");
        fs::write(current.join("theme.name"), "tokyo-night\n").unwrap();
        let theme = read(&current).unwrap();
        assert_eq!(theme.name, "tokyo-night");
        assert_eq!(theme.accent, [0x7a, 0xa2, 0xf7]);
        fs::remove_dir_all(&current).unwrap();
    }

    #[test]
    fn neutrals_run_from_the_themes_ground_to_its_ink() {
        let palette = Palette::of(&parse(TOKYO_NIGHT, false).unwrap());
        assert_eq!(palette.tone(OWN_GROUND), palette.ground);
        assert_eq!(
            palette.tone([0; 3]),
            palette.ground,
            "darker than the ground"
        );
        assert_eq!(palette.tone([255; 3]), palette.ink);
        assert_eq!(palette.ink, [0xa9, 0xb1, 0xd6], "the theme's foreground");
        // Each step up Slipstream's own text is a step up the theme's.
        let steps = [
            0x10131a,
            0x7d8697,
            0x8f98a8,
            0xaab2c0,
            0xe7ebf1,
            0xf2f4f8u32,
        ];
        let contrasts: Vec<f64> = steps
            .iter()
            .map(|own| {
                let [_, r, g, b] = own.to_be_bytes();
                contrast(palette.tone([r, g, b]), palette.ground)
            })
            .collect();
        assert!(contrasts.is_sorted(), "{contrasts:?}");
    }

    #[test]
    fn a_neutral_keeps_its_share_of_the_contrast() {
        let palette = Palette::of(&parse(TOKYO_NIGHT, false).unwrap());
        let hint = [0x8f, 0x98, 0xa8];
        let own = contrast(hint, OWN_GROUND).ln() / contrast(OWN_INK, OWN_GROUND).ln();
        let themed = contrast(palette.tone(hint), palette.ground).ln()
            / contrast(palette.ink, palette.ground).ln();
        assert!((own - themed).abs() < 0.03, "{own} against {themed}");
        // Quiet text stays readable in a theme whose own text is: 8.5:1 here, against
        // Slipstream's 15:1.
        assert!(contrast(palette.tone(hint), palette.ground) >= 4.0);
    }

    #[test]
    fn a_light_theme_puts_dark_ink_on_a_light_ground() {
        let text = "mode = \"light\"\nbackground = \"#eff1f5\"\nforeground = \"#4c4f69\"\n\
                    accent = \"#1e66f5\"\nyellow = \"#df8e1d\"\n";
        let palette = Palette::of(&parse(text, false).unwrap());
        assert!(palette.light);
        assert!(luminance(palette.tone([255; 3])) < luminance(palette.tone(OWN_GROUND)));
        // On a pale colour its dark ink reads, and on a deep one its pale ground.
        assert_eq!(palette.on([0xdf, 0x8e, 0x1d]), palette.ink);
        assert_eq!(palette.on([0x1e, 0x40, 0xa5]), palette.ground);
        // Light still falls on dark: the rain's ground is the ink, darker, and its ink the
        // theme's ground.
        assert!(luminance(palette.night_ground) < luminance(palette.ink));
        assert_eq!(palette.night_tone(OWN_GROUND), palette.night_ground);
        assert_eq!(palette.night_tone([255; 3]), palette.ground);
        // A dark theme has one ground for both.
        let dark = Palette::of(&parse(TOKYO_NIGHT, false).unwrap());
        assert_eq!(dark.night_ground, dark.ground);
        assert_eq!(
            dark.night_tone([0x8f, 0x98, 0xa8]),
            dark.tone([0x8f, 0x98, 0xa8])
        );
    }

    #[test]
    fn the_wallpapers_colours_land_on_the_themes() {
        let theme = parse(TOKYO_NIGHT, false).unwrap();
        let palette = Palette::of(&theme);
        assert_eq!(palette.lit(OWN_BACKDROP), palette.ground);
        assert_eq!(
            palette.lit([0; 3]),
            palette.ground,
            "darker than the backdrop"
        );
        // The wallpaper's own cyan, amber and mint are the theme's accent, warm colour and green.
        assert_eq!(palette.lit([51, 204, 255]), theme.accent);
        assert_eq!(palette.lit([255, 181, 71]), palette.warm);
        // The mint is a little short of full brightness, so it is a little short of the green.
        assert!(distance(palette.lit([60, 240, 191]), theme.green) < 16.0);
        assert_eq!(palette.lit([255; 3]), palette.ink);
        // Half way from the backdrop to a colour is half way from the ground to the theme's.
        let half = mix(OWN_BACKDROP, [51, 204, 255], 0.5);
        let wanted = mix(palette.ground, theme.accent, 0.5);
        assert!(
            distance(palette.lit(half), wanted) < 4.0,
            "{:?}",
            palette.lit(half)
        );
    }

    #[test]
    fn the_wallpaper_is_ink_on_paper_under_a_light_theme() {
        let text = "mode = \"light\"\nbackground = \"#eff1f5\"\nforeground = \"#4c4f69\"\n\
                    accent = \"#1e66f5\"\nyellow = \"#df8e1d\"\n";
        let palette = Palette::of(&parse(text, false).unwrap());
        let (dim, lit) = (palette.lit([40, 44, 52]), palette.lit([237, 250, 255]));
        assert!(luminance(lit) < luminance(dim), "more light is more ink");
        assert!(luminance(dim) < luminance(palette.ground));
    }

    #[test]
    fn the_ring_and_the_highlight_are_the_two_colours_given() {
        let theme = parse(TOKYO_NIGHT, false).unwrap();
        let palette = Palette::of(&theme);
        assert_eq!((palette.accent, palette.warm), (theme.accent, theme.yellow));
        let palette = Palette::with(&theme, theme.green, theme.magenta);
        assert_eq!((palette.accent, palette.warm), (theme.green, theme.magenta));
        // The wallpaper's cyan and amber go with them.
        assert_eq!(palette.lit([51, 204, 255]), theme.green);
        assert_eq!(palette.lit([255, 181, 71]), theme.magenta);
    }

    #[test]
    fn a_colour_is_picked_by_its_name_in_the_theme_or_as_a_number() {
        let theme = parse(TOKYO_NIGHT, false).unwrap();
        assert_eq!(pick("accent", Some(&theme)), Some(theme.accent));
        assert_eq!(pick(" Yellow ", Some(&theme)), Some(theme.yellow));
        assert_eq!(pick("#102030", Some(&theme)), Some([0x10, 0x20, 0x30]));
        assert_eq!(pick("mauve", Some(&theme)), None);
        assert_eq!(pick("accent", None), Some(OWN_CYAN));
        assert_eq!(pick("yellow", None), Some(OWN_AMBER));
        for (name, _) in PICKS {
            assert!(theme.colour(name).is_some(), "{name}");
        }
    }

    #[test]
    fn the_icon_theme_is_read_from_beside_the_colours() {
        let current = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../target/test-scratch")
            .join(format!("theme-icons-{}", std::process::id()));
        let _ = fs::remove_dir_all(&current);
        fs::create_dir_all(current.join("theme")).unwrap();
        fs::write(current.join("theme/colors.toml"), TOKYO_NIGHT).unwrap();
        assert_eq!(read(&current).unwrap().icons, None);
        fs::write(current.join("theme/icons.theme"), "Yaru-blue\n").unwrap();
        assert_eq!(read(&current).unwrap().icons.as_deref(), Some("Yaru-blue"));
        fs::write(current.join("theme/icons.theme"), "../elsewhere\n").unwrap();
        assert_eq!(read(&current).unwrap().icons, None, "a name, not a path");
        fs::remove_dir_all(&current).unwrap();
    }
}
