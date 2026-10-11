//! Slipstream's settings file, `~/.config/slipstream/settings.toml`. The compositor reads it at
//! startup and again whenever it changes; `slipstream-settings` and quick settings write it. Every
//! setting's name, default and meaning lives here, so they never disagree.
//!
//! ```toml
//! [appearance]
//! colour-scheme = "dark"
//! follow-omarchy-theme = true
//!
//! [wallpaper]
//! fade-after-secs = 120
//! variations = ["slipstream"]
//! change-every-mins = 10
//!
//! [motion]
//! reduced = false
//! effects = "slipstream"
//! close-into-rain = true
//!
//! [display]
//! night-light = false
//! night-light-schedule = "off"
//! night-light-from = "21:00"
//! night-light-to = "07:00"
//! screen-off-mins = 15
//! variable-refresh = "games"
//!
//! [notifications]
//! do-not-disturb = false
//! wait-while-typing = true
//! longest-wait-mins = 15
//! sounds = true
//! sound-theme = "ocean"
//! ding = "app"
//!
//! [sound]
//! volume-blip = true
//! click = "tap"
//! volume-step = 5
//! muffle-bullet-time = true
//!
//! [clipboard]
//! history = true
//!
//! [keys]
//! style = "windows"
//!
//! [mouse]
//! speed = 0.0
//! natural-scrolling = false
//! acceleration = "adaptive"
//!
//! [touchpad]
//! tap-to-click = true
//! natural-scrolling = true
//! speed = 0.0
//! disable-while-typing = true
//! scrolling = "two-finger"
//!
//! [keyboard]
//! repeat-delay-ms = 200
//! repeat-rate = 25
//!
//! [[keyboard.layouts]]
//! layout = "gb"
//! variant = ""
//!
//! [lock]
//! after-idle-mins = 0
//! before-sleep = true
//!
//! [plugins]
//! on = ["disk-space"]
//! glance = "disk-space"
//!
//! [session]
//! remember = false
//! reopen-without-asking = false
//!
//! [borders]
//! selected-tile = "accent"
//! bullet-time = "yellow"
//! accent = "yellow"
//!
//! [privacy]
//! screen-capture-allowed = ["/usr/bin/wf-recorder"]
//!
//! [[workspaces.list]]
//! id = 1
//! name = "Mail"
//!
//! [[workspaces.list]]
//! id = 2
//! name = ""
//! ```
//!
//! Missing keys take their defaults and unknown ones are ignored, so an older compositor still
//! reads a newer file.

pub mod battery;
pub mod card;
pub mod command;
mod display;
mod file;
mod input;
mod pages;
pub mod plugins;
pub mod screens;
pub mod shortcuts;
pub mod sounds;
pub mod sun;
pub mod theme;
mod run_or_raise;
mod wallpaper;
pub mod weight;
mod workspaces;

use serde::{Deserialize, Serialize};

pub use display::{
    Align, Display, NIGHT_FADE_MINS, NightSchedule, Position, SCALE_RANGE, ScreenPlace,
    VariableRefresh,
};
pub use file::{
    FILE_NAME, civil, move_aside, path, read, replace, replace_private, write, xdg_home,
};
pub use input::{
    Acceleration, Keyboard, KeyboardLayout, LAYOUT_LISTS, LayoutChoice, Mouse, Scrolling, Touchpad,
    layout_catalogue, layout_label, options_switch_layouts, system_layouts,
    system_layouts_from_env,
};
pub use pages::{SETTINGS_PAGES, SettingsPage};
pub use run_or_raise::{RaiseApp, RunOrRaise};
pub use wallpaper::{VARIATIONS, Variation, Wallpaper, current_id};
pub use workspaces::{MAX_WORKSPACES, WorkspaceEntry, Workspaces, unused_id, workspace_label};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Settings {
    pub appearance: Appearance,
    pub wallpaper: Wallpaper,
    pub motion: Motion,
    pub display: Display,
    pub notifications: Notifications,
    pub borders: Borders,
    pub session: Session,
    pub lock: Lock,
    pub sound: Sound,
    pub clipboard: Clipboard,
    pub privacy: Privacy,
    /// Which plugins are on, and the one whose headline is on the chip.
    pub plugins: plugins::Plugins,
    pub workspaces: Workspaces,
    pub keys: Keys,
    pub mouse: Mouse,
    pub touchpad: Touchpad,
    pub keyboard: Keyboard,
    pub run_or_raise: RunOrRaise,
}

/// The keyboard shortcuts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Keys {
    /// Whose shortcuts the desktop's keys follow.
    pub style: KeyStyle,
    /// Shortcuts of the person's own, each opening an app. One that clashes with a key of the
    /// style in force stays here and does nothing until the style changes back, so changing
    /// style never loses any.
    #[serde(rename = "app", skip_serializing_if = "Vec::is_empty")]
    pub apps: Vec<AppKey>,
    /// Shortcuts of the person's own, each doing one of a plugin's actions. Kept, as an app's
    /// are, while they clash or the plugin is off, and doing nothing meanwhile.
    #[serde(rename = "plugin", skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<PluginKey>,
}

/// One shortcut that does a plugin's action, as its manifest lists it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PluginKey {
    /// The keys, as the XDG shortcuts specification writes them: `LOGO+t`.
    pub keys: String,
    /// The plugin's id, and the action's.
    pub plugin: String,
    pub action: String,
}

/// One shortcut that opens an app.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AppKey {
    /// The keys, as the XDG shortcuts specification writes them: `LOGO+g`.
    pub keys: String,
    /// The app's desktop file ID, without `.desktop`: `org.example.Files`.
    pub app: String,
}

impl Keys {
    /// Adds a shortcut, in place of any other on the same keys.
    pub fn add_app_key(&mut self, keys: &str, app: &str) {
        self.remove_app_key(keys);
        self.apps.push(AppKey {
            keys: keys.to_string(),
            app: app.to_string(),
        });
    }

    /// Takes away the shortcut on these keys.
    pub fn remove_app_key(&mut self, keys: &str) {
        self.apps.retain(|key| key.keys != keys);
    }

    /// Gives a plugin's action these keys, in place of any keys it had and of any other plugin
    /// action on them.
    pub fn add_plugin_key(&mut self, keys: &str, plugin: &str, action: &str) {
        self.remove_plugin_key(plugin, action);
        self.plugins.retain(|key| key.keys != keys);
        self.plugins.push(PluginKey {
            keys: keys.to_string(),
            plugin: plugin.to_string(),
            action: action.to_string(),
        });
    }

    /// Takes away the keys of a plugin's action.
    pub fn remove_plugin_key(&mut self, plugin: &str, action: &str) {
        self.plugins
            .retain(|key| key.plugin != plugin || key.action != action);
    }

    /// The keys a plugin's action has, if any.
    pub fn plugin_key(&self, plugin: &str, action: &str) -> Option<&str> {
        self.plugins
            .iter()
            .find(|key| key.plugin == plugin && key.action == action)
            .map(|key| key.keys.as_str())
    }
}

/// Which desktop's habits the shortcuts follow. The actions are the same in each; the keys
/// differ, and a style leaves out a key its desktop has no habit for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyStyle {
    #[default]
    Windows,
    Mac,
    Omarchy,
}

impl KeyStyle {
    pub const ALL: [KeyStyle; 3] = [KeyStyle::Windows, KeyStyle::Mac, KeyStyle::Omarchy];

    /// Its name in the settings file, which `slipstream --list-keys` takes too.
    pub fn id(self) -> &'static str {
        match self {
            KeyStyle::Windows => "windows",
            KeyStyle::Mac => "mac",
            KeyStyle::Omarchy => "omarchy",
        }
    }

    /// The style called `id` in the settings file.
    pub fn from_id(id: &str) -> Option<KeyStyle> {
        KeyStyle::ALL.into_iter().find(|style| style.id() == id)
    }

    /// What it's called in the Settings app.
    pub fn label(self) -> &'static str {
        match self {
            KeyStyle::Windows => "Windows style",
            KeyStyle::Mac => "Mac style",
            KeyStyle::Omarchy => "Omarchy style",
        }
    }

    /// What the style is like, in a line for the Settings app.
    pub fn summary(self) -> &'static str {
        match self {
            KeyStyle::Windows => {
                "Tap Super to open apps, as the Windows key opens Start. Alt+Tab, Alt+F4, \
                 Super+L, Super+D and Super+Shift+S do what they do on Windows."
            }
            KeyStyle::Mac => {
                "Super is ⌘: Super+Space opens apps, and Super+Tab, Super+Q, Super+W and \
                 Super+Shift+3 and 4 do what ⌘ does on a Mac. Apps keep their own shortcuts, so \
                 copying is still Ctrl+C."
            }
            KeyStyle::Omarchy => {
                "Omarchy's keys: Super+Space opens apps, Super+W closes, Super+T floats, \
                 Super+C and Super+V copy and paste in any app, and Super+K lists every key."
            }
        }
    }
}

/// What apps are told to colour themselves, and where Slipstream's own colours come from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Appearance {
    /// Dark or light, for every app that asks the desktop which to be. A followed Omarchy theme
    /// says which itself.
    pub colour_scheme: ColourScheme,
    /// Take Slipstream's colours from the theme Omarchy is showing (`theme`), and change with it.
    /// Where there is no Omarchy, or with this off, Slipstream is drawn in its own.
    pub follow_omarchy_theme: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            colour_scheme: ColourScheme::default(),
            follow_omarchy_theme: true,
        }
    }
}

impl Appearance {
    /// The Omarchy theme to draw the desktop from: the one Omarchy is showing, when it is
    /// followed and there is one.
    pub fn theme(&self) -> Option<theme::Theme> {
        self.follow_omarchy_theme.then(theme::current).flatten()
    }

    /// What apps are asked to be: the followed theme's own dark or light, else the one chosen.
    pub fn scheme_with(&self, theme: Option<&theme::Theme>) -> ColourScheme {
        match theme {
            Some(theme) if theme.light => ColourScheme::Light,
            Some(_) => ColourScheme::Dark,
            None => self.colour_scheme,
        }
    }
}

/// Dark or light.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColourScheme {
    #[default]
    Dark,
    Light,
}

impl ColourScheme {
    pub const ALL: [ColourScheme; 2] = [ColourScheme::Dark, ColourScheme::Light];

    /// What it's called in the Settings app.
    pub fn label(self) -> &'static str {
        match self {
            ColourScheme::Dark => "Dark",
            ColourScheme::Light => "Light",
        }
    }

    /// The number the appearance portal's `color-scheme` key uses. Its third value, 0 for no
    /// preference, is never sent: Slipstream always has one.
    pub fn portal_value(self) -> u32 {
        match self {
            ColourScheme::Dark => 1,
            ColourScheme::Light => 2,
        }
    }

    /// The same preference as GNOME's `color-scheme` setting spells it, which is what GTK reads
    /// when it doesn't know the portal's own key.
    pub fn gnome_value(self) -> &'static str {
        match self {
            ColourScheme::Dark => "prefer-dark",
            ColourScheme::Light => "prefer-light",
        }
    }

    /// A stock GTK theme of that colour, for apps that follow a theme's name rather than a
    /// preference. Adwaita ships with GTK itself, so it is on every machine GTK apps run on.
    pub fn gtk_theme(self) -> &'static str {
        match self {
            ColourScheme::Dark => "Adwaita-dark",
            ColourScheme::Light => "Adwaita",
        }
    }
}

/// How windows and effects move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Motion {
    /// Reduced motion: moves jump, and every effect becomes a short fade.
    pub reduced: bool,
    /// What minimised apps turn into, and how a closed window falls away.
    pub effects: Effects,
    /// A closed window falls away in the chosen effects. Off, it fades. Reduced motion turns it
    /// off too.
    pub close_into_rain: bool,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            reduced: false,
            effects: Effects::default(),
            close_into_rain: true,
        }
    }
}

/// The look of the minimised apps' streams and of closing windows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Effects {
    /// Streaks of light falling at different depths, and a closed window dropping away with a
    /// wake of light.
    #[default]
    Slipstream,
    /// Code rain: glyphs falling in columns, and a closed window read out into them.
    Matrix,
}

/// Notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Notifications {
    /// Do not disturb: notifications still reach the notification centre, but don't pop up.
    pub do_not_disturb: bool,
    /// Pop-ups wait while you're typing and show at the next pause, together. Critical ones
    /// don't wait.
    pub wait_while_typing: bool,
    /// The longest a pop-up waits for a pause, in minutes. Urgent messages tend to reach a phone
    /// first, so a quarter of an hour of unbroken typing is left alone. At least 1.
    pub longest_wait_mins: u64,
    /// Plays the sound a notification asks for, by file or by name from the sound theme. Not
    /// while you're looking at the window it came from, and under do not disturb only for
    /// critical ones.
    pub sounds: bool,
    /// The freedesktop sound theme sound names are looked up in, falling back through the
    /// themes it inherits from to `freedesktop`.
    pub sound_theme: String,
    /// What a notification sounds like: `app` for the sound each app asks for, or one sound for
    /// them all, a preset from `sounds::DINGS` by its id or a file by its full path.
    pub ding: String,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            do_not_disturb: false,
            wait_while_typing: true,
            longest_wait_mins: 15,
            sounds: true,
            sound_theme: "ocean".into(),
            ding: sounds::APPS_OWN.into(),
        }
    }
}

impl Notifications {
    /// `longest_wait_mins` in seconds, never under a minute.
    pub fn longest_wait_secs(&self) -> f64 {
        self.longest_wait_mins.max(1) as f64 * 60.0
    }
}

/// The sounds Slipstream makes itself, as against the ones apps make.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Sound {
    /// A short click at the new level whenever the volume changes, from the keys or from a volume
    /// slider, as macOS and Windows do, so the setting can be heard as well as seen.
    /// Never on mute, never on brightness.
    pub volume_blip: bool,
    /// How far each press of the volume keys moves the volume, in percent.
    pub volume_step: u8,
    /// Muffle what's playing while bullet time is open: quieter, with the highs rolled off.
    pub muffle_bullet_time: bool,
    /// What the volume's click sounds like: a preset from `sounds::CLICKS` by its id, or a file
    /// by its full path.
    pub click: String,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            volume_blip: true,
            volume_step: 5,
            muffle_bullet_time: true,
            click: "tap".into(),
        }
    }
}

impl Sound {
    /// `volume_step`, from 1% to 25%.
    pub fn volume_step(&self) -> u8 {
        self.volume_step.clamp(1, 25)
    }
}

/// What's open when the session ends.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Session {
    /// Record which apps were open, on which workspace and in which tiling place, so the layout
    /// can be put back. Off by default: with it off nothing is written to disk at all. The way
    /// out's card carries the same switch, and turning it on there turns it on here.
    pub remember: bool,
    /// Put the layout back at the next login without offering first. Only means anything with
    /// `remember` on, since with it off there is nothing recorded to put back.
    pub reopen_without_asking: bool,
}

/// What programs are allowed to do without being asked again.
///
/// Only the compositor's own card adds to these lists, when someone answers "Always allow". The
/// Settings app can take an entry away; it can never put one in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Privacy {
    /// Programs allowed to record the screen without asking, by the full path of the executable
    /// that asked, as `/proc/<pid>/exe` gives it. Empty to begin with: every program asks the
    /// first time. Take an entry away to be asked again.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub screen_capture_allowed: Vec<String>,
}

impl Privacy {
    /// Whether this executable has been allowed to record the screen already.
    pub fn allows_capture(&self, exe: &str) -> bool {
        self.screen_capture_allowed.iter().any(|it| it == exe)
    }

    /// Remembers this executable. Saying yes twice doesn't list it twice.
    pub fn allow_capture(&mut self, exe: String) {
        if !self.allows_capture(&exe) {
            self.screen_capture_allowed.push(exe);
        }
    }

    /// Forgets it, so it is asked again the next time.
    pub fn forget_capture(&mut self, exe: &str) {
        self.screen_capture_allowed.retain(|it| it != exe);
    }
}

/// The clipboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Clipboard {
    /// Keep the last things copied, text and pictures, for Super+V to paste again. In memory
    /// only, forgotten at logout, and never what a password manager marks as secret. Turning it
    /// off forgets everything kept.
    pub history: bool,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self { history: true }
    }
}

/// The lock screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Lock {
    /// Lock after this many minutes with no keyboard or mouse input; 0 never does. Off by
    /// default: only Super+L, quick settings, `loginctl lock-session` and sleep lock.
    pub after_idle_mins: u64,
    /// Lock just before the machine sleeps, so it wakes up locked.
    pub before_sleep: bool,
}

impl Default for Lock {
    fn default() -> Self {
        Self {
            after_idle_mins: 0,
            before_sleep: true,
        }
    }
}

/// Slipstream's three colours, which Settings calls primary, secondary and tertiary; the keys keep
/// the names of what they first coloured. Each is one of the theme's colours by name
/// (`theme::PICKS`: `accent`, `yellow`, `green` and so on) or `#rrggbb`. A value that is neither
/// falls back to the default, so a mistyped hand edit leaves the desktop usable rather than
/// colourless. Without a theme the names are Slipstream's own colours of those names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Borders {
    /// The primary colour, for where the keys are aimed: the focused window's ring and the
    /// keyboard's ring in the panels, the workspace in front, a battery charging, plugins on.
    pub selected_tile: String,
    /// The secondary colour: bullet time's rings and frames, and whatever wants attention, from
    /// the notification count to Caps Lock and a low battery.
    pub bullet_time: String,
    /// The tertiary colour, for what's on or chosen: switches and tiles, sliders, the default
    /// button, today in the calendar, text cursors and toasts. Files from before it had this
    /// name call it `highlight`.
    #[serde(alias = "highlight")]
    pub accent: String,
}

/// The theme's accent: Slipstream's own is its ice cyan. It sits 73° of hue from the code rain,
/// so the ring reads against it rather than dissolving into it.
pub const SELECTED_TILE: &str = "accent";
/// The theme's yellow: Slipstream's own is its amber, near-opposite the ring's cyan: cold means
/// the live desktop, warm the slow world.
pub const BULLET_TIME: &str = "yellow";
/// The same yellow: what's on inside a panel is warm against the colour of where the keys are.
pub const ACCENT: &str = "yellow";
/// What `selected-tile` held before colours went by name: the same ice cyan, as a number. A
/// file that still says so means the accent, not that cyan whatever the theme.
const SELECTED_TILE_WAS: &str = "#42d3ff";
/// What `bullet-time` and `accent` held before colours went by name: Slipstream's amber.
const AMBER_WAS: &str = "#ffb547";

impl Default for Borders {
    fn default() -> Self {
        Self {
            selected_tile: SELECTED_TILE.to_string(),
            bullet_time: BULLET_TIME.to_string(),
            accent: ACCENT.to_string(),
        }
    }
}

impl Borders {
    /// `value` in the followed theme or in Slipstream's own colours: `default` where `value` is
    /// what the setting held as a number before colours went by name, or isn't a colour at all.
    fn resolve(
        value: &str,
        default: &str,
        was: &str,
        theme: Option<&theme::Theme>,
        own: theme::Rgb,
    ) -> theme::Rgb {
        let chosen = match theme::rgb(value) == theme::rgb(was) {
            true => default,
            false => value,
        };
        theme::pick(chosen, theme)
            .or_else(|| theme::pick(default, theme))
            .unwrap_or(own)
    }

    /// The primary colour, the ring's, in the followed theme or in Slipstream's own colours.
    pub fn ring(&self, theme: Option<&theme::Theme>) -> theme::Rgb {
        let tile = &self.selected_tile;
        Self::resolve(tile, SELECTED_TILE, SELECTED_TILE_WAS, theme, theme::OWN_CYAN)
    }

    /// The secondary colour, bullet time's, in the followed theme or in Slipstream's own colours.
    pub fn secondary(&self, theme: Option<&theme::Theme>) -> theme::Rgb {
        let bullet = &self.bullet_time;
        Self::resolve(bullet, BULLET_TIME, AMBER_WAS, theme, theme::OWN_AMBER)
    }

    /// The tertiary colour, the accent, in the followed theme or in Slipstream's own colours.
    pub fn highlight_colour(&self, theme: Option<&theme::Theme>) -> theme::Rgb {
        Self::resolve(&self.accent, ACCENT, AMBER_WAS, theme, theme::OWN_AMBER)
    }

    /// The primary colour as the renderer takes it, each channel from 0 to 1.
    pub fn selected_tile_rgb(&self, theme: Option<&theme::Theme>) -> [f32; 3] {
        unit(self.ring(theme))
    }

    /// The secondary colour as the renderer takes it.
    pub fn bullet_time_rgb(&self, theme: Option<&theme::Theme>) -> [f32; 3] {
        unit(self.secondary(theme))
    }

    /// The tertiary colour as the renderer takes it.
    pub fn accent_rgb(&self, theme: Option<&theme::Theme>) -> [f32; 3] {
        unit(self.highlight_colour(theme))
    }
}

fn unit(rgb: theme::Rgb) -> [f32; 3] {
    rgb.map(|channel| channel as f32 / 255.0)
}

impl Settings {
    /// Where Slipstream's colours land in `theme`, with the primary and tertiary colours as
    /// chosen here: `None` without a theme, when Slipstream is drawn in its own.
    pub fn palette(&self, theme: Option<&theme::Theme>) -> Option<theme::Palette> {
        theme.map(|theme| {
            theme::Palette::with(
                theme,
                self.borders.ring(Some(theme)),
                self.borders.highlight_colour(Some(theme)),
            )
        })
    }
}

/// `#rrggbb` or `rrggbb`, either case, as three channels from 0 to 1. Anything else is `None`.
pub fn rgb(text: &str) -> Option<[f32; 3]> {
    let digits = text.trim().strip_prefix('#').unwrap_or(text.trim());
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize| {
        u8::from_str_radix(&digits[at..at + 2], 16)
            .ok()
            .map(|value| value as f32 / 255.0)
    };
    Some([channel(0)?, channel(2)?, channel(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{HEADER, parse};

    #[test]
    fn run_or_raise_apps_and_home_screens_read_and_write_back_the_same() {
        let text = r#"
            [[workspaces.list]]
            id = 1
            name = " Chrome "
            screen = " DP-1 "

            [run-or-raise]
            raise = "ctrl+super+alt"

            [[run-or-raise.app]]
            workspace = "Chrome"
            key = "c"
            app = "browser"

            [[run-or-raise.app]]
            workspace = "Lists"
            key = "l"
            class = "chrome-trello.com__b_x-Default"
            command = "omarchy-launch-webapp https://trello.com/b/x"
        "#;
        let settings = parse(text).unwrap();
        let entries = settings.workspaces.entries();
        assert_eq!(entries[0].name, "Chrome");
        assert_eq!(entries[0].screen, "DP-1");
        assert_eq!(settings.run_or_raise.app.len(), 2);
        assert_eq!(settings.run_or_raise.app[0].app, "browser");
        assert_eq!(settings.run_or_raise.app[1].key, "l");
        let again = parse(&toml::to_string(&settings).unwrap()).unwrap();
        assert_eq!(again, settings);
    }

    #[test]
    fn with_no_run_or_raise_section_the_chord_is_hyper_and_there_are_no_apps() {
        let settings = parse("").unwrap();
        assert_eq!(settings.run_or_raise.raise, "ctrl+super+alt");
        assert!(settings.run_or_raise.app.is_empty());
        assert!(
            !toml::to_string(&settings.workspaces)
                .unwrap()
                .contains("screen")
        );
    }

    #[test]
    fn an_empty_file_means_every_default() {
        let settings = parse("").unwrap();
        assert_eq!(settings, Settings::default());
        assert_eq!(settings.wallpaper.fade_after_secs, 120);
        assert_eq!(settings.wallpaper.variations, ["slipstream"]);
        assert_eq!(settings.wallpaper.change_every_mins, 10);
        assert!(!settings.motion.reduced);
        assert!(settings.motion.close_into_rain);
        assert_eq!(settings.motion.effects, Effects::Slipstream);
        assert!(!settings.display.night_light);
        assert!(!settings.notifications.do_not_disturb);
        assert!(settings.notifications.wait_while_typing);
        assert_eq!(settings.notifications.longest_wait_mins, 15);
        assert!(settings.notifications.sounds);
        assert_eq!(settings.notifications.sound_theme, "ocean");
        assert!(!settings.session.remember);
        assert!(!settings.session.reopen_without_asking);
        assert_eq!(
            settings.borders.selected_tile_rgb(None).map(round3),
            [0.259, 0.827, 1.0]
        );
        assert_eq!(
            settings.borders.bullet_time_rgb(None).map(round3),
            [1.0, 0.71, 0.278]
        );
        assert_eq!(settings.borders.highlight_colour(None), theme::OWN_AMBER);
    }

    #[test]
    fn effects_are_read_by_name() {
        let settings = parse("[motion]\neffects = \"slipstream\"\n").unwrap();
        assert_eq!(settings.motion.effects, Effects::Slipstream);
        assert!(
            settings.motion.close_into_rain,
            "the rest keep their defaults"
        );
    }

    #[test]
    fn lock_defaults() {
        let settings = parse("").unwrap();
        assert_eq!(settings.lock.after_idle_mins, 0, "nothing locks by itself");
        assert_eq!(settings.display.screen_off_mins, 15);
        assert!(settings.lock.before_sleep);
        let set = parse("[lock]\nafter-idle-mins = 5\nbefore-sleep = false\n").unwrap();
        assert_eq!(
            set.lock,
            Lock {
                after_idle_mins: 5,
                before_sleep: false
            }
        );
    }

    fn round3(channel: f32) -> f32 {
        (channel * 1000.0).round() / 1000.0
    }

    #[test]
    fn a_colour_is_read_with_or_without_its_hash_in_either_case() {
        assert_eq!(rgb("#000000"), Some([0.0, 0.0, 0.0]));
        assert_eq!(rgb("FFFFFF"), Some([1.0, 1.0, 1.0]));
        assert_eq!(rgb(" #ffb547 "), rgb("#FFB547"));
    }

    #[test]
    fn a_colour_that_is_not_a_colour_falls_back_to_the_default() {
        for text in ["", "#fff", "mint", "#12345g", "#1234567", "rgb(1,2,3)"] {
            assert_eq!(rgb(text), None, "{text}");
        }
        let borders = Borders {
            selected_tile: "mint".into(),
            bullet_time: "#12345g".into(),
            accent: "".into(),
        };
        assert_eq!(borders.ring(None), theme::OWN_CYAN);
        assert_eq!(borders.secondary(None), theme::OWN_AMBER);
        assert_eq!(borders.highlight_colour(None), theme::OWN_AMBER);
    }

    #[test]
    fn missing_keys_keep_their_defaults_and_unknown_ones_are_ignored() {
        let settings =
            parse("[motion]\nreduced = true\nfuture-setting = 3\n\n[portal]\nzones = 4\n").unwrap();
        assert!(settings.motion.reduced);
        assert_eq!(settings.wallpaper, Wallpaper::default());
    }

    #[test]
    fn a_followed_theme_says_dark_or_light() {
        let text = "mode = \"light\"\nbackground = \"#eff1f5\"\nforeground = \"#4c4f69\"\n";
        let theme = theme::parse(text, false).unwrap();
        let appearance = Appearance::default();
        assert!(
            appearance.follow_omarchy_theme,
            "followed unless turned off"
        );
        assert_eq!(appearance.scheme_with(None), ColourScheme::Dark);
        assert_eq!(appearance.scheme_with(Some(&theme)), ColourScheme::Light);
    }

    #[test]
    fn the_ring_and_the_highlight_are_colours_of_the_theme_by_name() {
        let text = "background = \"#1a1b26\"\nforeground = \"#a9b1d6\"\naccent = \"#7aa2f7\"\n\
                    yellow = \"#e0af68\"\ngreen = \"#9ece6a\"\nmagenta = \"#ad8ee6\"\n";
        let theme = theme::parse(text, false).unwrap();
        let borders = Borders::default();
        assert_eq!(borders.ring(Some(&theme)), [0x7a, 0xa2, 0xf7]);
        assert_eq!(borders.highlight_colour(Some(&theme)), [0xe0, 0xaf, 0x68]);
        // Without a theme the same names are Slipstream's own colours.
        assert_eq!(borders.ring(None), theme::OWN_CYAN);
        assert_eq!(borders.highlight_colour(None), theme::OWN_AMBER);

        let chosen = Borders {
            selected_tile: "green".into(),
            bullet_time: "red".into(),
            accent: "Magenta".into(),
        };
        assert_eq!(chosen.ring(Some(&theme)), [0x9e, 0xce, 0x6a]);
        assert_eq!(chosen.highlight_colour(Some(&theme)), [0xad, 0x8e, 0xe6]);
        assert_eq!(chosen.ring(None), theme::OWN_MINT);
        assert_eq!(chosen.secondary(None), theme::OWN_RED);
        // A colour written as a number is that colour whatever the theme.
        let fixed = Borders {
            selected_tile: "#3CF0C0".into(),
            ..Borders::default()
        };
        assert_eq!(fixed.ring(Some(&theme)), [0x3c, 0xf0, 0xc0]);
        // The palette takes both, so everything drawn in them follows.
        let settings = Settings {
            borders: chosen,
            ..Settings::default()
        };
        let palette = settings.palette(Some(&theme)).unwrap();
        assert_eq!(palette.accent, [0x9e, 0xce, 0x6a]);
        assert_eq!(palette.warm, [0xad, 0x8e, 0xe6]);
        assert_eq!(settings.palette(None), None);
    }

    #[test]
    fn a_file_from_before_colours_had_names_still_follows_the_theme() {
        let settings =
            parse("[borders]\nselected-tile = \"#42D3FF\"\nbullet-time = \"#ffb547\"\n").unwrap();
        let text = "background = \"#1a1b26\"\nforeground = \"#a9b1d6\"\naccent = \"#7aa2f7\"\n\
                    yellow = \"#e0af68\"\n";
        let theme = theme::parse(text, false).unwrap();
        assert_eq!(settings.borders.ring(Some(&theme)), [0x7a, 0xa2, 0xf7]);
        assert_eq!(settings.borders.ring(None), theme::OWN_CYAN);
        assert_eq!(settings.borders.secondary(Some(&theme)), [0xe0, 0xaf, 0x68]);
    }

    #[test]
    fn a_file_that_called_the_accent_highlight_still_reads() {
        let settings = parse("[borders]\nhighlight = \"cyan\"\n").unwrap();
        assert_eq!(settings.borders.accent, "cyan");
    }

    #[test]
    fn what_is_written_reads_back() {
        let settings = Settings {
            appearance: Appearance {
                colour_scheme: ColourScheme::Light,
                follow_omarchy_theme: false,
            },
            session: Session {
                remember: true,
                reopen_without_asking: true,
            },
            lock: Lock {
                after_idle_mins: 10,
                before_sleep: false,
            },
            wallpaper: Wallpaper {
                fade_after_secs: 300,
                variations: vec!["vortex".into(), "slipstream".into()],
                change_every_mins: 5,
            },
            motion: Motion {
                reduced: true,
                ..Motion::default()
            },
            display: Display {
                night_light: true,
                night_light_schedule: NightSchedule::Custom,
                night_light_from: "22:15".into(),
                night_light_to: "06:45".into(),
                screen_off_mins: 30,
                variable_refresh: VariableRefresh::Always,
                screens: vec![ScreenPlace {
                    monitor: "Made Up MU27 0001".into(),
                    position: Position::Above,
                    align: Align::Centre,
                    mode: "2560x1440@144".into(),
                    scale: 1.25,
                }],
            },
            notifications: Notifications {
                do_not_disturb: true,
                wait_while_typing: false,
                longest_wait_mins: 30,
                sounds: false,
                sound_theme: "freedesktop".into(),
                ding: "/home/someone/ding.ogg".into(),
            },
            borders: Borders {
                selected_tile: "#b794ff".into(),
                bullet_time: "#ffcf5c".into(),
                accent: "orange".into(),
            },
            sound: Sound {
                volume_blip: false,
                volume_step: 2,
                muffle_bullet_time: false,
                click: "wood".into(),
            },
            clipboard: Clipboard { history: false },
            privacy: Privacy {
                screen_capture_allowed: vec!["/usr/bin/made-up-recorder".into()],
            },
            plugins: plugins::Plugins {
                on: vec!["made-up-timer".into()],
                glance: Some("made-up-timer".into()),
                fingerprints: [("made-up-timer".to_string(), "ab12".to_string())].into(),
                hidden: [("made-up-timer".to_string(), vec!["Breaks".to_string()])].into(),
            },
            workspaces: Workspaces {
                list: vec![
                    WorkspaceEntry {
                        id: 3,
                        name: "Mail".into(),
                        ..Default::default()
                    },
                    WorkspaceEntry {
                        id: 1,
                        name: String::new(),
                        ..Default::default()
                    },
                ],
            },
            keys: Keys {
                style: KeyStyle::Omarchy,
                apps: vec![AppKey {
                    keys: "LOGO+g".into(),
                    app: "org.example.Files".into(),
                }],
                plugins: vec![PluginKey {
                    keys: "LOGO+t".into(),
                    plugin: "timer".into(),
                    action: "start".into(),
                }],
            },
            mouse: Mouse {
                speed: 0.5,
                ..Mouse::default()
            },
            touchpad: Touchpad {
                tap_to_click: false,
                ..Touchpad::default()
            },
            keyboard: Keyboard {
                layouts: vec![KeyboardLayout::new("gb", "dvorak")],
                ..Keyboard::default()
            },
            run_or_raise: RunOrRaise::default(),
        };
        let text = toml::to_string(&settings).unwrap();
        assert!(text.contains("fade-after-secs = 300"), "{text}");
        assert!(text.contains(r#"style = "omarchy""#), "{text}");
        assert!(text.contains("[[keys.app]]"), "{text}");
        assert!(text.contains("tap-to-click = false"), "{text}");
        assert!(text.contains(r#"variant = "dvorak""#), "{text}");
        assert!(
            text.contains(r#"variations = ["vortex", "slipstream"]"#),
            "{text}"
        );
        assert!(text.contains("change-every-mins = 5"), "{text}");
        assert!(text.contains("night-light = true"), "{text}");
        assert!(text.contains(r#"mode = "2560x1440@144""#), "{text}");
        assert!(
            text.contains(r#"screen-capture-allowed = ["/usr/bin/made-up-recorder"]"#),
            "{text}"
        );
        assert!(
            text.contains(r#"night-light-schedule = "custom""#),
            "{text}"
        );
        assert!(text.contains("do-not-disturb = true"), "{text}");
        assert!(text.contains("history = false"), "{text}");
        assert!(text.contains(r##"selected-tile = "#b794ff""##), "{text}");
        assert!(text.contains(r#"glance = "made-up-timer""#), "{text}");
        assert_eq!(parse(&format!("{HEADER}{text}")).unwrap(), settings);
    }

    #[test]
    fn the_capture_allowlist_holds_each_program_once_and_can_forget_one() {
        let mut privacy = Privacy::default();
        assert!(!privacy.allows_capture("/usr/bin/grim"));
        privacy.allow_capture("/usr/bin/grim".into());
        privacy.allow_capture("/usr/bin/grim".into());
        assert_eq!(privacy.screen_capture_allowed, ["/usr/bin/grim"]);
        assert!(privacy.allows_capture("/usr/bin/grim"));
        // A different path is a different program, even with the same name.
        assert!(!privacy.allows_capture("/home/someone/bin/grim"));
        privacy.forget_capture("/usr/bin/grim");
        assert!(!privacy.allows_capture("/usr/bin/grim"));
        assert!(privacy.screen_capture_allowed.is_empty());
    }

    #[test]
    fn an_empty_allowlist_is_left_out_of_the_file() {
        let text = toml::to_string(&Settings::default()).unwrap();
        assert!(!text.contains("screen-capture-allowed"), "{text}");
    }

    #[test]
    fn a_settings_file_from_before_own_shortcuts_still_loads() {
        let settings = parse("[keys]\nstyle = \"mac\"\n").unwrap();
        assert_eq!(settings.keys.style, KeyStyle::Mac);
        assert!(settings.keys.apps.is_empty());
        let text = toml::to_string(&Settings::default()).unwrap();
        assert!(!text.contains("keys.app"), "{text}");
    }

    #[test]
    fn an_own_shortcut_replaces_another_on_the_same_keys() {
        let mut keys = Keys::default();
        keys.add_app_key("LOGO+g", "org.example.Files");
        keys.add_app_key("LOGO+h", "org.example.Notes");
        keys.add_app_key("LOGO+g", "org.example.Browser");
        let apps: Vec<_> = keys.apps.iter().map(|k| (&*k.keys, &*k.app)).collect();
        assert_eq!(
            apps,
            [
                ("LOGO+h", "org.example.Notes"),
                ("LOGO+g", "org.example.Browser")
            ]
        );
        keys.remove_app_key("LOGO+h");
        assert_eq!(keys.apps.len(), 1);
    }

    #[test]
    fn a_plugin_action_has_one_set_of_keys_and_keys_one_action() {
        let mut keys = Keys::default();
        keys.add_plugin_key("LOGO+t", "timer", "start");
        keys.add_plugin_key("LOGO+y", "timer", "start");
        keys.add_plugin_key("LOGO+u", "timer", "stop");
        keys.add_plugin_key("LOGO+u", "lamp", "toggle");
        assert_eq!(keys.plugin_key("timer", "start"), Some("LOGO+y"));
        assert_eq!(keys.plugin_key("timer", "stop"), None);
        assert_eq!(keys.plugin_key("lamp", "toggle"), Some("LOGO+u"));
        let text = toml::to_string(&Settings {
            keys: keys.clone(),
            ..Settings::default()
        })
        .unwrap();
        let read = parse(&text).unwrap();
        assert_eq!(read.keys, keys, "{text}");
        keys.remove_plugin_key("timer", "start");
        assert_eq!(keys.plugins.len(), 1);
    }
}
