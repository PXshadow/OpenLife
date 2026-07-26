//! Versioned binary world save/load (fast bulk I/O).
//!
//! Format `OLW1` / `OLW2` (u32 version):
//! ```text
//! magic[4] = b"OLW1"
//! version: u32 LE  (1 or 2)
//! width, height: u32 LE
//! wrap: u8
//! biomes: width*height × u8
//! floors: width*height × u16 LE
//! objects: width*height × i32 LE
//! helper_count: u32
//! helpers OLW1: (x,y,base,uses,owner,n,contained[n])
//! helpers OLW2: OLW1 + creation_time:f32, time_to_change:f32,
//!               for each contained: nest_len:u16, nest[nest_len]:i32
//! ```
//!
//! **OLW2** adds Haxe-style container time + one-level nested persistence.
//! Load accepts both versions; save always writes OLW2.

use crate::{ComplexObject, World};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::info;

/// Current on-disk write version (nested + container time).
pub const WORLD_FORMAT_VERSION: u32 = 2;
/// Oldest readable format.
pub const WORLD_FORMAT_MIN: u32 = 1;
/// Default number of rotated `.bak.N` files kept by [`save_world_file`].
pub const DEFAULT_BACKUP_KEEP: usize = 3;
const MAGIC: &[u8; 4] = b"OLW1";

/// Backup path for slot `n` (≥1): `{path}.bak.{n}` e.g. `world_v1.olw.bak.1`.
pub fn world_backup_path(path: &Path, n: usize) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(format!(".bak.{n}"));
    PathBuf::from(s)
}

/// Before overwrite: if `path` exists, rotate numbered backups.
///
/// Copies current file to `path.bak.1`, shifting previous `.bak.1` → `.bak.2` …
/// up through `keep`, dropping the oldest beyond `keep`. No-op if `path` is
/// missing or `keep == 0`.
pub fn rotate_world_backups(path: &Path, keep: usize) -> Result<(), String> {
    if keep == 0 || !path.exists() {
        return Ok(());
    }
    // Drop oldest beyond keep.
    let oldest = world_backup_path(path, keep);
    if oldest.exists() {
        std::fs::remove_file(&oldest).map_err(|e| e.to_string())?;
    }
    // Shift .bak.(keep-1) … .bak.1 upward.
    for n in (1..keep).rev() {
        let from = world_backup_path(path, n);
        let to = world_backup_path(path, n + 1);
        if from.exists() {
            if to.exists() {
                std::fs::remove_file(&to).map_err(|e| e.to_string())?;
            }
            std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    // Copy live file to .bak.1 (keep original for atomic rename replace).
    let bak1 = world_backup_path(path, 1);
    std::fs::copy(path, &bak1).map_err(|e| e.to_string())?;
    Ok(())
}

/// Save world to `path`, rotating up to [`DEFAULT_BACKUP_KEEP`] backups first.
pub fn save_world_file(world: &World, path: impl AsRef<Path>) -> Result<(), String> {
    save_world_file_with_options(world, path, true, DEFAULT_BACKUP_KEEP)
}

/// Save world; when `rotate` is true and the file exists, call
/// [`rotate_world_backups`] with `keep` before overwrite.
pub fn save_world_file_with_options(
    world: &World,
    path: impl AsRef<Path>,
    rotate: bool,
    keep: usize,
) -> Result<(), String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if rotate {
        rotate_world_backups(path, keep)?;
    }
    // Atomic-ish: write temp then rename for fast restart safety.
    let tmp = path.with_extension("olw.tmp");
    {
        let f = File::create(&tmp).map_err(|e| e.to_string())?;
        let mut w = BufWriter::with_capacity(1 << 20, f);
        write_world(world, &mut w)?;
        w.flush().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    let ms = t0.elapsed().as_millis();
    info!(
        path = %path.display(),
        helpers = world.helpers.len(),
        version = WORLD_FORMAT_VERSION,
        ms,
        "world saved"
    );
    Ok(())
}

pub fn load_world_file(path: impl AsRef<Path>) -> Result<World, String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    let f = File::open(path).map_err(|e| e.to_string())?;
    let mut r = BufReader::with_capacity(1 << 20, f);
    let world = read_world(&mut r)?;
    let ms = t0.elapsed().as_millis();
    info!(
        path = %path.display(),
        w = world.width_tiles,
        h = world.height_tiles,
        helpers = world.helpers.len(),
        version = world.format_version,
        ms,
        "world loaded"
    );
    Ok(world)
}

fn write_world(world: &World, w: &mut impl Write) -> Result<(), String> {
    w.write_all(MAGIC).map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(WORLD_FORMAT_VERSION)
        .map_err(|e| e.to_string())?;
    let width = world.width_tiles;
    let height = world.height_tiles;
    w.write_u32::<LittleEndian>(width as u32)
        .map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(height as u32)
        .map_err(|e| e.to_string())?;
    w.write_u8(if world.wrap { 1 } else { 0 })
        .map_err(|e| e.to_string())?;

    let (biomes, floors, objects) = world.export_dense();
    w.write_all(&biomes).map_err(|e| e.to_string())?;
    for f in floors {
        w.write_u16::<LittleEndian>(f).map_err(|e| e.to_string())?;
    }
    for id in objects {
        w.write_i32::<LittleEndian>(id).map_err(|e| e.to_string())?;
    }

    w.write_u32::<LittleEndian>(world.helpers.len() as u32)
        .map_err(|e| e.to_string())?;
    for ((x, y), h) in &world.helpers {
        w.write_i32::<LittleEndian>(*x).map_err(|e| e.to_string())?;
        w.write_i32::<LittleEndian>(*y).map_err(|e| e.to_string())?;
        w.write_i32::<LittleEndian>(h.base_id)
            .map_err(|e| e.to_string())?;
        w.write_i32::<LittleEndian>(h.uses_remaining)
            .map_err(|e| e.to_string())?;
        w.write_i32::<LittleEndian>(h.owner_id)
            .map_err(|e| e.to_string())?;
        w.write_u16::<LittleEndian>(h.contained.len() as u16)
            .map_err(|e| e.to_string())?;
        for c in &h.contained {
            w.write_i32::<LittleEndian>(*c).map_err(|e| e.to_string())?;
        }
        // OLW2: container time + nested
        w.write_f32::<LittleEndian>(h.creation_time)
            .map_err(|e| e.to_string())?;
        w.write_f32::<LittleEndian>(h.time_to_change)
            .map_err(|e| e.to_string())?;
        for i in 0..h.contained.len() {
            let nest = h.nested.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
            w.write_u16::<LittleEndian>(nest.len() as u16)
                .map_err(|e| e.to_string())?;
            for n in nest {
                w.write_i32::<LittleEndian>(*n).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

fn read_world(r: &mut impl Read) -> Result<World, String> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != MAGIC {
        return Err(format!("bad magic {:?}", magic));
    }
    let version = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
    if version < WORLD_FORMAT_MIN || version > WORLD_FORMAT_VERSION {
        return Err(format!(
            "unsupported world version {version} (want {WORLD_FORMAT_MIN}..={WORLD_FORMAT_VERSION})"
        ));
    }
    let width = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as i32;
    let height = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as i32;
    let wrap = r.read_u8().map_err(|e| e.to_string())? != 0;
    let n = (width as usize).saturating_mul(height as usize);

    let mut biomes = vec![0u8; n];
    r.read_exact(&mut biomes).map_err(|e| e.to_string())?;

    let mut floors = vec![0u16; n];
    for f in floors.iter_mut() {
        *f = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())?;
    }
    let mut objects = vec![0i32; n];
    for o in objects.iter_mut() {
        *o = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    }

    let mut world = World::new(width, height, wrap);
    world.format_version = version;
    world.fill_from_dense(&biomes, &floors, &objects)?;

    let n_helpers = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    for _ in 0..n_helpers {
        let x = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
        let y = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
        let base_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
        let uses = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
        let owner = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
        let nc = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
        let mut contained = Vec::with_capacity(nc);
        for _ in 0..nc {
            contained.push(r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?);
        }
        let (creation_time, time_to_change, nested) = if version >= 2 {
            let ct = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
            let ttc = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
            let mut nested = Vec::with_capacity(nc);
            for _ in 0..nc {
                let nl = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
                let mut nest = Vec::with_capacity(nl);
                for _ in 0..nl {
                    nest.push(r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?);
                }
                nested.push(nest);
            }
            // Drop empty nesting so wire stays flat when unused.
            if nested.iter().all(|s| s.is_empty()) {
                nested.clear();
            }
            (ct, ttc, nested)
        } else {
            (0.0, 0.0, Vec::new())
        };
        world.set_object_complex(
            x,
            y,
            ComplexObject {
                base_id,
                uses_remaining: uses,
                contained,
                nested,
                owner_id: owner,
                creation_time,
                time_to_change,
            },
        );
    }
    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn save_load_preserves_simple_and_complex() {
        let dir = env::temp_dir().join("ol_world_persist_test2");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("w.olw");

        let mut w = World::new(64, 32, true);
        w.ensure_full_map_chunks();
        w.set_biome(3, 4, 5);
        w.set_object(3, 4, 33);
        w.set_object_complex(
            10,
            11,
            ComplexObject {
                base_id: 391,
                uses_remaining: 2,
                contained: vec![33],
                nested: Vec::new(),
                owner_id: 7,
                creation_time: 12.5,
                time_to_change: 60.0,
            },
        );

        // Disable rotate so this test stays isolated from backup side-effects.
        save_world_file_with_options(&w, &path, false, 0).unwrap();
        let loaded = load_world_file(&path).unwrap();
        assert_eq!(loaded.width_tiles, 64);
        assert_eq!(loaded.height_tiles, 32);
        assert_eq!(loaded.get_biome(3, 4), 5);
        assert_eq!(loaded.get_object(3, 4), 33);
        let h = loaded.get_helper(10, 11).unwrap();
        assert_eq!(h.base_id, 391);
        assert_eq!(h.uses_remaining, 2);
        assert_eq!(h.contained, vec![33]);
        assert!(h.nested.is_empty());
        assert_eq!(h.owner_id, 7);
        assert!((h.creation_time - 12.5).abs() < 1e-5);
        assert!((h.time_to_change - 60.0).abs() < 1e-5);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nested_persisted_in_olw2() {
        let dir = env::temp_dir().join("ol_world_nested_persist");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("w.olw");

        let mut w = World::new(32, 32, false);
        w.ensure_full_map_chunks();
        w.set_object_complex(
            1,
            1,
            ComplexObject {
                base_id: 391,
                uses_remaining: 0,
                contained: vec![33, 40],
                nested: vec![vec![100, 101], vec![]],
                owner_id: 0,
                creation_time: 1.0,
                time_to_change: 0.0,
            },
        );
        save_world_file_with_options(&w, &path, false, 0).unwrap();
        let loaded = load_world_file(&path).unwrap();
        let h = loaded.get_helper(1, 1).unwrap();
        assert_eq!(h.contained, vec![33, 40]);
        assert_eq!(h.nested.len(), 2);
        assert_eq!(h.nested[0], vec![100, 101]);
        assert!(h.nested[1].is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn container_put_take_roundtrip() {
        let mut w = World::new(32, 32, false);
        w.set_object(1, 1, 391); // basket-like
        assert!(w.container_put(1, 1, 33, 4));
        assert!(w.container_put(1, 1, 40, 4));
        assert_eq!(w.container_take(1, 1, Some(0)), Some(33));
        assert_eq!(w.get_helper(1, 1).unwrap().contained, vec![40]);
    }

    #[test]
    fn rotate_world_backups_keeps_last_n() {
        let dir = env::temp_dir().join("ol_world_backup_rotate_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("world_v1.olw");

        let w = World::new(8, 8, false);
        for i in 1..=4 {
            save_world_file_with_options(&w, &path, true, 3).unwrap();
            assert!(path.exists(), "live file after save {i}");
        }
        assert!(world_backup_path(&path, 1).exists());
        assert!(world_backup_path(&path, 2).exists());
        assert!(world_backup_path(&path, 3).exists());
        assert!(!world_backup_path(&path, 4).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
