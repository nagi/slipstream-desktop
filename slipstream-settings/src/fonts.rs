//! Slipstream's fonts, embedded as the compositor embeds them, and handed to Pango from files in
//! the cache folder, so the app matches the desktop without installing fonts system-wide.

use std::fs;

use gtk::prelude::*;
use gtk4 as gtk;
use pango::prelude::*;

const FONTS: [(&str, &[u8]); 5] = [
    (
        "AtkinsonHyperlegible-Regular.ttf",
        include_bytes!("../../assets/fonts/AtkinsonHyperlegible-Regular.ttf"),
    ),
    (
        "AtkinsonHyperlegible-Bold.ttf",
        include_bytes!("../../assets/fonts/AtkinsonHyperlegible-Bold.ttf"),
    ),
    (
        "ChakraPetch-Bold.ttf",
        include_bytes!("../../assets/fonts/ChakraPetch-Bold.ttf"),
    ),
    (
        "JetBrainsMono-Regular.ttf",
        include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"),
    ),
    (
        "JetBrainsMono-Bold.ttf",
        include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf"),
    ),
];

/// Makes the fonts available to this app. Without them, the stylesheet's fallbacks apply.
pub fn register() {
    let dir = slipstream_config::xdg_home("XDG_CACHE_HOME", ".cache")
        .join("slipstream")
        .join("fonts");
    if let Err(err) = fs::create_dir_all(&dir) {
        eprintln!(
            "slipstream-settings: can't unpack fonts into {}: {err}",
            dir.display()
        );
        return;
    }
    let Some(font_map) = gtk::Label::new(None).pango_context().font_map() else {
        return;
    };
    for (name, bytes) in FONTS {
        let path = dir.join(name);
        let unpacked = fs::metadata(&path).is_ok_and(|meta| meta.len() == bytes.len() as u64);
        if !unpacked && let Err(err) = slipstream_config::replace(&path, bytes) {
            eprintln!(
                "slipstream-settings: can't unpack {}: {err}",
                path.display()
            );
            continue;
        }
        if let Err(err) = font_map.add_font_file(&path) {
            eprintln!("slipstream-settings: can't load {}: {err}", path.display());
        }
    }
}
