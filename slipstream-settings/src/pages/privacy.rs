//! Programs allowed to record the screen without asking.

use super::*;

/// Programs allowed to record the screen without asking, each of which can be taken back.
pub(super) fn build(store: &Store) -> gtk::Widget {
    let page = page(
        "Privacy",
        "What programs have been allowed to do without asking again. Nothing is added here: \
         Slipstream asks when a program first tries, and this is where an answer is taken back.",
    );
    fill_capture_allowed(&group(&page, "Screen recording"), store);
    page.upcast()
}

/// The allowed programs, built again from the file whenever one is taken away.
fn fill_capture_allowed(card: &gtk::Box, store: &Store) {
    while let Some(child) = card.first_child() {
        card.remove(&child);
    }
    let allowed = store.get().privacy.screen_capture_allowed;
    if allowed.is_empty() {
        row(
            card,
            "No programs are allowed",
            Some(
                "A program that asks to record the screen puts up a card naming it. Answer \
                 \u{201c}Always allow\u{201d} and it appears here.",
            ),
            &gtk::Box::new(gtk::Orientation::Horizontal, 0),
        );
        return;
    }
    let refill = refiller(card, store, fill_capture_allowed);
    for exe in allowed {
        // The program's own name reads as the answer given; the path is what was written down,
        // and two programs of the same name in different places are not the same allowance.
        let name = exe.rsplit('/').next().unwrap_or(&exe).to_string();
        let delete = gtk::Button::from_icon_name("user-trash-symbolic");
        delete.set_tooltip_text(Some("Ask again next time"));
        delete.add_css_class("flat");
        {
            let store = store.clone();
            let refill = refill.clone();
            let exe = exe.clone();
            delete.connect_clicked(move |_| {
                let exe = exe.clone();
                store.change(move |settings| settings.privacy.forget_capture(&exe));
                refill();
            });
        }
        row(card, &name, Some(&exe), &delete);
    }
}
