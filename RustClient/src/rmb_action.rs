//! RMB / modClick DROP/REMV/SWAP (L-ACT first-playtest residual).
//!
//! Full C++ `pointerDown` modClick selection lives in
//! [`crate::click_tile::select_tile_action`] / [`crate::click_tile::click_tile_mod`].
//! This module is the GUI convenience for right mouse + `Q`.

use crate::click_tile::{
    click_tile_mod, click_tile_mod_ex, our_held_id as click_our_held, ObjectClickResult,
    WalkOrUseResult,
};
use crate::move_state::MoveError;
use crate::session::ClientSession;

/// Outcome of an RMB / drop-key interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RmbClickResult {
    /// Path-to-adjacent (or immediate) object action (DROP/REMV/SWAP/USE/SREMV).
    Object(ObjectClickResult),
    /// Ground MOVE (rare under modClick; empty tile without hold may USE).
    Ground(crate::click_tile::ClickTileResult),
    /// Nothing to do.
    NoOp,
}

impl RmbClickResult {
    pub fn is_action(&self) -> bool {
        !matches!(self, RmbClickResult::NoOp)
    }

    pub fn label(&self) -> &'static str {
        match self {
            RmbClickResult::Object(r) => {
                if r.action_line.starts_with("DROP ") {
                    "DROP"
                } else if r.action_line.starts_with("REMV ") {
                    "REMV"
                } else if r.action_line.starts_with("SREMV ") {
                    "SREMV"
                } else if r.action_line.starts_with("SWAP ") {
                    "SWAP"
                } else if r.action_line.starts_with("USE ") {
                    "USE"
                } else if r.action_line.starts_with("SELF ") {
                    "SELF"
                } else {
                    "ACT"
                }
            }
            RmbClickResult::Ground(_) => "MOVE",
            RmbClickResult::NoOp => "noop",
        }
    }
}

/// True when the tile is a container we may REMV from (slots or contained ids).
pub fn tile_allows_remv(session: &ClientSession, tile_x: i32, tile_y: i32) -> bool {
    let Some(tile) = session.map.get(tile_x, tile_y) else {
        return false;
    };
    if tile.object_id <= 0 {
        return false;
    }
    if !tile.contained_ids().is_empty() {
        return true;
    }
    session
        .content
        .get(tile.object_id)
        .map(|d| d.num_slots > 0)
        .unwrap_or(false)
}

/// Our held object id (`0` empty, `<0` baby).
pub fn our_held_id(session: &ClientSession) -> i32 {
    click_our_held(session)
}

/// RMB / Q-key: full modClick action select (DROP/SWAP/REMV/USE).
///
/// // C++: LivingLifePage pointerDown right / modClick branches (~26104–26314)
/// // Playtest: holding → DROP path; bare hand on container → REMV; else modClick USE/MOVE.
pub fn click_rmb_tile(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
) -> Result<RmbClickResult, MoveError> {
    click_rmb_tile_ex(session, tile_x, tile_y, -1, -1)
}

/// Like [`click_rmb_tile`] with worn clothing slot + contained `hit_slot`.
///
/// `clothing_slot` in `0..5` → DROP held into slot / SREMV from slot (mod).
/// `hit_slot` = contained index from soft-FB or map stack (`-1` top); wires
/// SREMV `i` / REMV `i`. Prefer [`crate::hover_pick::resolve_hit_slot`] to map
/// `HoverPick::contained_slot` and/or a stack index into this argument.
///
/// // C++: `hitSlotIndex` → SREMV x y c i# via encode_sremv / LivingLifePage pointerDown
pub fn click_rmb_tile_ex(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    clothing_slot: i32,
    hit_slot: i32,
) -> Result<RmbClickResult, MoveError> {
    let held = our_held_id(session);
    // Normalize: soft-FB / stack already resolved; still clamp negatives to -1.
    let hit_slot = if hit_slot < 0 { -1 } else { hit_slot };
    // Worn clothing hit always produces a self action (even empty hands → SREMV).
    if (0..=5).contains(&clothing_slot) {
        match click_tile_mod_ex(session, tile_x, tile_y, true, hit_slot, clothing_slot)? {
            WalkOrUseResult::Object(r) => return Ok(RmbClickResult::Object(r)),
            WalkOrUseResult::Ground(r) => return Ok(RmbClickResult::Ground(r)),
        }
    }
    let tile = session.map.get(tile_x, tile_y);
    let dest_id = tile.map(|t| t.object_id).unwrap_or(0);
    // Empty hands + empty non-container tile → no wire (avoids useless USE on void).
    if held == 0 && dest_id <= 0 && !tile_allows_remv(session, tile_x, tile_y) {
        return Ok(RmbClickResult::NoOp);
    }
    match click_tile_mod(session, tile_x, tile_y, true, hit_slot)? {
        WalkOrUseResult::Object(r) => Ok(RmbClickResult::Object(r)),
        WalkOrUseResult::Ground(r) => Ok(RmbClickResult::Ground(r)),
    }
}

/// RMB with soft-FB contained + optional map stack index → resolved `hit_slot`.
///
/// Soft-FB wins when `soft_fb_contained >= 0`; else `map_stack_index` (see
/// [`crate::hover_pick::resolve_hit_slot`]).
pub fn click_rmb_tile_hit(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    clothing_slot: i32,
    soft_fb_contained: i32,
    map_stack_index: i32,
) -> Result<RmbClickResult, MoveError> {
    let hit_slot =
        crate::hover_pick::resolve_hit_slot(soft_fb_contained, map_stack_index);
    click_rmb_tile_ex(session, tile_x, tile_y, clothing_slot, hit_slot)
}

/// Session ergonomics (same pattern as [`crate::click_tile::ClickTileExt`]).
pub trait RmbClickExt {
    fn click_rmb(
        &mut self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<RmbClickResult, MoveError>;
    /// RMB with clothing slot + contained `hit_slot` (soft-FB or map stack).
    fn click_rmb_ex(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        clothing_slot: i32,
        hit_slot: i32,
    ) -> Result<RmbClickResult, MoveError>;
    fn click_drop_held(
        &mut self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<ObjectClickResult, MoveError>;
    /// Path-to-adjacent REMV with resolved `hit_slot` (`-1` top).
    fn click_remv_slot(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        hit_slot: i32,
    ) -> Result<ObjectClickResult, MoveError>;
}

impl RmbClickExt for ClientSession {
    fn click_rmb(
        &mut self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<RmbClickResult, MoveError> {
        click_rmb_tile(self, tile_x, tile_y)
    }

    fn click_rmb_ex(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        clothing_slot: i32,
        hit_slot: i32,
    ) -> Result<RmbClickResult, MoveError> {
        click_rmb_tile_ex(self, tile_x, tile_y, clothing_slot, hit_slot)
    }

    fn click_drop_held(
        &mut self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<ObjectClickResult, MoveError> {
        crate::click_tile::click_drop(self, tile_x, tile_y, -1)
    }

    fn click_remv_slot(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        hit_slot: i32,
    ) -> Result<ObjectClickResult, MoveError> {
        let slot = if hit_slot < 0 { -1 } else { hit_slot };
        crate::click_tile::click_remv(self, tile_x, tile_y, slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ClientObjectDef;
    use crate::frame::{write_message, FrameReader};
    use crate::parse::MapChunkHeader;
    use crate::session::{SessionConfig, SessionEvent};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    fn framed_text(s: &str) -> Vec<u8> {
        crate::frame::encode_raw(s).into_bytes()
    }

    fn test_cfg(port: u16) -> SessionConfig {
        SessionConfig {
            host: "127.0.0.1".into(),
            port,
            email: "user@test".into(),
            password: "secret".into(),
            account_key: "key123".into(),
            pad_email_to_80: false,
            read_timeout: Duration::from_millis(400),
            write_timeout: Duration::from_secs(2),
            ..SessionConfig::default()
        }
    }

    fn login_then_peer(bodies: Vec<Vec<u8>>) -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            write_message(&mut sock, "SN\n1/20\ntest_challenge_xyz\n184\n").unwrap();
            let mut fr = FrameReader::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = sock.read(&mut buf).unwrap();
                if n == 0 {
                    return;
                }
                let msgs = fr.push(&buf[..n]);
                if msgs
                    .into_iter()
                    .any(|m| m.starts_with("LOGIN ") || m.starts_with("RLOGIN "))
                {
                    break;
                }
            }
            write_message(&mut sock, "ACCEPTED\n").unwrap();
            for body in bodies {
                sock.write_all(&body).unwrap();
            }
            thread::sleep(Duration::from_millis(150));
        });
        (port, handle)
    }

    fn seed_open_map(session: &mut ClientSession, x: i32, y: i32, w: i32, h: i32) {
        let n = (w.max(1) * h.max(1)) as usize;
        let plain = vec!["0:0:0"; n].join(" ");
        let header = MapChunkHeader {
            size_x: w.max(1),
            size_y: h.max(1),
            x,
            y,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        session
            .map
            .apply_mc_plaintext(&header, &plain)
            .expect("seed open map");
    }

    fn drain_until_frame(session: &mut ClientSession) {
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }

    #[test]
    fn rmb_noop_when_empty_hands_empty_tile() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        seed_open_map(&mut session, 0, 0, 8, 8);
        session.move_state.x = 1;
        session.move_state.y = 1;
        let r = click_rmb_tile(&mut session, 2, 2).unwrap();
        assert_eq!(r, RmbClickResult::NoOp);
        let _ = handle.join();
    }

    #[test]
    fn rmb_drop_when_holding() {
        // held_id=33 at (5,5); MC first so bind succeeds.
        let bind_pu = "PU\n\
1 19 0 0 0 0 33 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        drain_until_frame(&mut session);
        seed_open_map(&mut session, 0, 0, 12, 12);
        assert_eq!(session.our_id, Some(1));
        assert!(our_held_id(&session) > 0, "fixture holds 33");

        let r = click_rmb_tile(&mut session, 6, 5).unwrap();
        match r {
            RmbClickResult::Object(o) => {
                assert_eq!(o.target, (6, 5));
                assert!(
                    o.action_line.starts_with("DROP 6 5 -1")
                        || o.action_line.starts_with("USE 6 5")
                        || o.action_line.starts_with("SWAP 6 5"),
                    "expected DROP/USE/SWAP wire, got {}",
                    o.action_line
                );
                assert!(o.already_adjacent || o.moved);
            }
            other => panic!("expected Object, got {other:?}"),
        }
        let _ = handle.join();
    }

    #[test]
    fn rmb_remv_on_container() {
        let bind_pu = "PU\n\
1 19 0 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        drain_until_frame(&mut session);
        seed_open_map(&mut session, 0, 0, 12, 12);
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 125,
            object_id_raw: "125,33".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        session.content.objects.insert(
            125,
            ClientObjectDef {
                id: 125,
                name: "Basket".into(),
                num_slots: 4,
                ..Default::default()
            },
        );
        assert!(tile_allows_remv(&session, 6, 5));
        assert_eq!(our_held_id(&session), 0);

        let r = click_rmb_tile(&mut session, 6, 5).unwrap();
        match r {
            RmbClickResult::Object(o) => {
                assert_eq!(o.target, (6, 5));
                assert!(
                    o.action_line.starts_with("REMV 6 5"),
                    "expected REMV, got {}",
                    o.action_line
                );
            }
            other => panic!("expected Object REMV, got {other:?}"),
        }
        let _ = handle.join();
    }

    #[test]
    fn rmb_sremv_uses_contained_hit_slot() {
        // Empty hands + clothing backpack + contained_slot 0 → SREMV x y 5 0#
        let bind_pu = "PU\n\
1 19 0 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;500 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        drain_until_frame(&mut session);
        seed_open_map(&mut session, 0, 0, 12, 12);
        session.move_state.x = 5;
        session.move_state.y = 5;
        assert_eq!(our_held_id(&session), 0);

        let r = click_rmb_tile_ex(&mut session, 5, 5, 5, 0).unwrap();
        match r {
            RmbClickResult::Object(o) => {
                assert_eq!(
                    o.action_line, "SREMV 5 5 5 0#",
                    "encode_sremv must receive contained hit_slot"
                );
            }
            other => panic!("expected SREMV Object, got {other:?}"),
        }
        let _ = handle.join();
    }

    #[test]
    fn rmb_hit_resolves_map_stack_when_no_soft_fb() {
        // soft_fb=-1, map_stack=1 → REMV x y 1# via click_rmb_tile_hit
        let bind_pu = "PU\n\
1 19 0 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        drain_until_frame(&mut session);
        seed_open_map(&mut session, 0, 0, 12, 12);
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 125,
            object_id_raw: "125,33,40".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        session.content.objects.insert(
            125,
            ClientObjectDef {
                id: 125,
                name: "Basket".into(),
                num_slots: 4,
                permanent: true,
                ..Default::default()
            },
        );
        // No soft-FB contained; stack index 1 → hit_slot 1.
        let r = click_rmb_tile_hit(&mut session, 6, 5, -1, -1, 1).unwrap();
        match r {
            RmbClickResult::Object(o) => {
                assert_eq!(o.action_line, "REMV 6 5 1#");
            }
            other => panic!("expected REMV from map stack, got {other:?}"),
        }
        // Soft-FB contained 0 wins over stack 1.
        session.player_action_pending = false;
        let r2 = click_rmb_tile_hit(&mut session, 6, 5, -1, 0, 1).unwrap();
        match r2 {
            RmbClickResult::Object(o) => {
                assert_eq!(o.action_line, "REMV 6 5 0#");
            }
            other => panic!("expected soft-FB hit_slot, got {other:?}"),
        }
        let _ = handle.join();
    }

    #[test]
    fn rmb_remv_uses_contained_hit_slot() {
        let bind_pu = "PU\n\
1 19 0 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        drain_until_frame(&mut session);
        seed_open_map(&mut session, 0, 0, 12, 12);
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 125,
            object_id_raw: "125,33,40".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        session.content.objects.insert(
            125,
            ClientObjectDef {
                id: 125,
                name: "Basket".into(),
                num_slots: 4,
                permanent: true,
                ..Default::default()
            },
        );
        let r = click_rmb_tile_ex(&mut session, 6, 5, -1, 1).unwrap();
        match r {
            RmbClickResult::Object(o) => {
                assert_eq!(o.action_line, "REMV 6 5 1#");
            }
            other => panic!("expected REMV slot 1, got {other:?}"),
        }
        let _ = handle.join();
    }

    #[test]
    fn tile_allows_remv_by_num_slots() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        seed_open_map(&mut session, 0, 0, 4, 4);
        session.map.apply_mx(&crate::parse::MapChange {
            x: 1,
            y: 1,
            floor_id: 0,
            object_id: 200,
            object_id_raw: "200".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        session.content.objects.insert(
            200,
            ClientObjectDef {
                id: 200,
                num_slots: 2,
                ..Default::default()
            },
        );
        assert!(tile_allows_remv(&session, 1, 1));
        assert!(!tile_allows_remv(&session, 0, 0));
        let _ = handle.join();
    }
}
