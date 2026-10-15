//! The sounds Slipstream makes itself.

use super::*;

pub(super) fn build(store: &Store) -> gtk::Widget {
    let page = page("Sound", "The sounds Slipstream makes itself");
    let keys = group(&page, "Volume");

    let blip = gtk::CheckButton::new();
    blip.set_active(store.get().sound.volume_blip);
    row(
        &keys,
        "Click when the volume changes",
        Some(
            "A short click through the speakers at the new level, from the volume keys or quick \
             settings' slider, as macOS and Windows do, so the volume can be heard as well as \
             seen. Never on mute, and never on brightness.",
        ),
        &blip,
    );
    let store = store.clone();
    blip.connect_toggled(move |blip| {
        let on = blip.is_active();
        store.change(move |settings| settings.sound.volume_blip = on);
    });
    page.upcast()
}
