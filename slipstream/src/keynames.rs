//! Keys named in the active style. The tour, its miniature and the hints are written with the
//! Windows style's keys; this finds what the same action is pressed with in another style, so the
//! text names the keys that actually work.
//!
//! Text marks the keys it names in braces: `{Super+R}` is that key, `{mod:Super+Alt+←}` the
//! modifier it adds to Super (`Alt`), `{key:Super+T}` the key without its modifiers (`T`), and
//! `{tap}` how the explorer opens ("Tap Super on its own", or "Press Super+Space").

use slipstream_config::KeyStyle;

use crate::keys::{self, Action, Binding};

#[derive(Debug, Clone, PartialEq)]
pub struct KeyNames {
    style: KeyStyle,
    /// Each Windows-style key's label, and the binding for the same action in `style`.
    names: Vec<(String, Binding)>,
}

impl Default for KeyNames {
    fn default() -> Self {
        Self::new(KeyStyle::default())
    }
}

impl KeyNames {
    pub fn new(style: KeyStyle) -> Self {
        let windows = keys::checked(keys::defaults(KeyStyle::Windows), KeyStyle::Windows);
        let own = keys::checked(keys::defaults(style), style);
        let names = windows
            .iter()
            .filter_map(|binding| {
                // The same key for the same action when the style keeps it, else the style's first
                // key for that action.
                let same = own.iter().find(|b| **b == *binding);
                let other = own.iter().find(|b| b.action == binding.action);
                let found = same.or(other)?;
                Some((keys::label(binding.mods, binding.key), *found))
            })
            .collect();
        Self { style, names }
    }

    pub fn style(&self) -> KeyStyle {
        self.style
    }

    fn binding(&self, label: &str) -> Option<Binding> {
        let label = label.replace('⏎', "Enter");
        self.names
            .iter()
            .find(|(name, _)| *name == label)
            .map(|(_, binding)| *binding)
    }

    /// The keycap for `cap`, a Windows-style key or set of keys (`Super+Alt+← → ↑ ↓`,
    /// `Super+1–9`). A cap with no action behind it, such as Esc, is itself.
    pub fn cap(&self, cap: &str) -> String {
        if cap == "Super" {
            return self.explorer();
        }
        if let Some(binding) = self.binding(cap) {
            let label = keys::label(binding.mods, binding.key);
            return match cap.contains('⏎') {
                true => label.replace("Enter", "⏎"),
                false => label,
            };
        }
        // A set: its first key decides the modifiers when the style keeps the same keys, and
        // otherwise each key is named in turn.
        let Some((mods, keys_part)) = cap.rsplit_once('+') else {
            return cap.to_string();
        };
        let first: String = keys_part
            .split([' ', '–'])
            .next()
            .unwrap_or_default()
            .to_string();
        let Some(binding) = self.binding(&format!("{mods}+{first}")) else {
            return cap.to_string();
        };
        if keys::key_name(binding.key) == first {
            return keys::mods_prefix(binding.mods) + keys_part;
        }
        keys_part
            .split(' ')
            .map(|key| self.cap(&format!("{mods}+{key}")))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The explorer's key, as a keycap.
    fn explorer(&self) -> String {
        if keys::taps_super(self.style) {
            return "Super".to_string();
        }
        keys::defaults(self.style)
            .iter()
            .find(|binding| binding.action == Action::Explorer)
            .map(|binding| keys::label(binding.mods, binding.key))
            .unwrap_or_else(|| "Super".to_string())
    }

    /// `text` with each key in braces named for the style. Arrows and Enter are spelled out
    /// (`Super+Ctrl+Up`), since running text has no symbols for them.
    pub fn fill(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let Some(close) = rest[open..].find('}') else {
                break;
            };
            let token = &rest[open + 1..open + close];
            out.push_str(&spelled(&self.token(token)));
            rest = &rest[open + close + 1..];
        }
        out.push_str(rest);
        out
    }

    fn token(&self, token: &str) -> String {
        if token == "tap" {
            return match keys::taps_super(self.style) {
                true => "Tap Super on its own".to_string(),
                false => format!("Press {}", self.explorer()),
            };
        }
        if let Some(cap) = token.strip_prefix("mod:") {
            let Some(binding) = self.binding(cap) else {
                return cap.to_string();
            };
            let without_super = keys::Mods {
                logo: false,
                ..binding.mods
            };
            return keys::mods_prefix(without_super)
                .trim_end_matches('+')
                .to_string();
        }
        if let Some(cap) = token.strip_prefix("key:") {
            return self
                .binding(cap)
                .map(|binding| keys::key_name(binding.key))
                .unwrap_or_else(|| cap.to_string());
        }
        self.cap(token)
    }
}

/// `cap` with its arrows and Enter as words.
fn spelled(cap: &str) -> String {
    [
        ('←', "Left"),
        ('→', "Right"),
        ('↑', "Up"),
        ('↓', "Down"),
        ('⏎', "Enter"),
    ]
    .iter()
    .fold(cap.to_string(), |cap, (symbol, word)| {
        cap.replace(*symbol, word)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_windows_style_names_its_own_keys() {
        let names = KeyNames::new(KeyStyle::Windows);
        for cap in [
            "Super+R",
            "Super+Shift+R",
            "Super+Alt+← → ↑ ↓",
            "Super+Shift+1–9",
            "Super+⏎",
            "Esc",
        ] {
            assert_eq!(names.cap(cap), cap);
        }
        assert_eq!(names.cap("Super"), "Super");
        assert_eq!(
            names.fill("{tap}, hold {mod:Super+Alt+←}, each further {key:Super+T}"),
            "Tap Super on its own, hold Alt, each further T"
        );
    }

    #[test]
    fn other_styles_name_the_same_actions_keys() {
        let mac = KeyNames::new(KeyStyle::Mac);
        assert_eq!(mac.cap("Super"), "Super+Space");
        assert_eq!(mac.fill("{Super+Tab} tips"), "Super+Ctrl+Up tips");
        assert_eq!(mac.cap("Super+Tab"), "Super+Ctrl+↑");
        assert_eq!(mac.cap("Super+Shift+1–9"), "Super+Alt+1–9");
        assert_eq!(mac.cap("Super+1–9"), "Super+1–9");
        assert_eq!(mac.fill("with {mod:Super+Shift+1}"), "with Alt");
        let omarchy = KeyNames::new(KeyStyle::Omarchy);
        assert_eq!(omarchy.cap("Super+R"), "Super+J");
        assert_eq!(omarchy.cap("Super+Alt+← → ↑ ↓"), "Super+Shift+← → ↑ ↓");
        assert_eq!(omarchy.cap("Super+]"), "Super+=");
        assert_eq!(omarchy.cap("Super+/"), "Super+K");
        assert_eq!(omarchy.cap("Super+⏎"), "Super+⏎");
        assert_eq!(
            omarchy.fill("{tap} and {Super+T}; each further {key:Super+T}"),
            "Press Super+Space and Super+L; each further L"
        );
    }

    #[test]
    fn a_key_the_style_has_no_action_for_is_left_as_written() {
        // Omarchy has no key that sends a window to the next workspace along.
        let omarchy = KeyNames::new(KeyStyle::Omarchy);
        assert_eq!(omarchy.cap("Super+Shift+→"), "Super+Shift+→");
    }
}
