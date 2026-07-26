//! Post-LOGIN packet sequence mirrored from Haxe `Connection.initConnection`.
//!
//! Order after ACCEPTED: MC, TOOL_SLOTS (TS), PU, NAME (NM), FX, FRAME (FM), BAD_BIOMES (BB).

use flate2::write::ZlibEncoder;
use flate2::Compression;
use ol_protocol::{format_player_update_line, format_server_message};
use ol_world::World;
use std::io::Write;

pub const DEFAULT_PERSON_OBJECT: i32 = 19;
/// Default walk move-speed on bootstrap PU / FX (matches `ol_sim::WALK_MOVE_SPEED`).
pub const DEFAULT_WALK_MOVE_SPEED: f32 = 3.75;

/// Same formula as `ol_sim::player_id_for_conn` so PU id matches sim spawn.
pub fn player_id_for_conn(conn_id: u64) -> i32 {
    (conn_id as i32).saturating_add(1).max(2)
}

fn build_chunk_plaintext(world: &World, origin_x: i32, origin_y: i32, size_x: i32, size_y: i32) -> String {
    // Same as ol_sim::build_chunk_plaintext: Haxe biome:floor:obj with container
    // wire form base,c0,c1 via World::encode_object_for_map.
    let mut parts = Vec::with_capacity((size_x * size_y).max(0) as usize);
    for dy in 0..size_y {
        for dx in 0..size_x {
            let tx = origin_x + dx;
            let ty = origin_y + dy;
            parts.push(format!(
                "{}:{}:{}",
                world.get_biome(tx, ty),
                world.get_floor(tx, ty),
                world.encode_object_for_map(tx, ty)
            ));
        }
    }
    parts.join(" ")
}

/// Build MC from **absolute** world tiles; wire header uses **client-relative** origin.
///
/// At login, birth origin equals world spawn, so relative center is (0,0).
fn build_map_chunk_packet(
    world: &World,
    world_center_x: i32,
    world_center_y: i32,
    wire_center_x: i32,
    wire_center_y: i32,
    width: i32,
    height: i32,
) -> Vec<u8> {
    let world_origin_x = world_center_x - width / 2;
    let world_origin_y = world_center_y - height / 2;
    let wire_origin_x = wire_center_x - width / 2;
    let wire_origin_y = wire_center_y - height / 2;
    let plain = build_chunk_plaintext(world, world_origin_x, world_origin_y, width, height);
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(plain.as_bytes()).expect("zlib");
    let compressed = enc.finish().expect("zlib finish");
    let header = format!(
        "MC\n{width} {height} {wire_origin_x} {wire_origin_y}\n{} {}\n#",
        plain.len(),
        compressed.len()
    );
    let mut out = header.into_bytes();
    out.extend_from_slice(&compressed);
    out
}

/// Full post-ACCEPTED bootstrap as discrete write buffers.
///
/// `spawn_x/y` are **absolute world** tiles (same as sim birth origin). Wire PU/MC
/// use birth-relative coordinates so the client sees birth as (0,0).
pub fn build_login_bootstrap(
    conn_id: u64,
    spawn_x: i32,
    spawn_y: i32,
    age: f32,
    food: f32,
    food_max: f32,
    held_id: i32,
    world_for_chunk: &World,
) -> Vec<Vec<u8>> {
    let p_id = player_id_for_conn(conn_id);
    let mut out: Vec<Vec<u8>> = Vec::new();

    out.push(format_server_message("ACCEPTED", &[]).into_bytes());
    // Birth-relative center (0,0); sample world at absolute spawn.
    out.push(build_map_chunk_packet(
        world_for_chunk,
        spawn_x,
        spawn_y,
        0,
        0,
        32,
        30,
    ));
    out.push(format_server_message("TS", &["0 1000"]).into_bytes());

    let pu = format_player_update_line(
        p_id,
        DEFAULT_PERSON_OBJECT,
        held_id,
        0, // birth-relative self position
        0,
        age,
        DEFAULT_WALK_MOVE_SPEED,
        1, // birth done_moving_seq
    );
    out.push(format_server_message("PU", &[&pu]).into_bytes());
    let name_line = format!("{p_id} NEWBORN FAMILY");
    out.push(format_server_message("NM", &[&name_line]).into_bytes());

    let fx = format!(
        "{} {} 0 0 {:.2} -1 0 0",
        food.ceil() as i32,
        food_max as i32,
        DEFAULT_WALK_MOVE_SPEED
    );
    out.push(format_server_message("FX", &[&fx]).into_bytes());
    out.push(format_server_message("FM", &[]).into_bytes());

    // Haxe BAD_BIOMES uses real biome ids; 21 = SNOWINGREY mountain wall.
    let bb = "21 MOUNTAIN\n2 RIVER\n6 OCEAN\n";
    out.push(format_server_message("BB", &[bb]).into_bytes());

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_haxe_order_and_accepted_format() {
        let w = World::new(64, 64, false);
        let chunks = build_login_bootstrap(1, 0, 0, 14.0, 10.0, 20.0, 0, &w);
        assert_eq!(String::from_utf8_lossy(&chunks[0]), "ACCEPTED\n#");
        assert!(chunks[1].starts_with(b"MC\n"));
        let mut joined = Vec::new();
        for c in chunks.iter().skip(2) {
            joined.extend_from_slice(c);
        }
        let rest = String::from_utf8_lossy(&joined);
        assert!(rest.contains("TS\n"));
        assert!(rest.contains("PU\n"));
        assert!(rest.contains("NM\n"));
        assert!(rest.contains("FX\n"));
        assert!(rest.contains("FM\n"));
        assert!(rest.contains("BB\n"));
        assert_eq!(player_id_for_conn(1), 2);
    }
}
