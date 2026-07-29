//! Versioned binary world save/load (fast bulk I/O).
//!
//! Format `OLW1` magic + u32 version:
//! ```text
//! magic[4] = b"OLW1"
//! version: u32 LE  (1, 2, or 3)
//! width, height: u32 LE
//! wrap: u8
//! biomes: width*height × u8
//! floors: width*height × u16 LE
//! objects: width*height × i32 LE
//! helper_count: u32
//! helpers OLW1: (x,y,base,uses,owner,n,contained[n])
//! helpers OLW2: OLW1 + creation_time:f32, time_to_change:f32,
//!               for each contained: nest_len:u16, nest[nest_len]:i32
//! helpers OLW3: x,y,base,uses,owner,
//!               living_owners[], owners_by_account[] (u16 len + i32s),
//!               creation_time:f32, time_to_change:f32,
//!               custom vars (Haxe dataVersion 6),
//!               ground_id:i32,
//!               slots: u16 count + recursive NestedHelper each
//!                 (id, owners[], uses, times, custom, sub-slots)
//! ```
//!
//! **OLW3** closes Haxe `ObjectHelper.WriteToFile` / `ReadFromFile` recursive
//! meta for contained trees (uses, owners, times, hits/coins/text/externId/countObj)
//! plus multi-owner arrays and `ground_id`. Load accepts v1–v3; save writes OLW3.
//!
//! Chunk **NESTED-OLW1** / `container_persist`.

use crate::{ComplexObject, NestedHelper, World};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::info;

/// Current on-disk write version (recursive NestedHelper meta).
pub const WORLD_FORMAT_VERSION: u32 = 3;
/// Oldest readable format.
pub const WORLD_FORMAT_MIN: u32 = 1;
/// Default number of rotated `.bak.N` files kept by [`save_world_file`].
pub const DEFAULT_BACKUP_KEEP: usize = 3;
const MAGIC: &[u8; 4] = b"OLW1";

// Haxe: ObjectDataSaveIds
const CUSTOM_HITS: i16 = 1;
const CUSTOM_COINS: i16 = 2;
const CUSTOM_TEXT: i16 = 3;
const CUSTOM_EXTERNID: i16 = 4;
const CUSTOM_COUNTOBJ: i16 = 5;

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

// ---------------------------------------------------------------------------
// Pure I/O helpers (Haxe WorldMap.WriteInt32Array / ObjectHelper.WriteToFile)
// ---------------------------------------------------------------------------

/// Haxe `WorldMap.WriteInt32Array` — length as u16 (Rust allows >100 owners safely).
fn write_i32_array(w: &mut impl Write, arr: &[i32]) -> Result<(), String> {
    w.write_u16::<LittleEndian>(arr.len() as u16)
        .map_err(|e| e.to_string())?;
    for v in arr {
        w.write_i32::<LittleEndian>(*v).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn read_i32_array(r: &mut impl Read) -> Result<Vec<i32>, String> {
    let n = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// Haxe custom variables block (dataVersion ≥ 6).
fn write_custom_vars(
    w: &mut impl Write,
    hits: f32,
    coins: f32,
    text: &str,
    extern_id: i32,
    count_obj: f32,
) -> Result<(), String> {
    let mut count: i16 = 0;
    if hits != 0.0 {
        count += 1;
    }
    if coins != 0.0 {
        count += 1;
    }
    if !text.is_empty() {
        count += 1;
    }
    if extern_id != 0 {
        count += 1;
    }
    if count_obj != 0.0 {
        count += 1;
    }
    w.write_i16::<LittleEndian>(count)
        .map_err(|e| e.to_string())?;
    if hits != 0.0 {
        w.write_i16::<LittleEndian>(CUSTOM_HITS)
            .map_err(|e| e.to_string())?;
        w.write_f32::<LittleEndian>(hits)
            .map_err(|e| e.to_string())?;
    }
    if coins != 0.0 {
        w.write_i16::<LittleEndian>(CUSTOM_COINS)
            .map_err(|e| e.to_string())?;
        w.write_f32::<LittleEndian>(coins)
            .map_err(|e| e.to_string())?;
    }
    if !text.is_empty() {
        w.write_i16::<LittleEndian>(CUSTOM_TEXT)
            .map_err(|e| e.to_string())?;
        let bytes = text.as_bytes();
        w.write_i16::<LittleEndian>(bytes.len() as i16)
            .map_err(|e| e.to_string())?;
        w.write_all(bytes).map_err(|e| e.to_string())?;
    }
    if extern_id != 0 {
        w.write_i16::<LittleEndian>(CUSTOM_EXTERNID)
            .map_err(|e| e.to_string())?;
        w.write_i32::<LittleEndian>(extern_id)
            .map_err(|e| e.to_string())?;
    }
    if count_obj != 0.0 {
        w.write_i16::<LittleEndian>(CUSTOM_COUNTOBJ)
            .map_err(|e| e.to_string())?;
        w.write_f32::<LittleEndian>(count_obj)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn read_custom_vars(r: &mut impl Read) -> Result<(f32, f32, String, i32, f32), String> {
    let count = r.read_i16::<LittleEndian>().map_err(|e| e.to_string())?;
    let mut hits = 0.0f32;
    let mut coins = 0.0f32;
    let mut text = String::new();
    let mut extern_id = 0i32;
    let mut count_obj = 0.0f32;
    for _ in 0..count {
        let data_id = r.read_i16::<LittleEndian>().map_err(|e| e.to_string())?;
        match data_id {
            CUSTOM_HITS => {
                hits = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
            }
            CUSTOM_COINS => {
                coins = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
            }
            CUSTOM_TEXT => {
                let len = r.read_i16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
                let mut buf = vec![0u8; len];
                r.read_exact(&mut buf).map_err(|e| e.to_string())?;
                text = String::from_utf8_lossy(&buf).into_owned();
            }
            CUSTOM_EXTERNID => {
                extern_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
            }
            CUSTOM_COUNTOBJ => {
                count_obj = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
            }
            other => return Err(format!("unknown custom DataId {other}")),
        }
    }
    Ok((hits, coins, text, extern_id, count_obj))
}

/// Sentinel id for Haxe `WriteToFile(null)` / `ReadFromFile` → `array[0] == -100`.
/// Used by optional body-object encode (held null rare; hiddenWound/fever often null).
pub const NESTED_NULL_ID: i32 = -100;

/// Haxe `ObjectHelper.WriteToFile` for a recursive contained helper (no map x/y).
// Haxe: ObjectHelper.WriteToFile
pub fn write_nested_helper(w: &mut impl Write, h: &NestedHelper) -> Result<(), String> {
    w.write_i32::<LittleEndian>(h.id).map_err(|e| e.to_string())?;
    write_i32_array(w, &h.living_owners)?;
    write_i32_array(w, &h.owners_by_account)?;
    w.write_i32::<LittleEndian>(h.uses_remaining)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(h.creation_time)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(h.time_to_change)
        .map_err(|e| e.to_string())?;
    write_custom_vars(w, h.hits, h.coins, &h.text, h.extern_id, h.count_obj)?;
    w.write_u16::<LittleEndian>(h.contained.len() as u16)
        .map_err(|e| e.to_string())?;
    for c in &h.contained {
        write_nested_helper(w, c)?;
    }
    Ok(())
}

// Haxe: ObjectHelper.ReadFromFile (contained branch, dataVersion ≥ 5/6)
pub fn read_nested_helper(r: &mut impl Read) -> Result<NestedHelper, String> {
    let id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let living_owners = read_i32_array(r)?;
    let owners_by_account = read_i32_array(r)?;
    let uses_remaining = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let creation_time = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    let time_to_change = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    let (hits, coins, text, extern_id, count_obj) = read_custom_vars(r)?;
    let nc = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    let mut contained = Vec::with_capacity(nc);
    for _ in 0..nc {
        contained.push(read_nested_helper(r)?);
    }
    Ok(NestedHelper {
        id,
        uses_remaining,
        living_owners,
        owners_by_account,
        creation_time,
        time_to_change,
        hits,
        coins,
        text,
        extern_id,
        count_obj,
        contained,
    })
}

/// Haxe `ObjectHelper.WriteToFile` when `obj == null` writes `[-100]`; we write a full
/// NestedHelper with [`NESTED_NULL_ID`] so the recursive reader stays uniform.
// Haxe: ObjectHelper.WriteToFile (null branch)
pub fn write_optional_nested_helper(
    w: &mut impl Write,
    h: Option<&NestedHelper>,
) -> Result<(), String> {
    match h {
        None => write_nested_helper(
            w,
            &NestedHelper {
                id: NESTED_NULL_ID,
                ..Default::default()
            },
        ),
        Some(helper) => write_nested_helper(w, helper),
    }
}

/// Inverse of [`write_optional_nested_helper`]; `id == -100` → `None`.
// Haxe: ObjectHelper.ReadFromFile (null via array[0] == -100)
pub fn read_optional_nested_helper(r: &mut impl Read) -> Result<Option<NestedHelper>, String> {
    let h = read_nested_helper(r)?;
    if h.id == NESTED_NULL_ID {
        Ok(None)
    } else {
        Ok(Some(h))
    }
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
        write_helper_olw3(w, *x, *y, h)?;
    }
    Ok(())
}

/// OLW3 top-level helper (map cell) — Haxe WriteMapObjHelpers → WriteToFile.
// Haxe: ObjectHelper.WriteToFile / WriteMapObjHelpers
fn write_helper_olw3(
    w: &mut impl Write,
    x: i32,
    y: i32,
    h: &ComplexObject,
) -> Result<(), String> {
    w.write_i32::<LittleEndian>(x).map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(y).map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(h.base_id)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(h.uses_remaining)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(h.owner_id)
        .map_err(|e| e.to_string())?;

    // Multi-owner arrays (Haxe livingOwners / ownersByPlayerAccount).
    let living = if h.living_owners.is_empty() && h.owner_id != 0 {
        vec![h.owner_id]
    } else {
        h.living_owners.clone()
    };
    write_i32_array(w, &living)?;
    write_i32_array(w, &h.owners_by_account)?;

    w.write_f32::<LittleEndian>(h.creation_time)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(h.time_to_change)
        .map_err(|e| e.to_string())?;
    write_custom_vars(w, h.hits, h.coins, &h.text, h.extern_id, h.count_obj)?;
    w.write_i32::<LittleEndian>(h.ground_id)
        .map_err(|e| e.to_string())?;

    // Recursive slots: prefer tracked meta; else synthesize from wire ids.
    let mut slots = h.slots.clone();
    if slots.is_empty() && !h.contained.is_empty() {
        slots = h
            .contained
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let nest = h.nested.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
                NestedHelper::from_wire(id, nest)
            })
            .collect();
    } else if !slots.is_empty() {
        // Reconcile top ids with runtime wire after put/take.
        for (i, s) in slots.iter_mut().enumerate() {
            if let Some(&id) = h.contained.get(i) {
                s.id = id;
            }
            if let Some(nest) = h.nested.get(i) {
                if s.contained.len() != nest.len()
                    || s.contained
                        .iter()
                        .zip(nest.iter())
                        .any(|(c, &nid)| c.id != nid)
                {
                    // Preserve deeper meta where possible by rebuilding one level.
                    let old = std::mem::take(&mut s.contained);
                    s.contained = nest
                        .iter()
                        .map(|&nid| {
                            old.iter()
                                .find(|c| c.id == nid)
                                .cloned()
                                .unwrap_or_else(|| NestedHelper::id_only(nid))
                        })
                        .collect();
                }
            }
        }
        // Length match to contained
        slots.truncate(h.contained.len());
        while slots.len() < h.contained.len() {
            let id = h.contained[slots.len()];
            let nest = h
                .nested
                .get(slots.len())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            slots.push(NestedHelper::from_wire(id, nest));
        }
    }

    w.write_u16::<LittleEndian>(slots.len() as u16)
        .map_err(|e| e.to_string())?;
    for s in &slots {
        write_nested_helper(w, s)?;
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
        let helper = match version {
            1 => read_helper_v1(r)?,
            2 => read_helper_v2(r)?,
            3 => read_helper_v3(r)?,
            _ => unreachable!(),
        };
        let (x, y, co) = helper;
        world.set_object_complex(x, y, co);
    }
    Ok(world)
}

fn read_helper_v1(r: &mut impl Read) -> Result<(i32, i32, ComplexObject), String> {
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
    let mut co = ComplexObject::new_simple(base_id);
    co.uses_remaining = uses;
    co.owner_id = owner;
    if owner != 0 {
        co.living_owners = vec![owner];
    }
    co.contained = contained;
    Ok((x, y, co))
}

fn read_helper_v2(r: &mut impl Read) -> Result<(i32, i32, ComplexObject), String> {
    let (x, y, mut co) = read_helper_v1(r)?;
    let ct = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    let ttc = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    let nc = co.contained.len();
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
    co.creation_time = ct;
    co.time_to_change = ttc;
    co.nested = nested;
    Ok((x, y, co))
}

fn read_helper_v3(r: &mut impl Read) -> Result<(i32, i32, ComplexObject), String> {
    let x = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let y = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let base_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let uses = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let owner = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let living_owners = read_i32_array(r)?;
    let owners_by_account = read_i32_array(r)?;
    let creation_time = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    let time_to_change = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    let (hits, coins, text, extern_id, count_obj) = read_custom_vars(r)?;
    let ground_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let ns = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    let mut slots = Vec::with_capacity(ns);
    for _ in 0..ns {
        slots.push(read_nested_helper(r)?);
    }

    let mut co = ComplexObject::new_simple(base_id);
    co.uses_remaining = uses;
    co.owner_id = owner;
    co.living_owners = living_owners;
    if co.owner_id == 0 {
        if let Some(&first) = co.living_owners.first() {
            co.owner_id = first;
        }
    }
    co.owners_by_account = owners_by_account;
    co.creation_time = creation_time;
    co.time_to_change = time_to_change;
    co.hits = hits;
    co.coins = coins;
    co.text = text;
    co.extern_id = extern_id;
    co.count_obj = count_obj;
    co.ground_id = ground_id;
    co.slots = slots;
    co.rebuild_wire_from_slots();
    Ok((x, y, co))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::io::Cursor;

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
        let mut h = ComplexObject::new_simple(391);
        h.uses_remaining = 2;
        h.contained = vec![33];
        h.owner_id = 7;
        h.living_owners = vec![7];
        h.creation_time = 12.5;
        h.time_to_change = 60.0;
        w.set_object_complex(10, 11, h);

        // Disable rotate so this test stays isolated from backup side-effects.
        save_world_file_with_options(&w, &path, false, 0).unwrap();
        let loaded = load_world_file(&path).unwrap();
        assert_eq!(loaded.format_version, WORLD_FORMAT_VERSION);
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

    /// NESTED-OLW1: one-level nests round-trip on disk (version 3 / magic OLW1).
    #[test]
    fn nested_persisted_in_olw2() {
        let dir = env::temp_dir().join("ol_world_nested_persist");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("w.olw");

        let mut w = World::new(32, 32, false);
        w.ensure_full_map_chunks();
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![33, 40];
        h.nested = vec![vec![100, 101], vec![]];
        h.creation_time = 1.0;
        w.set_object_complex(1, 1, h);
        save_world_file_with_options(&w, &path, false, 0).unwrap();
        let loaded = load_world_file(&path).unwrap();
        assert_eq!(loaded.format_version, 3);
        let h = loaded.get_helper(1, 1).unwrap();
        assert_eq!(h.contained, vec![33, 40]);
        assert_eq!(h.nested.len(), 2);
        assert_eq!(h.nested[0], vec![100, 101]);
        assert!(h.nested[1].is_empty());
        // Wire form after load matches Haxe `toString` colon nests.
        assert_eq!(h.to_map_string_id(), "391,33:100:101,40");
        assert_eq!(loaded.encode_object_for_map(1, 1), "391,33:100:101,40");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// container_put_nested → save → load preserves nests (runtime API path).
    #[test]
    fn container_api_nested_save_load_roundtrip() {
        let dir = env::temp_dir().join("ol_world_nested_api_persist");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("w.olw");

        let mut w = World::new(16, 16, false);
        w.ensure_full_map_chunks();
        w.set_object(2, 3, 391);
        assert!(w.container_put(2, 3, 292, 4));
        assert!(w.container_put(2, 3, 40, 4));
        assert!(w.container_put_nested(2, 3, 0, 100, 4));
        assert!(w.container_put_nested(2, 3, 0, 101, 4));
        assert_eq!(
            w.get_helper(2, 3).unwrap().to_map_string_id(),
            "391,292:100:101,40"
        );

        save_world_file_with_options(&w, &path, false, 0).unwrap();
        let loaded = load_world_file(&path).unwrap();
        let h = loaded.get_helper(2, 3).unwrap();
        assert_eq!(h.contained, vec![292, 40]);
        assert_eq!(h.nested, vec![vec![100, 101], vec![]]);
        assert_eq!(h.to_map_string_id(), "391,292:100:101,40");
        // Take nested after load still works.
        assert_eq!(
            {
                let mut w2 = loaded;
                w2.container_take_nested(2, 3, 0, Some(0))
            },
            Some(100)
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Legacy version=1 helpers load without nested fields (nests empty).
    #[test]
    fn olw1_v1_load_nested_empty() {
        // Minimal 1×1 world, one helper with contained only (no nest bytes).
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.write_u32::<LittleEndian>(1).unwrap(); // version 1
        buf.write_u32::<LittleEndian>(1).unwrap(); // w
        buf.write_u32::<LittleEndian>(1).unwrap(); // h
        buf.write_u8(0).unwrap(); // wrap
        buf.write_u8(0).unwrap(); // biome
        buf.write_u16::<LittleEndian>(0).unwrap(); // floor
        buf.write_i32::<LittleEndian>(391).unwrap(); // object
        buf.write_u32::<LittleEndian>(1).unwrap(); // helpers
        buf.write_i32::<LittleEndian>(0).unwrap(); // x
        buf.write_i32::<LittleEndian>(0).unwrap(); // y
        buf.write_i32::<LittleEndian>(391).unwrap(); // base
        buf.write_i32::<LittleEndian>(0).unwrap(); // uses
        buf.write_i32::<LittleEndian>(0).unwrap(); // owner
        buf.write_u16::<LittleEndian>(1).unwrap(); // n contained
        buf.write_i32::<LittleEndian>(33).unwrap(); // contained[0]
        // no creation_time / nest (v1)

        let loaded = read_world(&mut Cursor::new(buf)).unwrap();
        assert_eq!(loaded.format_version, 1);
        let h = loaded.get_helper(0, 0).unwrap();
        assert_eq!(h.contained, vec![33]);
        assert!(h.nested.is_empty());
        assert_eq!(h.creation_time, 0.0);
        assert_eq!(h.time_to_change, 0.0);
    }

    /// v1 load then save rewrites as current OLW (empty nests).
    #[test]
    fn v1_load_then_save_rewrites_current_format() {
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.write_u32::<LittleEndian>(1).unwrap();
        buf.write_u32::<LittleEndian>(1).unwrap();
        buf.write_u32::<LittleEndian>(1).unwrap();
        buf.write_u8(0).unwrap();
        buf.write_u8(0).unwrap();
        buf.write_u16::<LittleEndian>(0).unwrap();
        buf.write_i32::<LittleEndian>(391).unwrap();
        buf.write_u32::<LittleEndian>(1).unwrap();
        buf.write_i32::<LittleEndian>(0).unwrap();
        buf.write_i32::<LittleEndian>(0).unwrap();
        buf.write_i32::<LittleEndian>(391).unwrap();
        buf.write_i32::<LittleEndian>(2).unwrap(); // uses
        buf.write_i32::<LittleEndian>(9).unwrap(); // owner
        buf.write_u16::<LittleEndian>(2).unwrap();
        buf.write_i32::<LittleEndian>(33).unwrap();
        buf.write_i32::<LittleEndian>(40).unwrap();

        let loaded = read_world(&mut Cursor::new(buf)).unwrap();
        assert_eq!(loaded.format_version, 1);
        let mut out = Vec::new();
        write_world(&loaded, &mut out).unwrap();
        let reloaded = read_world(&mut Cursor::new(out)).unwrap();
        assert_eq!(reloaded.format_version, WORLD_FORMAT_VERSION);
        let h = reloaded.get_helper(0, 0).unwrap();
        assert_eq!(h.contained, vec![33, 40]);
        assert!(h.nested.is_empty());
        assert_eq!(h.uses_remaining, 2);
        assert_eq!(h.owner_id, 9);
        assert_eq!(h.to_map_string_id(), "391,33,40");
    }

    /// Asymmetric multi-slot nests: slot0 nested, slot1 empty, slot2 nested.
    #[test]
    fn asymmetric_multi_slot_nests_roundtrip() {
        let dir = env::temp_dir().join("ol_world_asymmetric_nest");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("w.olw");

        let mut w = World::new(8, 8, false);
        w.ensure_full_map_chunks();
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![10, 20, 30];
        h.nested = vec![vec![100], vec![], vec![200, 201]];
        h.uses_remaining = 4;
        h.creation_time = 5.0;
        h.time_to_change = 30.0;
        w.set_object_complex(0, 0, h);

        save_world_file_with_options(&w, &path, false, 0).unwrap();
        let loaded = load_world_file(&path).unwrap();
        let h = loaded.get_helper(0, 0).unwrap();
        assert_eq!(h.contained, vec![10, 20, 30]);
        assert_eq!(h.nested, vec![vec![100], vec![], vec![200, 201]]);
        assert_eq!(h.uses_remaining, 4);
        assert!((h.creation_time - 5.0).abs() < 1e-5);
        assert!((h.time_to_change - 30.0).abs() < 1e-5);
        assert_eq!(h.to_map_string_id(), "391,10:100,20,30:200:201");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// All-empty nested vectors cleared on load → flat wire encode.
    #[test]
    fn all_empty_nested_cleared_flat_wire() {
        let dir = env::temp_dir().join("ol_world_empty_nested");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("w.olw");

        let mut w = World::new(8, 8, false);
        w.ensure_full_map_chunks();
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![33, 40];
        // Explicit empty nest rows (should collapse after load).
        h.nested = vec![vec![], vec![]];
        w.set_object_complex(1, 1, h);
        // Runtime still has nested rows until load rebuild; force via slots path.
        h = w.get_helper(1, 1).unwrap().clone();
        h.synthesize_slots_from_wire();
        // Clear nested ids under slots so rebuild empties nested
        for s in &mut h.slots {
            s.contained.clear();
        }
        h.rebuild_wire_from_slots();
        assert!(h.nested.is_empty());
        w.set_object_complex(1, 1, h);

        save_world_file_with_options(&w, &path, false, 0).unwrap();
        let loaded = load_world_file(&path).unwrap();
        let h = loaded.get_helper(1, 1).unwrap();
        assert!(h.nested.is_empty());
        assert_eq!(h.to_map_string_id(), "391,33,40");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLW3: per-slot uses/times/custom + multi-owner + ground_id + multi-level nest.
    #[test]
    fn olw3_slot_meta_and_owners_roundtrip() {
        let dir = env::temp_dir().join("ol_world_olw3_meta");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("w.olw");

        let mut w = World::new(8, 8, false);
        w.ensure_full_map_chunks();
        let mut h = ComplexObject::new_simple(391);
        h.uses_remaining = 3;
        h.owner_id = 7;
        h.living_owners = vec![7, 8];
        h.owners_by_account = vec![100, 101];
        h.creation_time = 9.0;
        h.time_to_change = 12.0;
        h.hits = 2.5;
        h.coins = 1.25;
        h.text = "grave".into();
        h.extern_id = 42;
        h.count_obj = 3.0;
        h.ground_id = 99;
        h.slots = vec![
            NestedHelper {
                id: 292,
                uses_remaining: 4,
                creation_time: 1.0,
                time_to_change: 2.0,
                hits: 0.5,
                living_owners: vec![7],
                contained: vec![
                    NestedHelper {
                        id: 100,
                        uses_remaining: 1,
                        coins: 0.1,
                        contained: vec![NestedHelper::id_only(999)], // multi-level
                        ..Default::default()
                    },
                    NestedHelper::id_only(101),
                ],
                ..Default::default()
            },
            NestedHelper {
                id: 40,
                uses_remaining: 2,
                text: "note".into(),
                extern_id: 7,
                ..Default::default()
            },
        ];
        h.rebuild_wire_from_slots();
        // Wire only one nest level
        assert_eq!(h.contained, vec![292, 40]);
        assert_eq!(h.nested[0], vec![100, 101]);
        assert_eq!(h.to_map_string_id(), "391,292:100:101,40");
        // Deep level lives only in slots
        assert_eq!(h.slots[0].contained[0].contained[0].id, 999);

        w.set_object_complex(2, 2, h);
        save_world_file_with_options(&w, &path, false, 0).unwrap();
        let loaded = load_world_file(&path).unwrap();
        assert_eq!(loaded.format_version, 3);
        let h = loaded.get_helper(2, 2).unwrap();
        assert_eq!(h.uses_remaining, 3);
        assert_eq!(h.owner_id, 7);
        assert_eq!(h.living_owners, vec![7, 8]);
        assert_eq!(h.owners_by_account, vec![100, 101]);
        assert!((h.hits - 2.5).abs() < 1e-5);
        assert!((h.coins - 1.25).abs() < 1e-5);
        assert_eq!(h.text, "grave");
        assert_eq!(h.extern_id, 42);
        assert!((h.count_obj - 3.0).abs() < 1e-5);
        assert_eq!(h.ground_id, 99);
        assert_eq!(h.slots.len(), 2);
        assert_eq!(h.slots[0].uses_remaining, 4);
        assert!((h.slots[0].creation_time - 1.0).abs() < 1e-5);
        assert!((h.slots[0].hits - 0.5).abs() < 1e-5);
        assert_eq!(h.slots[0].contained[0].uses_remaining, 1);
        assert!((h.slots[0].contained[0].coins - 0.1).abs() < 1e-5);
        // Multi-level recursive nest preserved on disk
        assert_eq!(h.slots[0].contained[0].contained[0].id, 999);
        assert_eq!(h.slots[1].uses_remaining, 2);
        assert_eq!(h.slots[1].text, "note");
        assert_eq!(h.slots[1].extern_id, 7);
        assert_eq!(h.to_map_string_id(), "391,292:100:101,40");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// container_take_nested after load preserves remaining nest order.
    #[test]
    fn container_take_nested_after_load_preserves_order() {
        let dir = env::temp_dir().join("ol_world_take_nested_order");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("w.olw");

        let mut w = World::new(8, 8, false);
        w.ensure_full_map_chunks();
        w.set_object(0, 0, 391);
        assert!(w.container_put(0, 0, 292, 4));
        assert!(w.container_put_nested(0, 0, 0, 100, 4));
        assert!(w.container_put_nested(0, 0, 0, 101, 4));
        assert!(w.container_put_nested(0, 0, 0, 102, 4));
        save_world_file_with_options(&w, &path, false, 0).unwrap();

        let mut loaded = load_world_file(&path).unwrap();
        assert_eq!(loaded.container_take_nested(0, 0, 0, Some(1)), Some(101));
        let h = loaded.get_helper(0, 0).unwrap();
        assert_eq!(h.nested[0], vec![100, 102]);
        assert_eq!(h.to_map_string_id(), "391,292:100:102");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLW2 legacy bytes still load (nested ids only).
    #[test]
    fn olw2_legacy_load() {
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.write_u32::<LittleEndian>(2).unwrap();
        buf.write_u32::<LittleEndian>(1).unwrap();
        buf.write_u32::<LittleEndian>(1).unwrap();
        buf.write_u8(0).unwrap();
        buf.write_u8(0).unwrap();
        buf.write_u16::<LittleEndian>(0).unwrap();
        buf.write_i32::<LittleEndian>(391).unwrap();
        buf.write_u32::<LittleEndian>(1).unwrap();
        buf.write_i32::<LittleEndian>(0).unwrap();
        buf.write_i32::<LittleEndian>(0).unwrap();
        buf.write_i32::<LittleEndian>(391).unwrap();
        buf.write_i32::<LittleEndian>(0).unwrap();
        buf.write_i32::<LittleEndian>(0).unwrap();
        buf.write_u16::<LittleEndian>(2).unwrap(); // contained
        buf.write_i32::<LittleEndian>(33).unwrap();
        buf.write_i32::<LittleEndian>(40).unwrap();
        buf.write_f32::<LittleEndian>(1.0).unwrap();
        buf.write_f32::<LittleEndian>(0.0).unwrap();
        // nest for slot0: [100,101], slot1: []
        buf.write_u16::<LittleEndian>(2).unwrap();
        buf.write_i32::<LittleEndian>(100).unwrap();
        buf.write_i32::<LittleEndian>(101).unwrap();
        buf.write_u16::<LittleEndian>(0).unwrap();

        let loaded = read_world(&mut Cursor::new(buf)).unwrap();
        assert_eq!(loaded.format_version, 2);
        let h = loaded.get_helper(0, 0).unwrap();
        assert_eq!(h.contained, vec![33, 40]);
        assert_eq!(h.nested[0], vec![100, 101]);
        assert!(h.slots.is_empty()); // v2 has no slots meta
        assert_eq!(h.to_map_string_id(), "391,33:100:101,40");
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

    #[test]
    fn nested_helper_write_read_pure() {
        let mut tree = NestedHelper {
            id: 10,
            uses_remaining: 2,
            living_owners: vec![1, 2],
            owners_by_account: vec![9],
            creation_time: 3.0,
            time_to_change: 4.0,
            hits: 1.0,
            coins: 0.0,
            text: "x".into(),
            extern_id: 5,
            count_obj: 0.0,
            contained: vec![NestedHelper {
                id: 11,
                uses_remaining: 1,
                contained: vec![NestedHelper::id_only(12)],
                ..Default::default()
            }],
        };
        let mut buf = Vec::new();
        write_nested_helper(&mut buf, &tree).unwrap();
        let got = read_nested_helper(&mut Cursor::new(buf)).unwrap();
        // normalize empty fields for PartialEq
        tree.coins = 0.0;
        tree.count_obj = 0.0;
        assert_eq!(got, tree);
        assert_eq!(got.contained[0].contained[0].id, 12);
    }
}
