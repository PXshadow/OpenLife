//! Object-interaction and keepalive client→server messages.
//!
//! Wire forms match `server/protocol.txt` and LivingLifePage.cpp send paths:
//! ```text
//! autoSprintf( "%s %d %d%s#", action, sendX(x), sendY(y), extra );
//! ```
//! Exactly one space between tokens; trailing `#`; no leading spaces.

/// `KA x y#` — keepalive; x/y ignored by server but required (client uses 0 0).
pub fn encode_ka(x: i32, y: i32) -> String {
    format!("KA {x} {y}#")
}

/// `USE x y#` or `USE x y id#` or `USE x y id i#`.
///
/// Official client always appends `id` when the click target has `destID > 0`
/// (LivingLifePage ~26302–26313). Optional `i` is a container slot for
/// `+useOnContained` targets.
pub fn encode_use(x: i32, y: i32, object_id: Option<i32>, slot_index: Option<i32>) -> String {
    match (object_id, slot_index) {
        (Some(id), Some(i)) => format!("USE {x} {y} {id} {i}#"),
        (Some(id), None) => format!("USE {x} {y} {id}#"),
        (None, Some(i)) => {
            // Protocol lists optional id then i; without id, still send bare USE x y
            // (server does not document USE without id but with i alone).
            let _ = i;
            format!("USE {x} {y}#")
        }
        (None, None) => format!("USE {x} {y}#"),
    }
}

/// `DROP x y c#` — c is **-1** for ground/container add; 0..5 for own clothing slots.
///
/// Official client always sends the third param (`extra = " -1"` for ground DROP).
pub fn encode_drop(x: i32, y: i32, clothing_slot: i32) -> String {
    format!("DROP {x} {y} {clothing_slot}#")
}

/// `REMV x y i#` — i is container slot index, or **-1** for top of stack.
///
/// Bare-hand pickup of non-permanent ground objects often uses `REMV x y 0`
/// (LivingLifePage when modClick false + bareHandTrans leaves something).
pub fn encode_remv(x: i32, y: i32, slot_index: i32) -> String {
    format!("REMV {x} {y} {slot_index}#")
}

/// `SELF x y i#` — eat held food / clothing on self.
///
/// `i` = clothing slot (0=hat … 5=backpack), or **-1** when unused (eat / no slot).
pub fn encode_self(x: i32, y: i32, clothing_slot: i32) -> String {
    format!("SELF {x} {y} {clothing_slot}#")
}

/// `SREMV x y c i#` — remove from worn clothing container.
///
/// `c` clothing slot 0..5; `i` contained index or -1 for top.
pub fn encode_sremv(x: i32, y: i32, clothing_slot: i32, slot_index: i32) -> String {
    format!("SREMV {x} {y} {clothing_slot} {slot_index}#")
}

/// `SWAP x y#` — swap held object with ground object / container itself.
pub fn encode_swap(x: i32, y: i32) -> String {
    format!("SWAP {x} {y}#")
}

/// `BABY x y#` or `BABY x y id#` — pick up a baby.
pub fn encode_baby(x: i32, y: i32, player_id: Option<i32>) -> String {
    match player_id {
        Some(id) => format!("BABY {x} {y} {id}#"),
        None => format!("BABY {x} {y}#"),
    }
}

/// `UBABY x y i#` or `UBABY x y i id#` — use held item on other player (feed/heal/clothe).
pub fn encode_ubaby(x: i32, y: i32, clothing_slot: i32, player_id: Option<i32>) -> String {
    match player_id {
        Some(id) => format!("UBABY {x} {y} {clothing_slot} {id}#"),
        None => format!("UBABY {x} {y} {clothing_slot}#"),
    }
}

/// `FORCE x y#` — acknowledge a forced-pos PU.
pub fn encode_force(x: i32, y: i32) -> String {
    format!("FORCE {x} {y}#")
}

/// `JUMP x y#` — baby jump-out of arms / ground wiggle (protocol ignores x/y).
///
/// C++ `LivingLifePage::pointerDown` sends `"JUMP 0 0#"` when held by adult or
/// `age < noMoveAge` (0.20). MOVE must not be used for jump-out.
pub fn encode_jump(x: i32, y: i32) -> String {
    format!("JUMP {x} {y}#")
}

/// `SAY x y text#` — spoken text; x/y ignored by server but required (usually 0 0).
///
/// Text must not contain `#`. Official client truncates by age speech limit;
/// headless sends the string as provided (caller truncates if needed).
pub fn encode_say(x: i32, y: i32, text: &str) -> String {
    let text = text.replace('#', " ");
    format!("SAY {x} {y} {text}#")
}

/// `EMOT x y emotIndex#` — request temporary emotion display (server broadcasts PE).
///
/// // C++: LivingLifePage ~27086 `autoSprintf( "EMOT 0 0 %d#", emotIndex )`
/// x/y ignored (usually 0 0). Index is row in `emotionWords` / `emotionObjects`.
pub fn encode_emot(x: i32, y: i32, emot_index: i32) -> String {
    format!("EMOT {x} {y} {emot_index}#")
}

/// High-level object action kinds the headless client can queue/send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectAction {
    Use {
        x: i32,
        y: i32,
        object_id: Option<i32>,
        slot: Option<i32>,
    },
    Drop {
        x: i32,
        y: i32,
        clothing_slot: i32,
    },
    Remv {
        x: i32,
        y: i32,
        slot: i32,
    },
    SelfAct {
        x: i32,
        y: i32,
        clothing_slot: i32,
    },
    Sremv {
        x: i32,
        y: i32,
        clothing_slot: i32,
        slot: i32,
    },
    Swap {
        x: i32,
        y: i32,
    },
    Baby {
        x: i32,
        y: i32,
        player_id: Option<i32>,
    },
    Ubaby {
        x: i32,
        y: i32,
        clothing_slot: i32,
        player_id: Option<i32>,
    },
}

impl ObjectAction {
    /// Encode exactly as LivingLifePage would put in `nextActionMessageToSend`.
    pub fn encode(&self) -> String {
        match self {
            Self::Use {
                x,
                y,
                object_id,
                slot,
            } => encode_use(*x, *y, *object_id, *slot),
            Self::Drop {
                x,
                y,
                clothing_slot,
            } => encode_drop(*x, *y, *clothing_slot),
            Self::Remv { x, y, slot } => encode_remv(*x, *y, *slot),
            Self::SelfAct {
                x,
                y,
                clothing_slot,
            } => encode_self(*x, *y, *clothing_slot),
            Self::Sremv {
                x,
                y,
                clothing_slot,
                slot,
            } => encode_sremv(*x, *y, *clothing_slot, *slot),
            Self::Swap { x, y } => encode_swap(*x, *y),
            Self::Baby { x, y, player_id } => encode_baby(*x, *y, *player_id),
            Self::Ubaby {
                x,
                y,
                clothing_slot,
                player_id,
            } => encode_ubaby(*x, *y, *clothing_slot, *player_id),
        }
    }

    /// Target tile for adjacency / walk-then-act logic.
    pub fn target_xy(&self) -> (i32, i32) {
        match self {
            Self::Use { x, y, .. }
            | Self::Drop { x, y, .. }
            | Self::Remv { x, y, .. }
            | Self::SelfAct { x, y, .. }
            | Self::Sremv { x, y, .. }
            | Self::Swap { x, y }
            | Self::Baby { x, y, .. }
            | Self::Ubaby { x, y, .. } => (*x, *y),
        }
    }

    /// True for USE/DROP/REMV which protocol says are ignored mid-MOVE.
    pub fn blocked_while_moving(&self) -> bool {
        matches!(
            self,
            Self::Use { .. } | Self::Drop { .. } | Self::Remv { .. } | Self::Sremv { .. }
                | Self::Swap { .. } | Self::SelfAct { .. } | Self::Baby { .. }
                | Self::Ubaby { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ka_format() {
        assert_eq!(encode_ka(0, 0), "KA 0 0#");
    }

    #[test]
    fn use_matches_cpp_variants() {
        // Distance / bare: USE x y#
        assert_eq!(encode_use(3, 4, None, None), "USE 3 4#");
        // Clicked object: USE x y id#  (LivingLifePage extra = " %d")
        assert_eq!(encode_use(3, 4, Some(99), None), "USE 3 4 99#");
        // useOnContained: USE x y id i#
        assert_eq!(encode_use(3, 4, Some(99), Some(1)), "USE 3 4 99 1#");
    }

    #[test]
    fn drop_always_has_c_param_like_cpp() {
        // extra = " -1" for ground DROP
        assert_eq!(encode_drop(1, 2, -1), "DROP 1 2 -1#");
        assert_eq!(encode_drop(0, 0, 0), "DROP 0 0 0#"); // hat clothing
    }

    #[test]
    fn drop_clothing_slots_0_through_5() {
        // protocol: 0=hat … 5=backpack
        for c in 0..=5 {
            assert_eq!(
                encode_drop(10, 20, c),
                format!("DROP 10 20 {c}#"),
                "DROP clothing slot {c}"
            );
            assert_eq!(
                ObjectAction::Drop {
                    x: 10,
                    y: 20,
                    clothing_slot: c,
                }
                .encode(),
                format!("DROP 10 20 {c}#")
            );
        }
    }

    #[test]
    fn self_and_sremv_clothing_wire() {
        assert_eq!(encode_self(3, 4, -1), "SELF 3 4 -1#");
        for c in 0..=5 {
            assert_eq!(encode_self(0, 0, c), format!("SELF 0 0 {c}#"));
            assert_eq!(
                encode_sremv(0, 0, c, -1),
                format!("SREMV 0 0 {c} -1#")
            );
            assert_eq!(
                ObjectAction::Sremv {
                    x: 0,
                    y: 0,
                    clothing_slot: c,
                    slot: 0,
                }
                .encode(),
                format!("SREMV 0 0 {c} 0#")
            );
        }
    }

    #[test]
    fn remv_self_sremv_swap_baby_ubaby() {
        assert_eq!(encode_remv(5, 6, -1), "REMV 5 6 -1#");
        assert_eq!(encode_remv(5, 6, 0), "REMV 5 6 0#");
        assert_eq!(encode_self(0, 0, -1), "SELF 0 0 -1#");
        assert_eq!(encode_sremv(1, 2, 0, -1), "SREMV 1 2 0 -1#");
        assert_eq!(encode_swap(3, 4), "SWAP 3 4#");
        assert_eq!(encode_baby(1, 1, None), "BABY 1 1#");
        assert_eq!(encode_baby(1, 1, Some(42)), "BABY 1 1 42#");
        assert_eq!(encode_ubaby(1, 1, -1, None), "UBABY 1 1 -1#");
        assert_eq!(encode_ubaby(1, 1, 0, Some(9)), "UBABY 1 1 0 9#");
        assert_eq!(encode_force(10, 20), "FORCE 10 20#");
        assert_eq!(encode_jump(0, 0), "JUMP 0 0#");
        assert_eq!(encode_jump(1, 2), "JUMP 1 2#");
    }

    #[test]
    fn object_action_encode_roundtrip_cpp_template() {
        // "%s %d %d%s#" with action USE and extra " 99"
        let a = ObjectAction::Use {
            x: 10,
            y: 20,
            object_id: Some(99),
            slot: None,
        };
        assert_eq!(a.encode(), "USE 10 20 99#");
        let d = ObjectAction::Drop {
            x: 10,
            y: 20,
            clothing_slot: -1,
        };
        assert_eq!(d.encode(), "DROP 10 20 -1#");
    }

    #[test]
    fn say_format() {
        assert_eq!(encode_say(0, 0, "HELLO"), "SAY 0 0 HELLO#");
        assert_eq!(encode_say(0, 0, "HI#THERE"), "SAY 0 0 HI THERE#");
    }

    #[test]
    fn emot_format() {
        // C++: EMOT 0 0 %d#
        assert_eq!(encode_emot(0, 0, 0), "EMOT 0 0 0#");
        assert_eq!(encode_emot(0, 0, 12), "EMOT 0 0 12#");
        assert_eq!(encode_emot(1, 2, 5), "EMOT 1 2 5#");
    }

    #[test]
    fn single_space_no_trailing_junk() {
        for s in [
            encode_ka(0, 0),
            encode_use(1, 1, None, None),
            encode_use(1, 1, Some(5), Some(0)),
            encode_drop(0, 0, -1),
            encode_remv(0, 0, 0),
            encode_self(0, 0, -1),
            encode_sremv(0, 0, 5, -1),
            encode_swap(0, 0),
            encode_baby(0, 0, Some(1)),
            encode_ubaby(0, 0, -1, Some(2)),
            encode_force(0, 0),
            encode_emot(0, 0, 3),
        ] {
            assert!(s.ends_with('#'), "{s}");
            assert!(!s.contains("  "), "double space in {s}");
            assert!(!s.contains('\n'), "{s}");
            // No leading space
            assert!(!s.starts_with(' '), "{s}");
        }
    }
}
