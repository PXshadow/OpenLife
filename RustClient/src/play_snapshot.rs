//! Play-point snapshot tool for AI + human playtest.
//!
//! Capture a structured dump of session/world/map state at a moment in play
//! (login, after MOVE, before USE, death, …). Loadable from disk for CI/AI
//! regression notes. Soft-FB: F9 or on-screen SNAP when `settings.debug`.
//! CLI: `ohol-headless --snapshot PATH` (live) or `--snapshot-self-check`.
//!
//! Format: versioned key=value text (no new crates). Default dir: `logs/snapshots/`.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::session::ClientSession;

/// Magic / format version for on-disk snapshots.
pub const SNAPSHOT_FORMAT: u32 = 1;
pub const SNAPSHOT_MAGIC: &str = "OHOL_PLAY_SNAPSHOT";

/// Default directory under cwd for GUI/CLI dumps.
pub const DEFAULT_SNAPSHOT_DIR: &str = "logs/snapshots";

/// Optional extras from soft-FB (camera, last action line, hover).
#[derive(Debug, Clone, Default)]
pub struct SnapshotViewExtras {
    pub label: String,
    pub camera_x: f32,
    pub camera_y: f32,
    pub camera_zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub hover_tile_x: i32,
    pub hover_tile_y: i32,
    pub hover_object_id: i32,
    pub last_status: String,
    pub screen: String,
}

/// Structured play-point capture.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlaySnapshot {
    pub format: u32,
    pub created_unix: u64,
    pub label: String,
    pub our_id: Option<i32>,
    pub display_id: i32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub pos_x: f64,
    pub pos_y: f64,
    pub age: f32,
    pub held_id: i32,
    pub in_motion: bool,
    pub food_store: i32,
    pub food_capacity: i32,
    pub heat: f32,
    pub map_cells: usize,
    pub player_count: usize,
    pub content_objects: usize,
    pub data_version: i32,
    pub camera_x: f32,
    pub camera_y: f32,
    pub camera_zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub hover_tile_x: i32,
    pub hover_tile_y: i32,
    pub hover_object_id: i32,
    pub last_status: String,
    pub screen: String,
    /// Nearby map object ids (stable order: y then x), compact `x,y:id;…`
    pub map_near: String,
    /// Other players: `id@x,y;…`
    pub others: String,
    pub notes: String,
}

impl PlaySnapshot {
    /// Capture from a live session (+ optional GUI extras).
    pub fn capture(session: &ClientSession, extras: &SnapshotViewExtras) -> Self {
        let mut snap = Self {
            format: SNAPSHOT_FORMAT,
            created_unix: unix_now(),
            label: if extras.label.is_empty() {
                "play".into()
            } else {
                extras.label.clone()
            },
            our_id: session.our_id,
            camera_x: extras.camera_x,
            camera_y: extras.camera_y,
            camera_zoom: extras.camera_zoom,
            pan_x: extras.pan_x,
            pan_y: extras.pan_y,
            hover_tile_x: extras.hover_tile_x,
            hover_tile_y: extras.hover_tile_y,
            hover_object_id: extras.hover_object_id,
            last_status: extras.last_status.clone(),
            screen: extras.screen.clone(),
            content_objects: session.content.objects.len(),
            data_version: session.content.data_version,
            map_cells: session.map.len(),
            player_count: session.world.len(),
            in_motion: session.move_state.in_motion,
            pos_x: session.move_state.current_pos_x,
            pos_y: session.move_state.current_pos_y,
            ..Default::default()
        };

        if let Some(f) = &session.food {
            snap.food_store = f.food_store;
            snap.food_capacity = f.food_capacity;
        }
        if let Some(h) = &session.heat {
            snap.heat = h.heat;
        }

        if let Some(oid) = session.our_id {
            if let Some(p) = session.world.get(oid) {
                snap.display_id = p.display_id;
                snap.name = p.name.clone().unwrap_or_default();
                snap.x = p.x;
                snap.y = p.y;
                snap.age = p.age;
                snap.held_id = p.held_id;
            } else {
                snap.x = session.move_state.x;
                snap.y = session.move_state.y;
            }
        }

        // Nearby map cells (±4).
        let cx = snap.x;
        let cy = snap.y;
        let mut near = Vec::new();
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let tx = cx + dx;
                let ty = cy + dy;
                if let Some(cell) = session.map.get(tx, ty) {
                    if cell.object_id != 0 {
                        near.push(format!("{tx},{ty}:{}", cell.object_id));
                    }
                }
            }
        }
        snap.map_near = near.join(";");

        let mut others = Vec::new();
        for id in session.world.living_ids() {
            if Some(id) == session.our_id {
                continue;
            }
            if let Some(p) = session.world.get(id) {
                others.push(format!("{}@{},{}", id, p.x, p.y));
            }
        }
        snap.others = others.join(";");

        snap
    }

    /// Synthetic fixture for unit tests / CLI self-check (no server).
    pub fn synthetic_fixture() -> Self {
        Self {
            format: SNAPSHOT_FORMAT,
            created_unix: 1_700_000_000,
            label: "fixture".into(),
            our_id: Some(42),
            display_id: 19,
            name: "TEST".into(),
            x: 10,
            y: -3,
            pos_x: 10.25,
            pos_y: -3.0,
            age: 14.5,
            held_id: 33,
            in_motion: false,
            food_store: 8,
            food_capacity: 12,
            heat: 0.55,
            map_cells: 64,
            player_count: 2,
            content_objects: 100,
            data_version: 437,
            camera_x: 10.0,
            camera_y: -3.0,
            camera_zoom: 32.0,
            map_near: "10,-3:33;11,-3:0".into(),
            others: "7@12,-3".into(),
            screen: "Playing".into(),
            last_status: "ready".into(),
            notes: "synthetic".into(),
            ..Default::default()
        }
    }

    pub fn serialize(&self) -> String {
        let mut s = String::with_capacity(1024);
        let _ = writeln!(s, "{SNAPSHOT_MAGIC} format={SNAPSHOT_FORMAT}");
        let _ = writeln!(s, "created_unix={}", self.created_unix);
        let _ = writeln!(s, "label={}", escape_val(&self.label));
        let _ = writeln!(
            s,
            "our_id={}",
            self.our_id.map(|i| i.to_string()).unwrap_or_else(|| "-".into())
        );
        let _ = writeln!(s, "display_id={}", self.display_id);
        let _ = writeln!(s, "name={}", escape_val(&self.name));
        let _ = writeln!(s, "x={}", self.x);
        let _ = writeln!(s, "y={}", self.y);
        let _ = writeln!(s, "pos_x={:.4}", self.pos_x);
        let _ = writeln!(s, "pos_y={:.4}", self.pos_y);
        let _ = writeln!(s, "age={:.3}", self.age);
        let _ = writeln!(s, "held_id={}", self.held_id);
        let _ = writeln!(s, "in_motion={}", if self.in_motion { 1 } else { 0 });
        let _ = writeln!(s, "food_store={}", self.food_store);
        let _ = writeln!(s, "food_capacity={}", self.food_capacity);
        let _ = writeln!(s, "heat={:.4}", self.heat);
        let _ = writeln!(s, "map_cells={}", self.map_cells);
        let _ = writeln!(s, "player_count={}", self.player_count);
        let _ = writeln!(s, "content_objects={}", self.content_objects);
        let _ = writeln!(s, "data_version={}", self.data_version);
        let _ = writeln!(s, "camera_x={:.3}", self.camera_x);
        let _ = writeln!(s, "camera_y={:.3}", self.camera_y);
        let _ = writeln!(s, "camera_zoom={:.3}", self.camera_zoom);
        let _ = writeln!(s, "pan_x={:.3}", self.pan_x);
        let _ = writeln!(s, "pan_y={:.3}", self.pan_y);
        let _ = writeln!(s, "hover_tile_x={}", self.hover_tile_x);
        let _ = writeln!(s, "hover_tile_y={}", self.hover_tile_y);
        let _ = writeln!(s, "hover_object_id={}", self.hover_object_id);
        let _ = writeln!(s, "last_status={}", escape_val(&self.last_status));
        let _ = writeln!(s, "screen={}", escape_val(&self.screen));
        let _ = writeln!(s, "map_near={}", escape_val(&self.map_near));
        let _ = writeln!(s, "others={}", escape_val(&self.others));
        let _ = writeln!(s, "notes={}", escape_val(&self.notes));
        s
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let mut snap = Self {
            format: SNAPSHOT_FORMAT,
            ..Default::default()
        };
        let mut saw_magic = false;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with(SNAPSHOT_MAGIC) {
                saw_magic = true;
                if let Some(rest) = line.split_once("format=") {
                    if let Ok(n) = rest.1.trim().parse() {
                        snap.format = n;
                    }
                }
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let k = k.trim();
            let v = unescape_val(v.trim());
            match k {
                "created_unix" => snap.created_unix = v.parse().unwrap_or(0),
                "label" => snap.label = v,
                "our_id" => {
                    snap.our_id = if v == "-" {
                        None
                    } else {
                        v.parse().ok()
                    }
                }
                "display_id" => snap.display_id = v.parse().unwrap_or(0),
                "name" => snap.name = v,
                "x" => snap.x = v.parse().unwrap_or(0),
                "y" => snap.y = v.parse().unwrap_or(0),
                "pos_x" => snap.pos_x = v.parse().unwrap_or(0.0),
                "pos_y" => snap.pos_y = v.parse().unwrap_or(0.0),
                "age" => snap.age = v.parse().unwrap_or(0.0),
                "held_id" => snap.held_id = v.parse().unwrap_or(0),
                "in_motion" => snap.in_motion = v == "1" || v.eq_ignore_ascii_case("true"),
                "food_store" => snap.food_store = v.parse().unwrap_or(0),
                "food_capacity" => snap.food_capacity = v.parse().unwrap_or(0),
                "heat" => snap.heat = v.parse().unwrap_or(0.0),
                "map_cells" => snap.map_cells = v.parse().unwrap_or(0),
                "player_count" => snap.player_count = v.parse().unwrap_or(0),
                "content_objects" => snap.content_objects = v.parse().unwrap_or(0),
                "data_version" => snap.data_version = v.parse().unwrap_or(0),
                "camera_x" => snap.camera_x = v.parse().unwrap_or(0.0),
                "camera_y" => snap.camera_y = v.parse().unwrap_or(0.0),
                "camera_zoom" => snap.camera_zoom = v.parse().unwrap_or(0.0),
                "pan_x" => snap.pan_x = v.parse().unwrap_or(0.0),
                "pan_y" => snap.pan_y = v.parse().unwrap_or(0.0),
                "hover_tile_x" => snap.hover_tile_x = v.parse().unwrap_or(0),
                "hover_tile_y" => snap.hover_tile_y = v.parse().unwrap_or(0),
                "hover_object_id" => snap.hover_object_id = v.parse().unwrap_or(0),
                "last_status" => snap.last_status = v,
                "screen" => snap.screen = v,
                "map_near" => snap.map_near = v,
                "others" => snap.others = v,
                "notes" => snap.notes = v,
                _ => {}
            }
        }
        if !saw_magic {
            return Err("missing OHOL_PLAY_SNAPSHOT header".into());
        }
        Ok(snap)
    }

    pub fn write_file(&self, path: impl AsRef<Path>) -> Result<PathBuf, String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, self.serialize()).map_err(|e| e.to_string())?;
        Ok(path.to_path_buf())
    }

    pub fn read_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::parse(&text)
    }

    /// `logs/snapshots/snap_<unix>_<label>.txt`
    pub fn default_path_for_label(label: &str) -> PathBuf {
        let safe: String = label
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .take(32)
            .collect();
        let safe = if safe.is_empty() {
            "play".into()
        } else {
            safe
        };
        PathBuf::from(DEFAULT_SNAPSHOT_DIR).join(format!(
            "snap_{}_{}.txt",
            unix_now(),
            safe
        ))
    }

    /// One-line summary for logs / title bar.
    pub fn summary_line(&self) -> String {
        format!(
            "snap[{}] our={:?} @{},{} held={} food={}/{} heat={:.2} players={} map={}",
            self.label,
            self.our_id,
            self.x,
            self.y,
            self.held_id,
            self.food_store,
            self.food_capacity,
            self.heat,
            self.player_count,
            self.map_cells
        )
    }
}

/// Soft-FB SNAP button rect (bottom-right). Returns (x0,y0,x1,y1) in FB pixels.
pub fn snapshot_button_rect(fb_w: u32, fb_h: u32) -> (i32, i32, i32, i32) {
    let w = 72i32;
    let h = 28i32;
    let x1 = fb_w as i32 - 12;
    let y1 = fb_h as i32 - 12;
    (x1 - w, y1 - h, x1, y1)
}

pub fn snapshot_button_hit(fb_w: u32, fb_h: u32, mx: i32, my: i32) -> bool {
    let (x0, y0, x1, y1) = snapshot_button_rect(fb_w, fb_h);
    mx >= x0 && mx < x1 && my >= y0 && my < y1
}

/// Draw semi-transparent SNAP chip (only when debug tools visible).
pub fn draw_snapshot_button(fb: &mut crate::render::Framebuffer) {
    let (x0, y0, x1, y1) = snapshot_button_rect(fb.width, fb.height);
    let bg = [40, 90, 140, 220];
    let border = [120, 200, 255, 255];
    for y in y0..y1 {
        for x in x0..x1 {
            if x >= 0 && y >= 0 && (x as u32) < fb.width && (y as u32) < fb.height {
                let edge = x == x0 || x == x1 - 1 || y == y0 || y == y1 - 1;
                fb.put(x, y, if edge { border } else { bg });
            }
        }
    }
    let cx = (x0 + x1) as f32 * 0.5;
    let cy = (y0 + y1) as f32 * 0.5 - 4.0;
    crate::hud::draw_pencil_string(fb, "SNAP", cx, cy, 1.4, [240, 250, 255, 255], true);
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn escape_val(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "")
}

fn unescape_val(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(o) => {
                    out.push('\\');
                    out.push(o);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Write snapshot from session; returns path written.
pub fn write_play_snapshot(
    session: &ClientSession,
    extras: &SnapshotViewExtras,
    path: Option<&Path>,
) -> Result<PathBuf, String> {
    let snap = PlaySnapshot::capture(session, extras);
    let path = path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PlaySnapshot::default_path_for_label(&snap.label));
    snap.write_file(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_synthetic() {
        let a = PlaySnapshot::synthetic_fixture();
        let text = a.serialize();
        assert!(text.contains(SNAPSHOT_MAGIC));
        let b = PlaySnapshot::parse(&text).unwrap();
        assert_eq!(b.our_id, Some(42));
        assert_eq!(b.x, 10);
        assert_eq!(b.held_id, 33);
        assert!((b.heat - 0.55).abs() < 1e-4);
        assert_eq!(b.label, "fixture");
        assert!(b.summary_line().contains("our=Some(42)"));
    }

    #[test]
    fn write_read_temp() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_snap_test_{}.txt",
            std::process::id()
        ));
        let a = PlaySnapshot::synthetic_fixture();
        a.write_file(&tmp).unwrap();
        let b = PlaySnapshot::read_file(&tmp).unwrap();
        assert_eq!(a.our_id, b.our_id);
        assert_eq!(a.map_near, b.map_near);
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn button_hit_corner() {
        let (x0, y0, x1, y1) = snapshot_button_rect(960, 540);
        assert!(snapshot_button_hit(960, 540, (x0 + x1) / 2, (y0 + y1) / 2));
        assert!(!snapshot_button_hit(960, 540, 10, 10));
    }

    #[test]
    fn reject_bad_magic() {
        assert!(PlaySnapshot::parse("not a snapshot\nx=1\n").is_err());
    }
}
