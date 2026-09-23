//! Wire-side vanilla object-id remap (Haxe `mapIdToVanillaId` on MX / MC).
//!
//! World storage stays raw OpenLife ids. Only vanilla (non-OpenLife) clients
//! receive patched floor + object ids. OpenLife clients
//! (`client_tag` contains `OpenLifeClientName`) keep raw ids.
//!
//! Haxe: `Connection.sendMapUpdate` / `sendMapUpdateForMoving` /
//! `WorldMap.getChunk(..., patchIds: !isOpenLifeClient)`.

use crate::SimState;
use ol_net::OutboundHub;
use ol_protocol::{format_map_change, format_map_change_moving};
use ol_content::map_object_id_string;

/// Haxe `isOpenLifeClient` — `client_tag.indexOf(OpenLifeClientName) != -1`.
// Haxe: Connection.login isOpenLifeClient
pub fn is_open_life_client(client_tag: &str, open_life_client_name: &str) -> bool {
    let name = open_life_client_name.trim();
    if name.is_empty() {
        return false;
    }
    client_tag.contains(name)
}

/// True when this connection should receive remapped ids.
pub fn should_patch_conn(state: &SimState, conn_id: u64) -> bool {
    if state.last_vanilla_id < 1 {
        return false;
    }
    let Some(p) = state.players.get(&conn_id) else {
        return true;
    };
    !is_open_life_client(&p.client_tag, &state.open_life_client_name)
}

/// True when this connection is the original client (not an Open Life client).
fn vanilla_client(state: &SimState, conn_id: u64) -> bool {
    match state.players.get(&conn_id) {
        Some(p) => !is_open_life_client(&p.client_tag, &state.open_life_client_name),
        // No tag yet: same default as `should_patch_conn` (treat as vanilla).
        None => true,
    }
}

/// Object id safe for this connection's client.
///
/// Open Life clients keep raw ids. Vanilla clients use `mapIdToVanillaId` when
/// `last_vanilla_id` is set. When that cutoff is unset, synthetic multi-use
/// dummy ids are still collapsed to their parent: the original client crashes
/// in `getObject(id)->isStatue` while applying a map chunk that contains one.
// Haxe: Server.mapIdToVanillaId; Connection.sendMapUpdate patchIds
pub fn map_obj_id_for_conn(state: &SimState, conn_id: u64, id: i32) -> i32 {
    if !vanilla_client(state, conn_id) {
        return id;
    }
    if state.last_vanilla_id >= 1 {
        return state.content.map_id_to_vanilla_id(id, state.last_vanilla_id);
    }
    state.content.resolve_base_id(id)
}

/// Patch floor + object ids for a vanilla viewer.
pub fn map_floor_obj_for_conn(state: &SimState, conn_id: u64, floor: i32, obj: i32) -> (i32, i32) {
    (
        map_obj_id_for_conn(state, conn_id, floor),
        map_obj_id_for_conn(state, conn_id, obj),
    )
}

/// Rebuild an MX packet's floor/object fields with `map_id`.
pub fn patch_mx_packet(pkt: &[u8], map_id: impl Fn(i32) -> i32) -> Vec<u8> {
    let Ok(s) = std::str::from_utf8(pkt) else {
        return pkt.to_vec();
    };
    if !s.starts_with("MX\n") {
        return pkt.to_vec();
    }
    let rest = &s[3..];
    let line = rest.split('\n').next().unwrap_or("");
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return pkt.to_vec();
    }
    let x = parts[0];
    let y = parts[1];
    let floor = map_id(parts[2].parse().unwrap_or(0));
    let obj = map_id(parts[3].parse().unwrap_or(0));
    let tail = parts[4..].join(" ");
    format!("MX\n{x} {y} {floor} {obj} {tail}\n#").into_bytes()
}

fn send_one(outbound: &OutboundHub, cid: u64, packet: Vec<u8>, urgent: bool) {
    if urgent {
        outbound.send_urgent(cid, packet);
    } else {
        outbound.send(cid, packet);
    }
}

/// Fan a packet to connections; MX ids are remapped per vanilla client.
pub fn send_nearby_maybe_mx(
    state: &SimState,
    outbound: &OutboundHub,
    conn_ids: &[u64],
    packet: Vec<u8>,
    urgent: bool,
) {
    if !packet.starts_with(b"MX\n") {
        for &cid in conn_ids {
            send_one(outbound, cid, packet.clone(), urgent);
        }
        return;
    }
    for &cid in conn_ids {
        let pkt = if vanilla_client(state, cid) {
            patch_mx_packet(&packet, |id| map_obj_id_for_conn(state, cid, id))
        } else {
            packet.clone()
        };
        send_one(outbound, cid, pkt, urgent);
    }
}

/// Format MX with per-viewer vanilla remap.
pub fn format_map_change_for_conn(
    state: &SimState,
    conn_id: u64,
    x: i32,
    y: i32,
    floor: i32,
    obj: i32,
    player_id: i32,
) -> String {
    let (floor, obj) = map_floor_obj_for_conn(state, conn_id, floor, obj);
    format_map_change(x, y, floor, obj, player_id)
}

/// Format moving MX with per-viewer vanilla remap.
pub fn format_map_change_moving_for_conn(
    state: &SimState,
    conn_id: u64,
    x: i32,
    y: i32,
    floor: i32,
    obj: i32,
    player_id: i32,
    old_x: i32,
    old_y: i32,
    speed: f32,
) -> String {
    let (floor, obj) = map_floor_obj_for_conn(state, conn_id, floor, obj);
    format_map_change_moving(x, y, floor, obj, player_id, old_x, old_y, speed)
}

/// Vanilla-remap floor + container-format object string for one connection.
pub fn map_floor_obj_str_for_conn(
    state: &SimState,
    conn_id: u64,
    floor: i32,
    obj: &str,
) -> (i32, String) {
    if !should_patch_conn(state, conn_id) {
        return (floor, obj.to_string());
    }
    let last_v = state.last_vanilla_id;
    let floor = map_obj_id_for_conn(state, conn_id, floor);
    let obj = map_object_id_string(obj, |id| {
        state.content.map_id_to_vanilla_id(id, last_v)
    });
    (floor, obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Player, SimState};
    use ol_content::ContentDb;
    use ol_protocol::format_map_change;
    use std::sync::Arc;

    fn state_with_map() -> SimState {
        let mut db = ContentDb::default();
        db.last_open_life_id = 200;
        db.vanilla_obj_id_map.insert(150, 33);
        let mut state = SimState::with_default_empty(Arc::new(db));
        state.last_vanilla_id = 100;
        state.open_life_client_name = "OpenLife".into();
        state
    }

    #[test]
    fn open_life_client_name_substring() {
        assert!(is_open_life_client("OpenLife-dev", "OpenLife"));
        assert!(is_open_life_client("fooOpenLifeBar", "OpenLife"));
        assert!(!is_open_life_client("client_official", "OpenLife"));
        assert!(!is_open_life_client("OpenLife-dev", ""));
    }

    #[test]
    fn vanilla_client_maps_openlife_id() {
        let mut state = state_with_map();
        let mut p = Player::new(2, 1, "v@t");
        p.client_tag = "client_official".into();
        state.players.insert(1, p);
        assert_eq!(map_obj_id_for_conn(&state, 1, 150), 33);
        assert_eq!(map_obj_id_for_conn(&state, 1, 50), 50);
        assert_eq!(map_obj_id_for_conn(&state, 1, 160), 0);
        assert_eq!(map_obj_id_for_conn(&state, 1, 250), 150);
    }

    #[test]
    fn open_life_client_keeps_raw_ids() {
        let mut state = state_with_map();
        let mut p = Player::new(3, 2, "ol@t");
        p.client_tag = "OpenLifeClient".into();
        state.players.insert(2, p);
        assert_eq!(map_obj_id_for_conn(&state, 2, 150), 150);
        assert_eq!(map_obj_id_for_conn(&state, 2, 160), 160);
        assert!(!should_patch_conn(&state, 2));
    }

    #[test]
    fn mapping_off_keeps_real_ids_and_hides_dummies() {
        let mut state = state_with_map();
        state.last_vanilla_id = -1;
        let mut db = (*state.content).clone();
        db.dummy_parent.insert(9001, 418);
        state.content = Arc::new(db);
        let mut p = Player::new(2, 1, "v@t");
        p.client_tag = "client_official".into();
        state.players.insert(1, p);
        // Real ids still pass through when the vanilla cutoff is unset.
        assert_eq!(map_obj_id_for_conn(&state, 1, 150), 150);
        // Synthetic multi-use id would null-deref getObject on the original client.
        assert_eq!(map_obj_id_for_conn(&state, 1, 9001), 418);
    }

    #[test]
    fn patch_mx_rewrites_floor_and_object() {
        let raw = format_map_change(3, 4, 150, 160, -1).into_bytes();
        let patched = patch_mx_packet(&raw, |id| if id == 150 { 33 } else if id == 160 { 0 } else { id });
        let s = String::from_utf8(patched).unwrap();
        assert!(s.starts_with("MX\n"));
        assert!(s.contains("3 4 33 0 -1"), "got {s}");
        assert!(!s.contains(" 150 "));
    }
}
