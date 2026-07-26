//! Extra pure chat query formatters (COUNT, NEAR, DIST, BIOME, FLOOR, WJOURNAL, SAVE).

/// `COUNT n` online players.
pub fn format_count_query(n: usize) -> String {
    format!("COUNT {n}")
}

/// `WJOURNAL x y object_id tick` or `WJOURNAL none` (world journal peek).
///
/// Pure formatter for the last shared [`ol_world::JournalEntry`] summary.
pub fn format_wjournal_query(entry: Option<(i32, i32, i32, u64)>) -> String {
    match entry {
        Some((x, y, object_id, tick)) => format!("WJOURNAL {x} {y} {object_id} {tick}"),
        None => "WJOURNAL none".into(),
    }
}

/// Operator `SAY SAVE` body when force-save hook Arc is present (`SAVE OK`)
/// or when deferred (`SAVE deferred`). Pure — no I/O.
pub fn format_save_reply(hook_present: bool) -> String {
    if hook_present {
        "SAVE OK".into()
    } else {
        "SAVE deferred".into()
    }
}

/// Non-operator `SAY SAVE` denial body.
pub fn format_save_denied() -> String {
    "SAVE DENIED".into()
}

/// Non-operator `SAY STATS` denial body.
pub fn format_stats_denied() -> String {
    "STATS DENIED".into()
}

/// Non-operator `SAY BENCH` denial body.
pub fn format_bench_denied() -> String {
    "BENCH DENIED".into()
}

/// Operator `SAY STATS` body (server counters + sim snapshot). Pure; no I/O.
///
/// Format:
/// `STATS ticks=T intents=I skip=S conns=C logins=L deaths=D crafts=R autosaves=A online=O players=P sim_tick=K sim_time=X.XX world=WxH helpers=H chunks=N paused=0|1`
#[allow(clippy::too_many_arguments)]
pub fn format_stats_query(
    ticks: u64,
    intents: u64,
    skip: u64,
    connections: u64,
    logins: u64,
    deaths: u64,
    crafts: u64,
    autosaves: u64,
    online: usize,
    players: usize,
    sim_tick: u64,
    sim_time: f32,
    world_w: i32,
    world_h: i32,
    helpers: usize,
    chunks: usize,
    paused: bool,
) -> String {
    format!(
        "STATS ticks={ticks} intents={intents} skip={skip} conns={connections} logins={logins} deaths={deaths} crafts={crafts} autosaves={autosaves} online={online} players={players} sim_tick={sim_tick} sim_time={sim_time:.2} world={world_w}x{world_h} helpers={helpers} chunks={chunks} paused={}",
        if paused { 1 } else { 0 }
    )
}

/// Operator `SAY BENCH` body: micro-benchmark timing. Pure formatter.
///
/// Format: `BENCH us=U ops=N rate=R` where `rate` is ops/sec (integer).
pub fn format_bench_query(elapsed_us: u64, ops: u64) -> String {
    let rate = if elapsed_us == 0 {
        ops
    } else {
        ((ops as u128).saturating_mul(1_000_000) / elapsed_us as u128) as u64
    };
    format!("BENCH us={elapsed_us} ops={ops} rate={rate}")
}

/// Fixed op count for `SAY BENCH` pure CPU sample (chebyshev loops).
pub const BENCH_OPS: u64 = 50_000;

/// Run a short pure CPU sample (no I/O, no world locks) and return (elapsed_us, ops).
///
/// Uses Chebyshev distance math — cheap, deterministic work for operator diagnostics.
pub fn run_bench_sample() -> (u64, u64) {
    use std::time::Instant;
    let t0 = Instant::now();
    let mut acc = 0i32;
    let n = BENCH_OPS as i32;
    for i in 0..n {
        acc = acc.wrapping_add(chebyshev(i, i / 2, i / 3, i / 4));
    }
    // Prevent optimizer from dropping the loop entirely.
    std::hint::black_box(acc);
    let us = t0.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    (us, BENCH_OPS)
}

/// `NEAR id id …` or `NEAR none`.
pub fn format_near_query(ids: &[i32]) -> String {
    if ids.is_empty() {
        "NEAR none".into()
    } else {
        let s = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        format!("NEAR {s}")
    }
}

/// Chebyshev distance.
pub fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// `DIST p_id d` or `DIST FAIL`.
pub fn format_dist_query(target: i32, dist: Option<i32>) -> String {
    match dist {
        Some(d) => format!("DIST {target} {d}"),
        None => format!("DIST {target} FAIL"),
    }
}

/// `BIOME id` optional name.
pub fn format_biome_query(biome: u8, name: &str) -> String {
    format_biome_query_with_hex(biome, name, None)
}

/// `BIOME id [name] [RRGGBB]` — optional map-PNG hex from `biome_colors`.
pub fn format_biome_query_with_hex(biome: u8, name: &str, hex: Option<&str>) -> String {
    let hex = hex.filter(|h| !h.is_empty());
    match (name.is_empty(), hex) {
        (true, None) => format!("BIOME {biome}"),
        (false, None) => format!("BIOME {biome} {name}"),
        (true, Some(h)) => format!("BIOME {biome} {h}"),
        (false, Some(h)) => format!("BIOME {biome} {name} {h}"),
    }
}

/// `FLOOR id`
pub fn format_floor_query(floor: u16) -> String {
    format!("FLOOR {floor}")
}

/// Common biome id names (OHOL-ish).
pub fn biome_name(id: u8) -> &'static str {
    match id {
        0 => "grassland",
        1 => "swamp",
        2 => "yellow",
        3 => "gray",
        4 => "snow",
        5 => "desert",
        6 => "jungle",
        9 => "ocean",
        17 => "river",
        21 => "mountain",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatters() {
        assert_eq!(format_count_query(3), "COUNT 3");
        assert_eq!(format_count_query(0), "COUNT 0");
        assert_eq!(format_near_query(&[]), "NEAR none");
        assert_eq!(format_near_query(&[2, 5]), "NEAR 2 5");
        assert_eq!(format_near_query(&[1]), "NEAR 1");
        assert_eq!(chebyshev(0, 0, 3, 4), 4);
        assert_eq!(chebyshev(10, 10, 10, 10), 0);
        assert_eq!(chebyshev(-2, 5, 1, 1), 4);
        assert_eq!(format_dist_query(9, Some(4)), "DIST 9 4");
        assert_eq!(format_dist_query(9, None), "DIST 9 FAIL");
        assert_eq!(format_biome_query(5, "desert"), "BIOME 5 desert");
        assert_eq!(format_biome_query(99, ""), "BIOME 99");
        assert_eq!(
            format_biome_query_with_hex(5, "desert", Some("DBAC4D")),
            "BIOME 5 desert DBAC4D"
        );
        assert_eq!(
            format_biome_query_with_hex(99, "", Some("AABBCC")),
            "BIOME 99 AABBCC"
        );
        assert_eq!(format_biome_query_with_hex(1, "swamp", None), "BIOME 1 swamp");
        assert_eq!(format_biome_query_with_hex(1, "swamp", Some("")), "BIOME 1 swamp");
        assert_eq!(format_floor_query(2), "FLOOR 2");
        assert_eq!(format_floor_query(0), "FLOOR 0");
        assert_eq!(format_wjournal_query(None), "WJOURNAL none");
        assert_eq!(
            format_wjournal_query(Some((10, -3, 391, 42))),
            "WJOURNAL 10 -3 391 42"
        );
        assert_eq!(format_save_reply(true), "SAVE OK");
        assert_eq!(format_save_reply(false), "SAVE deferred");
        assert_eq!(format_save_denied(), "SAVE DENIED");
        assert_eq!(format_stats_denied(), "STATS DENIED");
        assert_eq!(format_bench_denied(), "BENCH DENIED");
        let stats = format_stats_query(
            10, 20, 1, 2, 3, 4, 5, 6, 2, 3, 99, 12.5, 500, 500, 100, 50, false,
        );
        assert!(stats.starts_with("STATS ticks=10 "), "got {stats}");
        assert!(stats.contains("intents=20"), "got {stats}");
        assert!(stats.contains("online=2"), "got {stats}");
        assert!(stats.contains("world=500x500"), "got {stats}");
        assert!(stats.contains("paused=0"), "got {stats}");
        assert_eq!(format_bench_query(0, 1000), "BENCH us=0 ops=1000 rate=1000");
        assert_eq!(
            format_bench_query(1_000_000, 50_000),
            "BENCH us=1000000 ops=50000 rate=50000"
        );
        let (us, ops) = run_bench_sample();
        assert_eq!(ops, BENCH_OPS);
        // Sample should finish quickly on any modern machine.
        assert!(us < 5_000_000, "bench sample too slow: {us}us");
    }

    #[test]
    fn biome_names_known() {
        assert_eq!(biome_name(0), "grassland");
        assert_eq!(biome_name(4), "snow");
        assert_eq!(biome_name(5), "desert");
        assert_eq!(biome_name(6), "jungle");
        assert_eq!(biome_name(9), "ocean");
        assert_eq!(biome_name(17), "river");
        assert_eq!(biome_name(21), "mountain");
        assert_eq!(biome_name(255), "");
    }
}
