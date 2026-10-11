//! What apps are told to colour themselves: Slipstream answers the settings portal itself.
//!
//! `org.freedesktop.impl.portal.Settings` is the one channel every toolkit uses to ask the
//! desktop whether to be dark or light. GTK, Qt through its xdgdesktopportal platform theme,
//! Firefox, Chromium and anything packaged as a Flatpak all read `color-scheme` from it, and a
//! sandboxed app has no other way to find out at all.
//!
//! Slipstream names only itself in `XDG_CURRENT_DESKTOP`, so no other desktop's backend answers
//! for it. Without this one nothing would, and every app would fall back to light.
//!
//! A followed Omarchy theme reaches apps the same way: the portal's `accent-color` is the colour
//! chosen for Slipstream's ring, which libadwaita apps and anything else that reads the key
//! take for their own buttons and selections, and GNOME's `icon-theme` is the theme's icons.
//!
//! `session/slipstream.portal` points xdg-desktop-portal at the bus name below and
//! `slipstream-portals.conf` asks for it first; the rest of the namespace (fonts, contrast, and
//! the accent colour where no theme is followed) still goes on to whatever backend the machine
//! has.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use slipstream_config::{ColourScheme, theme::Theme};
use zbus::{
    interface,
    object_server::SignalEmitter,
    zvariant::{OwnedValue, Value},
};

const NAME: &str = "org.freedesktop.impl.portal.desktop.slipstream";
const PATH: &str = "/org/freedesktop/portal/desktop";
const INTERFACE: &str = "org.freedesktop.impl.portal.Settings";

/// The cross-desktop namespace, and the only one that carries a preference rather than a theme's
/// name.
const APPEARANCE: &str = "org.freedesktop.appearance";
/// GNOME's namespace. GTK reads the theme's name from here, so apps that follow a theme rather
/// than a preference still turn dark.
const GNOME: &str = "org.gnome.desktop.interface";

/// What apps are told.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub scheme: ColourScheme,
    /// The followed theme's accent, red, green and blue from 0 to 1.
    pub accent: Option<(f64, f64, f64)>,
    /// The followed theme's icon theme.
    pub icons: Option<String>,
}

impl Look {
    /// What `settings` and the followed `theme`, if there is one, have apps told.
    pub fn of(settings: &slipstream_config::Settings, theme: Option<&Theme>) -> Self {
        Self {
            scheme: settings.appearance.scheme_with(theme),
            accent: theme.map(|theme| {
                let [r, g, b] = settings.borders.ring(Some(theme)).map(|c| c as f64 / 255.0);
                (r, g, b)
            }),
            icons: theme.and_then(|theme| theme.icons.clone()),
        }
    }
}

/// What the settings file and the theme say now. Read by the bus thread, written by the main
/// loop.
static LOOK: Mutex<Look> = Mutex::new(Look {
    scheme: ColourScheme::Dark,
    accent: None,
    icons: None,
});
/// The connection, once the name is ours, for sending `SettingChanged`.
static CONNECTION: OnceLock<zbus::blocking::Connection> = OnceLock::new();

/// Every namespace and key Slipstream answers for, as they stand. What no theme sets is left
/// out, and so left to another backend.
fn readings() -> Vec<(&'static str, Vec<(&'static str, OwnedValue)>)> {
    let look = LOOK.lock().unwrap().clone();
    let text = |value: &str| OwnedValue::from(zbus::zvariant::Str::from(value.to_string()));
    let mut appearance = vec![("color-scheme", OwnedValue::from(look.scheme.portal_value()))];
    if let Some(accent) = look.accent
        && let Ok(value) = OwnedValue::try_from(Value::from(accent))
    {
        appearance.push(("accent-color", value));
    }
    let mut gnome = vec![
        ("color-scheme", text(look.scheme.gnome_value())),
        ("gtk-theme", text(look.scheme.gtk_theme())),
    ];
    if let Some(icons) = &look.icons {
        gnome.push(("icon-theme", text(icons)));
    }
    vec![(APPEARANCE, appearance), (GNOME, gnome)]
}

/// Whether a namespace is one of those asked for. No namespaces at all means all of them, and a
/// name ending in `*` matches everything that starts with the rest of it, as the portal's
/// interface describes.
fn asked_for(namespace: &str, filters: &[String]) -> bool {
    filters.is_empty()
        || filters.iter().any(|filter| match filter.strip_suffix('*') {
            Some(prefix) => namespace.starts_with(prefix),
            None => namespace == filter,
        })
}

fn reading(namespace: &str, key: &str) -> Option<OwnedValue> {
    readings()
        .into_iter()
        .filter(|(held, _)| *held == namespace)
        .flat_map(|(_, keys)| keys)
        .find(|(held, _)| *held == key)
        .map(|(_, value)| value)
}

struct Server;

#[interface(name = "org.freedesktop.impl.portal.Settings")]
impl Server {
    /// Everything Slipstream answers for, in the namespaces asked for.
    async fn read_all(
        &self,
        namespaces: Vec<String>,
    ) -> HashMap<String, HashMap<String, OwnedValue>> {
        readings()
            .into_iter()
            .filter(|(namespace, _)| asked_for(namespace, &namespaces))
            .map(|(namespace, keys)| {
                let keys = keys
                    .into_iter()
                    .map(|(key, value)| (key.to_string(), value))
                    .collect();
                (namespace.to_string(), keys)
            })
            .collect()
    }

    /// One value. `ReadOne` is what a portal that knows this interface's second version calls;
    /// `Read` is the same thing under the name the first version used, and the portal still calls
    /// it once at startup.
    async fn read(&self, namespace: &str, key: &str) -> zbus::fdo::Result<OwnedValue> {
        self.read_one(namespace, key).await
    }

    async fn read_one(&self, namespace: &str, key: &str) -> zbus::fdo::Result<OwnedValue> {
        // An error rather than a value is how a backend says "not mine": the portal moves on to
        // the next one, which is what should answer for fonts, contrast and the accent colour.
        reading(namespace, key).ok_or_else(|| {
            zbus::fdo::Error::UnknownProperty(format!("Slipstream doesn't set {namespace} {key}"))
        })
    }

    #[zbus(property, name = "version")]
    fn version(&self) -> u32 {
        2
    }

    #[zbus(signal)]
    async fn setting_changed(
        emitter: &SignalEmitter<'_>,
        namespace: &str,
        key: &str,
        value: Value<'_>,
    ) -> zbus::Result<()>;
}

/// Takes the appearance backend's name on the session bus and serves it. If something else has
/// the name there is another Slipstream running, and this one leaves the setting to it.
pub fn serve(look: Look) {
    *LOOK.lock().unwrap() = look;
    std::thread::spawn(move || {
        let connection = zbus::blocking::connection::Builder::session()
            .and_then(|builder| builder.name(NAME))
            .and_then(|builder| builder.serve_at(PATH, Server))
            .and_then(|builder| builder.build());
        match connection {
            Ok(connection) => {
                tracing::info!("serving the appearance settings portal on the session bus");
                let _ = CONNECTION.set(connection);
            }
            Err(err) => tracing::warn!("couldn't serve the appearance settings portal: {err}"),
        }
    });
}

/// The settings or the theme changed. Apps are told one key at a time, as the interface asks,
/// and take it without restarting.
pub fn set(look: Look) {
    {
        let mut held = LOOK.lock().unwrap();
        if *held == look {
            return;
        }
        *held = look;
    }
    let Some(connection) = CONNECTION.get().cloned() else {
        return;
    };
    std::thread::spawn(move || {
        for (namespace, keys) in readings() {
            for (key, value) in keys {
                let body = (namespace, key, Value::from(value));
                if let Err(err) =
                    connection.emit_signal(None::<&str>, PATH, INTERFACE, "SettingChanged", &body)
                {
                    tracing::warn!(namespace, key, "couldn't send SettingChanged: {err}");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaces_filter_by_name_and_by_prefix() {
        let all: Vec<String> = Vec::new();
        assert!(asked_for(APPEARANCE, &all));
        assert!(asked_for(APPEARANCE, &[APPEARANCE.to_string()]));
        assert!(asked_for(APPEARANCE, &["org.freedesktop.*".to_string()]));
        assert!(!asked_for(APPEARANCE, &[GNOME.to_string()]));
        assert!(!asked_for(APPEARANCE, &["org.gnome.*".to_string()]));
    }

    #[test]
    fn dark_is_the_portals_first_preference_and_light_its_second() {
        assert_eq!(ColourScheme::Dark.portal_value(), 1);
        assert_eq!(ColourScheme::Light.portal_value(), 2);
    }

    #[test]
    fn keys_outside_slipstreams_own_are_left_to_another_backend() {
        assert!(reading(APPEARANCE, "color-scheme").is_some());
        assert!(reading("org.gnome.desktop.a11y", "always-show-text-caret").is_none());
    }

    #[test]
    fn a_followed_theme_gives_apps_its_accent_and_its_icons() {
        let text = "mode = \"light\"\nbackground = \"#eff1f5\"\nforeground = \"#4c4f69\"\n\
                    accent = \"#ff0080\"\n";
        let mut theme = slipstream_config::theme::parse(text, false).unwrap();
        theme.icons = Some("Yaru-blue".into());
        let settings = slipstream_config::Settings::default();
        let look = Look::of(&settings, Some(&theme));
        assert_eq!(look.scheme, ColourScheme::Light);
        assert_eq!(look.accent, Some((1.0, 0.0, 128.0 / 255.0)));
        assert_eq!(look.icons.as_deref(), Some("Yaru-blue"));
        // With none, both are another backend's to answer.
        let own = Look::of(&settings, None);
        assert_eq!(
            (own.scheme, own.accent, own.icons),
            (ColourScheme::Dark, None, None)
        );
    }
}
