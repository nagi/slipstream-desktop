//! The workspaces' list: their order, names and ids.

use serde::{Deserialize, Serialize};

/// The most workspaces there can be. The bar has to find room for every one of them.
pub const MAX_WORKSPACES: usize = 20;

/// The workspaces, in order, each with a name of its own or none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Workspaces {
    pub list: Vec<WorkspaceEntry>,
}

/// One workspace. The id is what the compositor follows when the list is edited, so renaming,
/// reordering or deleting its neighbours never moves a window off it; the name is what shows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct WorkspaceEntry {
    pub id: u32,
    /// Empty means the workspace goes by its number.
    pub name: String,
    /// The screen it opens on, by connector name (`DP-1`). Empty, or a screen that isn't
    /// connected, leaves it to open wherever the keyboard is.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub screen: String,
    /// Left off the bar while nothing is on it, as an app's workspace that comes and goes with
    /// the app. It still shows while it's on screen.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hide_when_empty: bool,
}

impl Default for Workspaces {
    /// Five, going by their numbers.
    fn default() -> Self {
        Self {
            list: (1..=5)
                .map(|id| WorkspaceEntry {
                    id,
                    name: String::new(),
                    ..Default::default()
                })
                .collect(),
        }
    }
}

impl Workspaces {
    /// The list as the compositor uses it: never empty, never over `MAX_WORKSPACES`, each id once
    /// (a repeated or missing id gets a fresh one rather than two workspaces merging), and names
    /// trimmed.
    pub fn entries(&self) -> Vec<WorkspaceEntry> {
        let mut out: Vec<WorkspaceEntry> = Vec::new();
        for entry in self.list.iter().take(MAX_WORKSPACES) {
            let id = if entry.id == 0 || out.iter().any(|seen| seen.id == entry.id) {
                let taken: Vec<u32> = self
                    .list
                    .iter()
                    .chain(out.iter())
                    .map(|entry| entry.id)
                    .collect();
                unused_id(&taken)
            } else {
                entry.id
            };
            out.push(WorkspaceEntry {
                id,
                name: entry.name.trim().to_string(),
                screen: entry.screen.trim().to_string(),
                hide_when_empty: entry.hide_when_empty,
            });
        }
        if out.is_empty() {
            out.push(WorkspaceEntry {
                id: 1,
                name: String::new(),
                ..Default::default()
            });
        }
        out
    }

    /// An id no workspace in the list has, for one being added.
    pub fn fresh_id(&self) -> u32 {
        let taken: Vec<u32> = self.list.iter().map(|entry| entry.id).collect();
        unused_id(&taken)
    }
}

/// An id none of `taken` is: one past the highest, or, when the highest is already `u32::MAX` (a
/// hand edit can put it there), the lowest free one from 1. There are never more than a few dozen
/// ids, so the search always ends.
pub fn unused_id(taken: &[u32]) -> u32 {
    match taken.iter().max() {
        None => 1,
        Some(highest) => highest
            .checked_add(1)
            .unwrap_or_else(|| (1..u32::MAX).find(|id| !taken.contains(id)).unwrap_or(1)),
    }
}

/// What a workspace is called on screen: its name, or its number counting from 1.
pub fn workspace_label(name: &str, index: usize) -> String {
    let name = name.trim();
    if name.is_empty() {
        (index + 1).to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::parse;

    #[test]
    fn a_workspace_can_hide_while_empty_and_writes_back_only_when_it_does() {
        let settings = parse(
            "[[workspaces.list]]\nid = 1\nname = \"Chat\"\nhide-when-empty = true\n[[workspaces.list]]\nid = 2\nname = \"Web\"\n",
        )
        .unwrap();
        let entries = settings.workspaces.entries();
        assert!(entries[0].hide_when_empty);
        assert!(!entries[1].hide_when_empty, "shown by default");
        let text = toml::to_string(&settings).unwrap();
        assert_eq!(text.matches("hide-when-empty").count(), 1);
        assert_eq!(parse(&text).unwrap().workspaces, settings.workspaces);
    }

    #[test]
    fn workspaces_default_to_five_numbers_and_are_never_empty_or_doubled() {
        let settings = parse("").unwrap();
        let entries = settings.workspaces.entries();
        assert_eq!(entries.len(), 5);
        assert!(entries.iter().all(|entry| entry.name.is_empty()));
        assert_eq!(workspace_label("", 2), "3");
        assert_eq!(workspace_label("  Mail ", 0), "Mail");

        let none = Workspaces { list: Vec::new() };
        assert_eq!(none.entries().len(), 1, "never no workspaces at all");

        let doubled = parse(
            "[[workspaces.list]]\nid = 4\nname = \"Mail\"\n[[workspaces.list]]\nid = 4\nname = \"Web\"\n",
        )
        .unwrap();
        let entries = doubled.workspaces.entries();
        assert_eq!(entries.len(), 2);
        assert_ne!(
            entries[0].id, entries[1].id,
            "a repeated id doesn't merge two workspaces"
        );

        let many = Workspaces {
            list: (1..=40)
                .map(|id| WorkspaceEntry {
                    id,
                    name: String::new(),
                    ..Default::default()
                })
                .collect(),
        };
        assert_eq!(many.entries().len(), MAX_WORKSPACES);
        assert_eq!(many.fresh_id(), 41);
    }

    #[test]
    fn an_id_at_the_top_of_its_range_gets_no_neighbour_past_it() {
        let file = parse(
            "[[workspaces.list]]\nid = 4294967295\nname = \"Mail\"\n\
             [[workspaces.list]]\nid = 4294967295\nname = \"Web\"\n\
             [[workspaces.list]]\nid = 0\nname = \"\"\n",
        )
        .unwrap();
        let entries = file.workspaces.entries();
        let ids: Vec<u32> = entries.iter().map(|entry| entry.id).collect();
        assert_eq!(
            ids,
            [u32::MAX, 1, 2],
            "the doubled id and the missing one get free ones"
        );
        assert_eq!(file.workspaces.fresh_id(), 1);
        assert_eq!(unused_id(&[]), 1);
        assert_eq!(unused_id(&[3, 7]), 8);
    }
}
