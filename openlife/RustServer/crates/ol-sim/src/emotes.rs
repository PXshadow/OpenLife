//! Named emote catalog for PE / SAY (Haxe `Emote` + `emotionWords.ini` subset).
//!
//! Indices follow OneLifeData7 `contentSettings/emotionWords.ini` line order
//! (0-based), which drives client PE face/body layers. Haxe
//! `GlobalPlayerInstance.Emote` matches for 0–33; later body gestures
//! (`point`/`wave`/…) come from the content word list.

/// One named emote entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmoteEntry {
    /// Wire / SAY token (uppercase, no leading slash).
    pub name: &'static str,
    /// PE emot index sent as `PE p_id index`.
    pub index: i32,
    /// Player may trigger via `SAY NAME` / `SAY EMOTE NAME` (content `/` words + aliases).
    pub player: bool,
}

/// Canonical catalog: emotionWords order + useful aliases (`YAWN`, `DANCE`, `TAUNT`).
///
/// Player-facing verbs are those with `player: true`. System-only faces stay
/// queryable via `?EMOTES` / `EMOTE <name>` when listed.
pub const EMOTES: &[EmoteEntry] = &[
    EmoteEntry {
        name: "HAPPY",
        index: 0,
        player: true,
    },
    EmoteEntry {
        name: "MAD",
        index: 1,
        player: true,
    },
    EmoteEntry {
        name: "ANGRY",
        index: 2,
        player: true,
    },
    // AFK / SAY YAWN face (historically index 2 / angry-ish closed face).
    EmoteEntry {
        name: "YAWN",
        index: 2,
        player: true,
    },
    EmoteEntry {
        name: "SAD",
        index: 3,
        player: true,
    },
    EmoteEntry {
        name: "DEVIOUS",
        index: 4,
        player: true,
    },
    EmoteEntry {
        name: "JOY",
        index: 5,
        player: true,
    },
    // Celebratory body language alias → joy.
    EmoteEntry {
        name: "DANCE",
        index: 5,
        player: true,
    },
    EmoteEntry {
        name: "BLUSH",
        index: 6,
        player: true,
    },
    EmoteEntry {
        name: "YELLOWFEVER",
        index: 7,
        player: false,
    },
    EmoteEntry {
        name: "SNOWSPLAT",
        index: 8,
        player: false,
    },
    EmoteEntry {
        name: "HUBBA",
        index: 9,
        player: true,
    },
    EmoteEntry {
        name: "ILL",
        index: 10,
        player: true,
    },
    EmoteEntry {
        name: "YOOHOO",
        index: 11,
        player: true,
    },
    EmoteEntry {
        name: "HMPH",
        index: 12,
        player: true,
    },
    EmoteEntry {
        name: "LOVE",
        index: 13,
        player: true,
    },
    EmoteEntry {
        name: "OREALLY",
        index: 14,
        player: true,
    },
    EmoteEntry {
        name: "SHOCK",
        index: 15,
        player: true,
    },
    EmoteEntry {
        name: "MURDERFACE",
        index: 16,
        player: false,
    },
    EmoteEntry {
        name: "PNEUMONIA",
        index: 18,
        player: false,
    },
    EmoteEntry {
        name: "BIOMERELIEF",
        index: 19,
        player: false,
    },
    EmoteEntry {
        name: "DEHYDRATION",
        index: 20,
        player: false,
    },
    EmoteEntry {
        name: "HEATSTROKE",
        index: 21,
        player: false,
    },
    EmoteEntry {
        name: "TERRIFIED",
        index: 27,
        player: false,
    },
    EmoteEntry {
        name: "HOMESICK",
        index: 28,
        player: false,
    },
    EmoteEntry {
        name: "SPICYFOOD",
        index: 29,
        player: false,
    },
    EmoteEntry {
        name: "REFUSEFOOD",
        index: 30,
        player: false,
    },
    EmoteEntry {
        name: "STARVING",
        index: 31,
        player: false,
    },
    EmoteEntry {
        name: "SATISFIED",
        index: 32,
        player: false,
    },
    EmoteEntry {
        name: "MIAMFOOD",
        index: 32,
        player: false,
    },
    EmoteEntry {
        name: "POINT",
        index: 34,
        player: true,
    },
    EmoteEntry {
        name: "WAIT",
        index: 35,
        player: true,
    },
    EmoteEntry {
        name: "WAVE",
        index: 36,
        player: true,
    },
    EmoteEntry {
        name: "HERE",
        index: 37,
        player: true,
    },
    EmoteEntry {
        name: "UPYOURS",
        index: 38,
        player: true,
    },
    // Hostile gesture default → mad face.
    EmoteEntry {
        name: "TAUNT",
        index: 1,
        player: true,
    },
];

/// Default PE index for bare `SAY EMOTE` / missing args.
pub const DEFAULT_EMOTE_INDEX: i32 = 0;

/// PE index for `SAY WAVE` (`emotionWords` /wave).
pub const WAVE_EMOT_INDEX: i32 = 36;

/// PE index for `SAY POINT` (`emotionWords` /point).
pub const POINT_EMOT_INDEX: i32 = 34;

/// PE index for `SAY DANCE` (alias of joy).
pub const DANCE_EMOT_INDEX: i32 = 5;

/// PE index for bare `SAY TAUNT` (mad).
pub const TAUNT_EMOT_INDEX: i32 = 1;

/// Look up emote by name (case-insensitive). Accepts optional leading `/`.
pub fn emote_by_name(name: &str) -> Option<&'static EmoteEntry> {
    let n = name.trim().trim_start_matches('/');
    if n.is_empty() {
        return None;
    }
    let upper = n.to_ascii_uppercase();
    EMOTES.iter().find(|e| e.name == upper)
}

/// Resolve a token to a PE index: integer parse, else named emote.
pub fn resolve_emote_token(token: &str) -> Option<i32> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(n) = t.parse::<i32>() {
        if (0..=64).contains(&n) {
            return Some(n);
        }
        return None;
    }
    emote_by_name(t).map(|e| e.index)
}

/// `SAY ?EMOTES` / `SAY EMOTES` body without leading p_id.
///
/// Format: `EMOTES NAME=idx NAME=idx …` for player-facing entries only.
pub fn format_emotes_query() -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for e in EMOTES.iter().filter(|e| e.player) {
        // Prefer first listing when multiple names share an index (e.g. YAWN/ANGRY).
        if seen.insert(e.name) {
            parts.push(format!("{}={}", e.name, e.index));
        }
    }
    format!("EMOTES {}", parts.join(" "))
}

/// Result of parsing a SAY emote command (before rate limit / fan-out).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SayEmoteCmd {
    /// Emit PE with this index.
    Pe { index: i32 },
    /// PE point + optional relative target (map hint for speaker).
    Point {
        index: i32,
        dx: i32,
        dy: i32,
    },
}

/// Private speaker confirmation for `SAY POINT dx dy`.
///
/// Format: `POINT dx dy at tx ty` (absolute map tile = feet + delta).
pub fn format_point_confirm(dx: i32, dy: i32, feet_x: i32, feet_y: i32) -> String {
    let tx = feet_x + dx;
    let ty = feet_y + dy;
    format!("POINT {dx} {dy} at {tx} {ty}")
}

/// Parse `SAY` payload for named / numeric emote commands.
///
/// Supports:
/// - `EMOTE` / `EMOTE <n|name>`
/// - `YAWN`, `WAVE`, `DANCE`, `WAIT`, `HERE`, `HUBBA`, …
/// - `POINT` / `POINT <dx> <dy>`
/// - `TAUNT` / `TAUNT <name|n>` (default mad)
/// - bare player emote names: `HAPPY`, `MAD`, `JOY`, …
pub fn parse_say_emote(text: &str) -> Option<SayEmoteCmd> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut parts = text.split_whitespace();
    let verb = parts.next()?.to_ascii_uppercase();
    let rest: Vec<&str> = parts.collect();

    match verb.as_str() {
        "EMOTE" | "EMOT" => {
            let index = rest
                .first()
                .and_then(|t| resolve_emote_token(t))
                .unwrap_or(DEFAULT_EMOTE_INDEX);
            Some(SayEmoteCmd::Pe { index })
        }
        "POINT" => {
            let (dx, dy) = match rest.as_slice() {
                [a, b] => {
                    let dx = a.parse().ok()?;
                    let dy = b.parse().ok()?;
                    (dx, dy)
                }
                [] => (0, 0),
                _ => return None,
            };
            Some(SayEmoteCmd::Point {
                index: POINT_EMOT_INDEX,
                dx,
                dy,
            })
        }
        "TAUNT" => {
            let index = rest
                .first()
                .and_then(|t| resolve_emote_token(t))
                .unwrap_or(TAUNT_EMOT_INDEX);
            Some(SayEmoteCmd::Pe { index })
        }
        other => {
            // Bare named player emote (WAVE, DANCE, HAPPY, YAWN, …).
            let entry = emote_by_name(other)?;
            if !entry.player {
                return None;
            }
            if !rest.is_empty() {
                // Named verbs take no extra args (use EMOTE / POINT / TAUNT forms).
                return None;
            }
            Some(SayEmoteCmd::Pe {
                index: entry.index,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_dance_point_taunt_indices() {
        assert_eq!(emote_by_name("wave").unwrap().index, WAVE_EMOT_INDEX);
        assert_eq!(emote_by_name("DANCE").unwrap().index, DANCE_EMOT_INDEX);
        assert_eq!(emote_by_name("point").unwrap().index, POINT_EMOT_INDEX);
        assert_eq!(emote_by_name("taunt").unwrap().index, TAUNT_EMOT_INDEX);
        assert_eq!(emote_by_name("/happy").unwrap().index, 0);
        assert_eq!(emote_by_name("mad").unwrap().index, 1);
        assert_eq!(emote_by_name("love").unwrap().index, 13);
    }

    #[test]
    fn resolve_numeric_and_name() {
        assert_eq!(resolve_emote_token("0"), Some(0));
        assert_eq!(resolve_emote_token("36"), Some(WAVE_EMOT_INDEX));
        assert_eq!(resolve_emote_token("wave"), Some(WAVE_EMOT_INDEX));
        assert_eq!(resolve_emote_token("MAD"), Some(1));
        assert_eq!(resolve_emote_token("nope"), None);
        assert_eq!(resolve_emote_token("999"), None);
    }

    #[test]
    fn parse_wave_dance_yawn() {
        assert_eq!(
            parse_say_emote("WAVE"),
            Some(SayEmoteCmd::Pe {
                index: WAVE_EMOT_INDEX
            })
        );
        assert_eq!(
            parse_say_emote("dance"),
            Some(SayEmoteCmd::Pe {
                index: DANCE_EMOT_INDEX
            })
        );
        assert_eq!(
            parse_say_emote("YAWN"),
            Some(SayEmoteCmd::Pe { index: 2 })
        );
        assert_eq!(
            parse_say_emote("EMOTE 3"),
            Some(SayEmoteCmd::Pe { index: 3 })
        );
        assert_eq!(
            parse_say_emote("EMOTE wave"),
            Some(SayEmoteCmd::Pe {
                index: WAVE_EMOT_INDEX
            })
        );
        assert_eq!(
            parse_say_emote("EMOTE"),
            Some(SayEmoteCmd::Pe {
                index: DEFAULT_EMOTE_INDEX
            })
        );
    }

    #[test]
    fn parse_point_and_taunt() {
        assert_eq!(
            parse_say_emote("POINT"),
            Some(SayEmoteCmd::Point {
                index: POINT_EMOT_INDEX,
                dx: 0,
                dy: 0
            })
        );
        assert_eq!(
            parse_say_emote("POINT 2 -1"),
            Some(SayEmoteCmd::Point {
                index: POINT_EMOT_INDEX,
                dx: 2,
                dy: -1
            })
        );
        assert_eq!(parse_say_emote("POINT 1"), None);
        assert_eq!(
            parse_say_emote("TAUNT"),
            Some(SayEmoteCmd::Pe {
                index: TAUNT_EMOT_INDEX
            })
        );
        assert_eq!(
            parse_say_emote("TAUNT mad"),
            Some(SayEmoteCmd::Pe { index: 1 })
        );
        assert_eq!(
            parse_say_emote("TAUNT angry"),
            Some(SayEmoteCmd::Pe { index: 2 })
        );
        assert_eq!(
            parse_say_emote("TAUNT 15"),
            Some(SayEmoteCmd::Pe { index: 15 })
        );
    }

    #[test]
    fn format_emotes_lists_player() {
        let q = format_emotes_query();
        assert!(q.starts_with("EMOTES "));
        assert!(q.contains("WAVE=36"));
        assert!(q.contains("POINT=34"));
        assert!(q.contains("DANCE=5"));
        assert!(q.contains("TAUNT=1"));
        assert!(q.contains("HAPPY=0"));
        assert!(!q.contains("STARVING="), "system-only not in player list");
    }

    #[test]
    fn point_confirm() {
        assert_eq!(
            format_point_confirm(2, -1, 10, 20),
            "POINT 2 -1 at 12 19"
        );
    }

    #[test]
    fn haxe_core_indices_stable() {
        // GlobalPlayerInstance.Emote 0–15 subset.
        assert_eq!(emote_by_name("happy").unwrap().index, 0);
        assert_eq!(emote_by_name("mad").unwrap().index, 1);
        assert_eq!(emote_by_name("angry").unwrap().index, 2);
        assert_eq!(emote_by_name("sad").unwrap().index, 3);
        assert_eq!(emote_by_name("joy").unwrap().index, 5);
        assert_eq!(emote_by_name("love").unwrap().index, 13);
        assert_eq!(emote_by_name("shock").unwrap().index, 15);
        assert_eq!(emote_by_name("terrified").unwrap().index, 27);
        assert_eq!(emote_by_name("homesick").unwrap().index, 28);
    }

    #[test]
    fn non_player_bare_name_rejected() {
        assert!(parse_say_emote("STARVING").is_none());
        assert!(parse_say_emote("hello world").is_none());
        assert!(parse_say_emote("WAVE extra").is_none());
    }
}
