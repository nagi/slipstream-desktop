//! The living wallpaper's settings, and the table of its variations.

use serde::{Deserialize, Serialize};

/// The living wallpaper, and when the UI steps aside for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Wallpaper {
    /// Seconds without keyboard or pointer input before the UI fades out; 0 never fades.
    pub fade_after_secs: u64,
    /// Which variations to show, by id; they take turns in a shuffled order. Empty means all of
    /// them. Unknown ids are ignored, and if that leaves none, the default is used.
    pub variations: Vec<String>,
    /// Minutes one variation holds before the next takes over. Ignored when only one is chosen;
    /// 0 never changes.
    pub change_every_mins: u64,
}

impl Default for Wallpaper {
    fn default() -> Self {
        Self {
            fade_after_secs: 120,
            variations: vec![VARIATIONS[0].id.to_string()],
            change_every_mins: 10,
        }
    }
}

impl Wallpaper {
    /// The variations to cycle through: the ones named in the file, in the file's order, or all
    /// of them if it names none. Never empty — a file that names only ids nobody knows falls
    /// back to the default.
    pub fn chosen(&self) -> Vec<&'static Variation> {
        if self.variations.is_empty() {
            return VARIATIONS.iter().collect();
        }
        let mut chosen: Vec<&'static Variation> = Vec::new();
        for id in &self.variations {
            let id = current_id(id);
            if let Some(variation) = VARIATIONS.iter().find(|v| v.id == id) {
                // A file naming a variation and the one it replaced shows it once.
                if !chosen.contains(&variation) {
                    chosen.push(variation);
                }
            }
        }
        if chosen.is_empty() {
            vec![&VARIATIONS[0]]
        } else {
            chosen
        }
    }
}

/// Variations that were replaced, and the id of what replaced each, so a settings file written
/// before the change keeps showing something in the same place in its list.
const REPLACED: &[(&str, &str)] = &[
    ("decode", "vortex"),
    ("rain", "departures"),
    ("wind-tunnel", "coral"),
    ("flow-field", "attractor"),
    ("shear", "murmuration"),
    ("turntable", "murmuration"),
    ("mach", "physarum"),
    ("canyon", "physarum"),
    ("console", "maze"),
];

/// The id a settings file's variation goes by now: itself, or what replaced it.
pub fn current_id(id: &str) -> &str {
    REPLACED
        .iter()
        .find(|(old, _)| *old == id)
        .map_or(id, |(_, new)| new)
}

/// One living-wallpaper variation, as the settings file and the Settings app know it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variation {
    pub id: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
}

/// Every variation, in the order the Settings app lists them.
/// The compositor's `saver::make` builds one for each id, and a test there checks the two agree.
pub const VARIATIONS: &[Variation] = &[
    Variation {
        id: "slipstream",
        title: "Slipstream",
        blurb: "The logo arrives letter by letter, holds, and peels away downwind in smoke.",
    },
    Variation {
        id: "vortex",
        title: "Vortex",
        blurb: "The logo sits in the eye of a turning tunnel of light, and rings on the minute.",
    },
    Variation {
        id: "departures",
        title: "Departures",
        blurb: "A split-flap board with the time, date, workspace and battery, turning as they \
                change.",
    },
    Variation {
        id: "prompt",
        title: "Prompt",
        blurb: "A command is typed at a terminal, and prints the logo as a banner.",
    },
    Variation {
        id: "circuit",
        title: "Circuit",
        blurb: "Tracks are routed in from the edges, and pulses run along them to the logo.",
    },
    Variation {
        id: "life",
        title: "Life",
        blurb: "Conway's Game of Life, seeded with the logo, with a spark dropped in when it \
                settles.",
    },
    Variation {
        id: "sonar",
        title: "Sonar",
        blurb: "A beam sweeps round, and everything it touches answers and fades behind it.",
    },
    Variation {
        id: "tide",
        title: "Tide",
        blurb: "Swell crosses the screen in shaded bands, and the logo surfaces out of it.",
    },
    Variation {
        id: "warp",
        title: "Warp",
        blurb: "A steering tunnel of streaks, and the logo comes up out of its vanishing point.",
    },
    Variation {
        id: "glitch",
        title: "Glitch",
        blurb: "The picture smears like a broken video stream, then swirls away.",
    },
    Variation {
        id: "contours",
        title: "Contours",
        blurb: "A pressure map of the air round the logo, with a weather system drifting across.",
    },
    Variation {
        id: "contrails",
        title: "Contrails",
        blurb: "Aircraft leave trails that light up the logo, and now and then skywrite the time.",
    },
    Variation {
        id: "coral",
        title: "Coral",
        blurb: "The letters seed a chemical reaction that grows over the screen in stripes and \
                spots, then dies back.",
    },
    Variation {
        id: "attractor",
        title: "Attractor",
        blurb: "The logo unravels into a strange attractor that slowly changes shape, then winds \
                back into letters.",
    },
    Variation {
        id: "murmuration",
        title: "Murmuration",
        blurb: "The logo takes off as a flock of starlings that wheels round the screen, scatters \
                from a falcon, and lands back in the letters.",
    },
    Variation {
        id: "maze",
        title: "Maze",
        blurb: "A walk through a maze, seen first hand, to the logo on the wall at its end.",
    },
    Variation {
        id: "physarum",
        title: "Physarum",
        blurb: "A slime mould creeps out of the letters, spreads over the screen as a network of \
                veins, then draws back into the logo.",
    },
    Variation {
        id: "galaxies",
        title: "Galaxies",
        blurb: "The two halves of the logo wind up into spiral galaxies that collide and merge, \
                then the collision runs backwards into letters.",
    },
    Variation {
        id: "chladni",
        title: "Chladni",
        blurb: "The logo is sand on a ringing plate: each note shakes it into a new figure, and \
                then it walks home.",
    },
    Variation {
        id: "frost",
        title: "Frost",
        blurb: "Frost grows out of the letters in branching ferns, glitters, and melts back the \
                way it came.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chosen_variations_are_the_file_s_own_order_and_never_empty() {
        let all = Wallpaper {
            variations: Vec::new(),
            ..Default::default()
        };
        assert_eq!(all.chosen().len(), VARIATIONS.len(), "none named means all");
        let some = Wallpaper {
            variations: vec!["vortex".into(), "nothing".into(), "slipstream".into()],
            ..Default::default()
        };
        let ids: Vec<&str> = some.chosen().iter().map(|v| v.id).collect();
        assert_eq!(ids, ["vortex", "slipstream"]);
        let nonsense = Wallpaper {
            variations: vec!["nothing".into()],
            ..Default::default()
        };
        assert_eq!(nonsense.chosen(), vec![&VARIATIONS[0]]);
    }

    #[test]
    fn a_replaced_variation_s_id_still_chooses_its_replacement() {
        let old = Wallpaper {
            variations: vec!["decode".into(), "slipstream".into(), "vortex".into()],
            ..Default::default()
        };
        let ids: Vec<&str> = old.chosen().iter().map(|v| v.id).collect();
        assert_eq!(ids, ["vortex", "slipstream"], "in its place, and once");
        for (old, new) in REPLACED {
            assert!(
                VARIATIONS.iter().all(|v| v.id != *old),
                "{old} is replaced, so it isn't in the table"
            );
            assert_eq!(current_id(old), *new);
        }
        assert_eq!(current_id("tide"), "tide");
    }

    #[test]
    fn every_variation_is_named_once_with_something_to_read() {
        for (i, variation) in VARIATIONS.iter().enumerate() {
            assert!(
                VARIATIONS[..i].iter().all(|other| other.id != variation.id),
                "{} appears twice",
                variation.id
            );
            assert!(!variation.title.is_empty() && variation.blurb.len() > 20);
            assert!(
                variation
                    .id
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch == '-'),
                "{} should be a plain id",
                variation.id
            );
        }
    }
}
