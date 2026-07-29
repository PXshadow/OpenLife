//! Shared OLC1 / OLT1 / OLA1 / OLG1 / OLO1 / OLSN / OLS1 binary content cache.
//!
//! See `docs/port/CONTENT_BINARY.md`. Text `OneLifeData7` remains source of truth;
//! these blobs are a **fast-start cache**.
//!
//! **P4#30:** OLC1/OLT1 wire format parse+encode lives in shared crate [`ol_binary`]
//! (also used by server `ol-content`). This module maps DTOs ↔ [`ClientContent`]
//! and owns client-only bake (OLA1/OLG1/OLO1/OLSN/OLS1), multi-use/variable dummies,
//! and prefer_cache policy.
//!
//! - **OLC1** — objects (client subset of ObjectRecord / Haxe ObjectData) + multi-use
//!   dummy id lists (Haxe `ObjectBake` / server `assign_multi_use_dummies`).
//!   Format 2 appends `use_chance`. Format 3 adds sim fields + blocking radii
//!   (`map_chance`, biomes, heat, `speed_mult`, decay, left/right radii).
//!   Format 4 adds creation/using/eating/decay SoundUsage strings (`sounds=`).
//!   Format 5 adds sparse `useVanishIndex` / `useAppearIndex` (P4#25 setupSpriteUseVis).
//!   Format 6 adds `variableDummyIDs` lists (P4#26 C++ `autoGenerateVariableObjects`).
//!   Format 7 adds `deadlyDistance` / `useDistance` / `moves` (server IS-CLOSE / animals).
//! - **OLT1** — transitions. Format 1: craft subset. Format 2: reverse/no-use/move/
//!   min-use + last-use + **max-use** records stored alongside normal (no key collision).
//!   Record **flag bit6** = max-use table (`TR_F_MAX_USE_TARGET`); **bit7** =
//!   `switch_number_of_uses`. Header **flags bit0** = category-expanded at bake
//!   (`OLT1_F_CATEGORY_EXPANDED`): load skips runtime re-expand (still loads
//!   `categories/` for prob-set pick).
//! - **OLA1** — animations (client; via [`crate::anim_bank`]).
//! - **OLG1** — ground tile / ground overlay (`ground_tN`) path index (client; [`crate::ground_sprites`]).
//! - **OLO1** — editor `overlays/{tag}/{id}.tga` path index (client; [`crate::overlay_bank`]).
//! - **OLSN** — sound **index only** (client; [`crate::sound_bank`]); lazy AIFF decode.
//! - **OLS1** — sprite **meta only** (client; [`crate::sprite_bank`]); lazy TGA + atlas.
//!   Written by [`bake_content`] as `ols1_sprites.bin` (**P4#31**).
//!
//! Header layout matches OLS1 (`sprite_bank`): magic, format_version, data_version,
//! record_count, flags, header_crc32 (reserved 0), then dense records.
//! OLC1/OLT1 remain objects/transitions only — graphics/sound indexes are separate blobs.


use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ol_binary::{
    encode_olc1, encode_olt1, parse_olc1, parse_olt1, peek_format, Olc1Record, Olc1Sprite as BinSprite,
    Olt1Record, Olt1Table, OBJ_F_BLOCKS_WALKING, OBJ_F_CONTAINABLE, OBJ_F_CREATION_SOUND_FORCE,
    OBJ_F_CREATION_SOUND_INITIAL_ONLY, OBJ_F_DRAW_BEHIND_PLAYER, OBJ_F_FLOOR, OBJ_F_FLOOR_HUGGING,
    OBJ_F_HELD_IN_HAND, OBJ_F_NO_BACK_ACCESS, OBJ_F_PERMANENT, OBJ_F_RIDEABLE, OBJ_F_SIDE_ACCESS,
    SPR_BODY_PART_MASK, SPR_BODY_PART_SHIFT, SPR_F_BEHIND_PLAYER, SPR_F_BEHIND_SLOTS, SPR_F_H_FLIP,
    SPR_F_INVIS_HOLDING, SPR_F_INVIS_WORN, SPR_PART_BACK_FOOT, SPR_PART_BODY, SPR_PART_FRONT_FOOT,
    SPR_PART_HEAD, SPR_PART_NONE,
};
use sha1::{Digest, Sha1};

use crate::anim_bank::bake_ola1_from_dir;
use crate::content::{
    apply_default_switch_number_of_uses_patches, apply_object_description_tags,
    apply_sprite_use_vis, description_has_var_numeral, parse_variable_dollar_count,
    var_object_label, var_object_numeral, variable_target_is_hidden, ClientContent,
    ClientObjectDef, ClientTransition, ObjectSprite,
};
use crate::ground_sprites::bake_olg1_from_roots;
use crate::overlay_bank::bake_olo1_from_root;
use crate::sound_bank::bake_olsn_from_dir;
use crate::sprite_bank::bake_ols1_from_dir;

// Re-export shared format constants (stable client API surface).
pub use ol_binary::{
    olt1_lacks_category_expanded, peek_blob_flags, OLC1_FORMAT_VERSION, OLC1_FORMAT_VERSION_V1,
    OLC1_FORMAT_VERSION_V2, OLC1_FORMAT_VERSION_V3, OLC1_FORMAT_VERSION_V4, OLC1_FORMAT_VERSION_V5,
    OLC1_FORMAT_VERSION_V6, OLC1_FORMAT_VERSION_V7, OLC1_MAGIC, OLT1_FORMAT_VERSION,
    OLT1_FORMAT_VERSION_V1, OLT1_F_CATEGORY_EXPANDED, OLT1_MAGIC,
};

/// Manifest format version (`cache/manifest.json`).
pub const MANIFEST_FORMAT: u32 = 1;

/// Per-phase wall times for [`bake_content`] (and CLI reporting).
///
/// Sprite **pixel** atlas pages are **not** part of default bake (**P4#40 OLSA** is
/// optional via `--bake-sprite-atlas` / [`crate::sprite_bank::OlsaBakeStats`]);
/// `ols1` here is meta-only. Ground **OLGA** pixel pages are a separate CLI path
/// (`--bake-ground-atlas`) with their own timings on [`crate::ground_sprites::OlgaBakeStats`].
#[derive(Debug, Clone, Default)]
pub struct BakeTimings {
    pub text_load: Duration,
    pub dummies: Duration,
    pub olc1: Duration,
    pub olt1: Duration,
    pub ola1: Duration,
    pub olg1: Duration,
    pub olo1: Duration,
    pub olsn: Duration,
    /// OLS1 sprite **meta** bake (no TGA pixels / atlas pages).
    pub ols1: Duration,
    pub write_blobs: Duration,
    pub total: Duration,
}

impl BakeTimings {
    /// Human-readable multi-line breakdown (CLI / logs).
    pub fn report_lines(&self) -> String {
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        format!(
            "  {:>8.1} ms  text_load\n\
               {:>8.1} ms  dummies\n\
               {:>8.1} ms  olc1\n\
               {:>8.1} ms  olt1\n\
               {:>8.1} ms  ola1\n\
               {:>8.1} ms  olg1\n\
               {:>8.1} ms  olo1\n\
               {:>8.1} ms  olsn\n\
               {:>8.1} ms  ols1_meta (no pixel pages; P4#40)\n\
               {:>8.1} ms  write_blobs\n\
               {:>8.1} ms  total",
            ms(self.text_load),
            ms(self.dummies),
            ms(self.olc1),
            ms(self.olt1),
            ms(self.ola1),
            ms(self.olg1),
            ms(self.olo1),
            ms(self.olsn),
            ms(self.ols1),
            ms(self.write_blobs),
            ms(self.total),
        )
    }
}

/// Result of baking content into a cache directory.
#[derive(Debug, Clone)]
pub struct BakeResult {
    pub data_version: i32,
    pub object_count: usize,
    pub transition_count: usize,
    pub anim_count: usize,
    pub ground_count: usize,
    pub overlay_count: usize,
    pub sound_count: usize,
    /// OLS1 sprite meta records (`ols1_sprites.bin`).
    pub sprite_count: usize,
    pub dummy_count: usize,
    pub cache_dir: PathBuf,
    pub olc1_bytes: usize,
    pub olt1_bytes: usize,
    pub ola1_bytes: usize,
    pub olg1_bytes: usize,
    pub olo1_bytes: usize,
    pub olsn_bytes: usize,
    pub ols1_bytes: usize,
    /// Wall-clock breakdown (bake phases).
    pub timings: BakeTimings,
}

/// Blob entry inside `manifest.json`.
#[derive(Debug, Clone)]
pub struct ManifestBlob {
    pub sha1: String,
    pub bytes: u64,
    pub count: u32,
}

/// Parsed `cache/manifest.json` (minimal; no full serde dependency).
#[derive(Debug, Clone)]
pub struct ContentManifest {
    pub format: u32,
    pub data_version: i32,
    pub created_utc: String,
    pub source: String,
    pub olc1: Option<ManifestBlob>,
    pub olt1: Option<ManifestBlob>,
    pub ola1: Option<ManifestBlob>,
    pub olg1: Option<ManifestBlob>,
    pub olo1: Option<ManifestBlob>,
    pub olsn: Option<ManifestBlob>,
    pub ols1: Option<ManifestBlob>,
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn sha1_hex(data: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(data);
    hex::encode(h.finalize())
}

// ── OLC1 (via shared ol_binary) ──────────────────────────────────────────────

/// Serialize objects (+ multi-use / variable dummy id lists) to OLC1 bytes.
/// Skips runtime-materialized dummy records (`dummy_parent` / `variable_dummy_parent`).
pub fn write_olc1(db: &ClientContent) -> Vec<u8> {
    let mut ids: Vec<i32> = db
        .objects
        .iter()
        .filter(|(_, d)| d.dummy_parent == 0 && d.variable_dummy_parent == 0)
        .map(|(id, _)| *id)
        .collect();
    ids.sort_unstable();
    let records: Vec<Olc1Record> = ids
        .iter()
        .map(|id| client_object_to_olc1(db.objects.get(id).expect("key")))
        .collect();
    encode_olc1(db.data_version as u32, 0, &records)
}

fn client_object_to_olc1(def: &ClientObjectDef) -> Olc1Record {
    let mut flags = 0u32;
    if def.permanent {
        flags |= OBJ_F_PERMANENT;
    }
    if def.blocks_walking {
        flags |= OBJ_F_BLOCKS_WALKING;
    }
    if def.containable {
        flags |= OBJ_F_CONTAINABLE;
    }
    if def.floor {
        flags |= OBJ_F_FLOOR;
    }
    if def.draw_behind_player {
        flags |= OBJ_F_DRAW_BEHIND_PLAYER;
    }
    if def.held_in_hand {
        flags |= OBJ_F_HELD_IN_HAND;
    }
    if def.rideable {
        flags |= OBJ_F_RIDEABLE;
    }
    if def.side_access {
        flags |= OBJ_F_SIDE_ACCESS;
    }
    if def.no_back_access {
        flags |= OBJ_F_NO_BACK_ACCESS;
    }
    if def.creation_sound_initial_only {
        flags |= OBJ_F_CREATION_SOUND_INITIAL_ONLY;
    }
    if def.floor_hugging {
        flags |= OBJ_F_FLOOR_HUGGING;
    }
    if def.creation_sound_force {
        flags |= OBJ_F_CREATION_SOUND_FORCE;
    }
    let sprites: Vec<BinSprite> = def.sprites.iter().map(client_sprite_to_bin).collect();
    let use_vanish_idx: Vec<i32> = def
        .sprites
        .iter()
        .enumerate()
        .filter(|(_, s)| s.use_vanish)
        .map(|(i, _)| i as i32)
        .collect();
    let use_appear_idx: Vec<i32> = def
        .sprites
        .iter()
        .enumerate()
        .filter(|(_, s)| s.use_appear)
        .map(|(i, _)| i as i32)
        .collect();
    Olc1Record {
        id: def.id,
        name: def.name.clone(),
        description: def.description.clone(),
        flags,
        food_value: def.food_value,
        num_uses: def.num_uses,
        min_pickup_age: def.min_pickup_age,
        person: def.person,
        held_x: def.held_offset.0,
        held_y: def.held_offset.1,
        clothing: def.clothing as u8,
        cloth_x: def.clothing_offset.0,
        cloth_y: def.clothing_offset.1,
        num_slots: def.num_slots,
        // OLC1 v8 trailer; client object def does not store these yet.
        contain_size: 0.0,
        slot_size: 1.0,
        slot_pos: def.slot_pos.clone(),
        sprites,
        dummy_ids: def.dummy_ids.clone(),
        dummy_parent: def.dummy_parent,
        use_chance: def.use_chance,
        left_blocking_radius: def.left_blocking_radius,
        right_blocking_radius: def.right_blocking_radius,
        map_chance: def.map_chance,
        heat_value: def.heat_value,
        speed_mult: def.speed_mult,
        r_value: def.r_value,
        decay_factor: def.decay_factor,
        decays_to_obj: def.decays_to_obj,
        winter_decay_factor: def.winter_decay_factor,
        spring_regrow_factor: def.spring_regrow_factor,
        biomes: def.biomes.clone(),
        creation_sound: def.creation_sound.clone(),
        using_sound: def.using_sound.clone(),
        eating_sound: def.eating_sound.clone(),
        decay_sound: def.decay_sound.clone(),
        use_vanish_idx,
        use_appear_idx,
        variable_dummy_ids: def.variable_dummy_ids.clone(),
        // OLC1 v7 trailer — Haxe ObjectData.writeToFile deadlyDistance/useDistance + moves.
        // Haxe: ObjectData.writeToFile / readFromFile
        deadly_distance: def.deadly_distance,
        use_distance: def.use_distance,
        moves: def.moves,
    }
}

fn client_sprite_to_bin(s: &ObjectSprite) -> BinSprite {
    let mut sf = 0u8;
    if s.h_flip {
        sf |= SPR_F_H_FLIP;
    }
    if s.invis_holding {
        sf |= SPR_F_INVIS_HOLDING;
    }
    if s.invis_worn {
        sf |= SPR_F_INVIS_WORN;
    }
    if s.behind_slots {
        sf |= SPR_F_BEHIND_SLOTS;
    }
    if s.behind_player {
        sf |= SPR_F_BEHIND_PLAYER;
    }
    let part = if s.is_body {
        SPR_PART_BODY
    } else if s.is_head {
        SPR_PART_HEAD
    } else if s.is_back_foot {
        SPR_PART_BACK_FOOT
    } else if s.is_front_foot {
        SPR_PART_FRONT_FOOT
    } else {
        SPR_PART_NONE
    };
    sf |= (part << SPR_BODY_PART_SHIFT) & SPR_BODY_PART_MASK;
    BinSprite {
        sprite_id: s.sprite_id,
        x: s.x,
        y: s.y,
        rot: s.rot,
        flags: sf,
        age_start: s.age_start,
        age_end: s.age_end,
        r: s.r,
        g: s.g,
        b: s.b,
        parent: s.parent,
    }
}

/// Load OLC1 into a new / existing [`ClientContent`] (replaces objects, sets data_version).
/// Accepts format 1..=7. Runtime-materializes multi-use + variable dummy object records.
pub fn load_olc1(data: &[u8], db: &mut ClientContent) -> Result<u32, String> {
    let blob = parse_olc1(data)?;
    let fmt = blob.header.format;
    let data_version = blob.header.data_version;
    db.objects.clear();
    db.dummy_parent.clear();
    db.data_version = data_version as i32;
    for rec in blob.records {
        let def = olc1_to_client_object(rec);
        for &did in &def.dummy_ids {
            db.dummy_parent.insert(did, def.id);
        }
        if def.dummy_parent != 0 {
            db.dummy_parent.insert(def.id, def.dummy_parent);
        }
        db.objects.insert(def.id, def);
    }
    materialize_dummy_object_records(db);
    // Format < 6: regenerate `$N` variable dummies from description (legacy cache).
    // Format ≥ 6: materialize from stored `variable_dummy_ids` lists.
    if fmt < OLC1_FORMAT_VERSION_V6 {
        assign_variable_dummies(db);
    } else {
        materialize_variable_dummy_object_records(db);
    }
    Ok(data_version)
}

/// Peek OLC1 format_version from blob header (None if too short / wrong magic).
pub fn peek_olc1_format(data: &[u8]) -> Option<u32> {
    peek_format(data, OLC1_MAGIC)
}

fn olc1_to_client_object(rec: Olc1Record) -> ClientObjectDef {
    let mut draw_behind_player = rec.flags & OBJ_F_DRAW_BEHIND_PLAYER != 0;
    // C++: wide objects force drawBehindPlayer (also applied on text parse).
    if rec.left_blocking_radius > 0 || rec.right_blocking_radius > 0 {
        draw_behind_player = true;
    }
    let mut sprites: Vec<ObjectSprite> = rec
        .sprites
        .iter()
        .map(bin_sprite_to_client)
        .collect();
    // P4#25: apply sparse use vanish/appear indices from format ≥ 5 trailer.
    for idx in &rec.use_vanish_idx {
        if *idx >= 0 {
            if let Some(spr) = sprites.get_mut(*idx as usize) {
                spr.use_vanish = true;
            }
        }
    }
    for idx in &rec.use_appear_idx {
        if *idx >= 0 {
            if let Some(spr) = sprites.get_mut(*idx as usize) {
                spr.use_appear = true;
            }
        }
    }
    let mut def = ClientObjectDef {
        id: rec.id,
        name: rec.name,
        description: rec.description,
        permanent: rec.flags & OBJ_F_PERMANENT != 0,
        blocks_walking: rec.flags & OBJ_F_BLOCKS_WALKING != 0,
        left_blocking_radius: rec.left_blocking_radius,
        right_blocking_radius: rec.right_blocking_radius,
        containable: rec.flags & OBJ_F_CONTAINABLE != 0,
        food_value: rec.food_value,
        num_uses: rec.num_uses,
        use_chance: rec.use_chance,
        min_pickup_age: rec.min_pickup_age,
        person: rec.person,
        floor: rec.flags & OBJ_F_FLOOR != 0,
        draw_behind_player,
        floor_hugging: rec.flags & OBJ_F_FLOOR_HUGGING != 0,
        wall_layer: false,
        front_wall: false,
        map_chance: rec.map_chance,
        biomes: rec.biomes,
        heat_value: rec.heat_value,
        r_value: rec.r_value,
        speed_mult: rec.speed_mult,
        decay_factor: rec.decay_factor,
        decays_to_obj: rec.decays_to_obj,
        winter_decay_factor: rec.winter_decay_factor,
        spring_regrow_factor: rec.spring_regrow_factor,
        held_offset: (rec.held_x, rec.held_y),
        contain_offset: (0, 0),
        held_in_hand: rec.flags & OBJ_F_HELD_IN_HAND != 0,
        rideable: rec.flags & OBJ_F_RIDEABLE != 0,
        side_access: rec.flags & OBJ_F_SIDE_ACCESS != 0,
        no_back_access: rec.flags & OBJ_F_NO_BACK_ACCESS != 0,
        clothing: rec.clothing as char,
        clothing_offset: (rec.cloth_x, rec.cloth_y),
        num_slots: rec.num_slots,
        slot_pos: rec.slot_pos,
        sprites,
        dummy_ids: rec.dummy_ids,
        dummy_parent: rec.dummy_parent,
        variable_dummy_ids: rec.variable_dummy_ids,
        variable_dummy_parent: 0,
        is_variable_hidden: false,
        creation_sound: rec.creation_sound,
        using_sound: rec.using_sound,
        eating_sound: rec.eating_sound,
        decay_sound: rec.decay_sound,
        creation_sound_initial_only: rec.flags & OBJ_F_CREATION_SOUND_INITIAL_ONLY != 0,
        creation_sound_force: rec.flags & OBJ_F_CREATION_SOUND_FORCE != 0,
        main_eyes_offset: (0.0, 0.0),
        // OLC1 has no homeMarker bit yet — set via description (`eveHomeMarker`) below.
        home_marker: false,
        // OLC1 v7 (format < 7 → parse defaults 0/1/0).
        // Haxe: ObjectData.deadlyDistance / useDistance / moves
        deadly_distance: rec.deadly_distance,
        use_distance: rec.use_distance,
        moves: rec.moves,
    };
    // P3#21: +containOffsetX_/Y_ live in description (not OLC1 fields).
    // L-HUD: homeMarker recovered from eveHomeMarker in description.
    apply_object_description_tags(&mut def);
    def
}

fn bin_sprite_to_client(s: &BinSprite) -> ObjectSprite {
    let part = s.body_part();
    ObjectSprite {
        sprite_id: s.sprite_id,
        x: s.x,
        y: s.y,
        rot: s.rot,
        h_flip: s.h_flip(),
        age_start: s.age_start,
        age_end: s.age_end,
        r: s.r,
        g: s.g,
        b: s.b,
        parent: s.parent,
        invis_holding: s.invis_holding(),
        invis_worn: s.invis_worn(),
        // only_when_worn (invisWorn=2) not in OLC1 sprite flags — residual text-path.
        only_when_worn: false,
        behind_slots: s.behind_slots(),
        behind_player: s.behind_player(),
        is_body: part == SPR_PART_BODY,
        is_head: part == SPR_PART_HEAD,
        is_back_foot: part == SPR_PART_BACK_FOOT,
        is_front_foot: part == SPR_PART_FRONT_FOOT,
        is_eyes: false,
        is_mouth: false,
        use_vanish: false,
        use_appear: false,
        skip_drawing: false,
    }
}

// ── OLT1 (via shared ol_binary) ──────────────────────────────────────────────

fn client_transition_to_olt1(tr: &ClientTransition, max_use_table: bool) -> Olt1Record {
    Olt1Record {
        actor_id: tr.actor_id,
        target_id: tr.target_id,
        new_actor_id: tr.new_actor_id,
        new_target_id: tr.new_target_id,
        flags: Olt1Record::pack_flags(
            tr.last_use_actor,
            tr.last_use_target,
            tr.reverse_use_actor,
            tr.reverse_use_target,
            tr.no_use_actor,
            tr.no_use_target,
            max_use_table,
            tr.switch_number_of_uses,
        ),
        auto_decay_seconds: tr.auto_decay_seconds,
        actor_min_use_fraction: tr.actor_min_use_fraction,
        target_min_use_fraction: tr.target_min_use_fraction,
        move_dist: tr.move_dist,
        desired_move_dist: tr.desired_move_dist,
    }
}

/// Serialize transitions (normal + last-use + max-use) to OLT1 format-2 bytes.
///
/// Sets header flag [`OLT1_F_CATEGORY_EXPANDED`] when
/// [`ClientContent::transitions_category_expanded`] is true (normal after
/// [`ClientContent::load_from_dir`] / bake). Load then skips re-expand.
/// Max-use rows use record flag bit6; switch flag uses bit7 (**P4#29**).
pub fn write_olt1(db: &ClientContent) -> Vec<u8> {
    let mut normal: Vec<(i32, i32)> = db.transitions.keys().copied().collect();
    normal.sort_unstable();
    let mut last: Vec<(i32, i32)> = db.transitions_last_use.keys().copied().collect();
    last.sort_unstable();
    let mut max_use: Vec<(i32, i32)> = db.transitions_max_use.keys().copied().collect();
    max_use.sort_unstable();
    let mut records = Vec::with_capacity(normal.len() + last.len() + max_use.len());
    for k in &normal {
        records.push(client_transition_to_olt1(
            db.transitions.get(k).expect("key"),
            false,
        ));
    }
    for k in &last {
        records.push(client_transition_to_olt1(
            db.transitions_last_use.get(k).expect("key"),
            false,
        ));
    }
    for k in &max_use {
        records.push(client_transition_to_olt1(
            db.transitions_max_use.get(k).expect("key"),
            true,
        ));
    }
    let flags = if db.transitions_category_expanded {
        OLT1_F_CATEGORY_EXPANDED
    } else {
        0
    };
    encode_olt1(db.data_version as u32, flags, &records)
}

/// Load OLT1 into `db.transitions` + `transitions_last_use` + `transitions_max_use`
/// (replaces all three). Format 1 = craft subset; format 2 = full reverse/move/min-use.
///
/// Sets [`ClientContent::transitions_category_expanded`] from header flags.
/// Routes records by last-use bits and max-use bit6; restores `switch_number_of_uses`
/// from bit7. Legacy blobs without max-use/switch bits still load correctly.
pub fn load_olt1(data: &[u8], db: &mut ClientContent) -> Result<u32, String> {
    let blob = parse_olt1(data)?;
    let data_version = blob.header.data_version;
    db.transitions.clear();
    db.transitions_last_use.clear();
    db.transitions_max_use.clear();
    db.transitions_category_expanded =
        blob.header.flags & OLT1_F_CATEGORY_EXPANDED != 0;
    if db.data_version == 0 {
        db.data_version = data_version as i32;
    }
    for rec in blob.records {
        let tr = ClientTransition {
            actor_id: rec.actor_id,
            target_id: rec.target_id,
            new_actor_id: rec.new_actor_id,
            new_target_id: rec.new_target_id,
            last_use_actor: rec.last_use_actor(),
            last_use_target: rec.last_use_target(),
            auto_decay_seconds: rec.auto_decay_seconds,
            reverse_use_actor: rec.reverse_use_actor(),
            reverse_use_target: rec.reverse_use_target(),
            no_use_actor: rec.no_use_actor(),
            no_use_target: rec.no_use_target(),
            move_dist: rec.move_dist,
            desired_move_dist: rec.desired_move_dist,
            actor_min_use_fraction: rec.actor_min_use_fraction,
            target_min_use_fraction: rec.target_min_use_fraction,
            switch_number_of_uses: rec.switch_number_of_uses(),
        };
        let key = (rec.actor_id, rec.target_id);
        match rec.table() {
            Olt1Table::LastUse => {
                db.transitions_last_use.insert(key, tr);
            }
            Olt1Table::MaxUse => {
                db.transitions_max_use.insert(key, tr);
            }
            Olt1Table::Normal => {
                db.transitions.insert(key, tr);
            }
        }
    }
    Ok(data_version)
}

// ── multi-use dummies (Haxe ObjectBake / server assign_multi_use_dummies) ─────

/// Allocate dummy object ids for `num_uses >= 2` parents.
///
/// // C++: `autoGenerateUsedObjects` / Haxe: `ObjectBake`
pub fn assign_multi_use_dummies(db: &mut ClientContent) {
    let stale: Vec<i32> = db
        .objects
        .iter()
        .filter(|(_, d)| d.dummy_parent != 0)
        .map(|(id, _)| *id)
        .collect();
    for id in stale {
        db.objects.remove(&id);
    }
    db.dummy_parent.clear();
    for def in db.objects.values_mut() {
        def.dummy_ids.clear();
        def.dummy_parent = 0;
    }

    let mut next = db
        .root
        .as_ref()
        .and_then(|r| fs::read_to_string(r.join("objects").join("nextObjectNumber.txt")).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let max_id = db.objects.keys().copied().max().unwrap_or(0);
    if next <= max_id {
        next = max_id + 1;
    }

    let mut multi: Vec<i32> = db
        .objects
        .iter()
        .filter(|(_, d)| d.num_uses >= 2 && d.dummy_parent == 0)
        .map(|(id, _)| *id)
        .collect();
    multi.sort_unstable();

    for id in multi {
        let n = db.objects.get(&id).map(|d| d.num_uses).unwrap_or(0);
        if n < 2 {
            continue;
        }
        let mut dummies = Vec::with_capacity((n - 1) as usize);
        for _ in 0..(n - 1) {
            let did = next;
            next += 1;
            dummies.push(did);
            db.dummy_parent.insert(did, id);
        }
        if let Some(def) = db.objects.get_mut(&id) {
            def.dummy_ids = dummies;
        }
    }
}

/// Insert lightweight object records for multi-use dummy ids so `get(dummy_id)`
/// works for soft-FB draw (sprites/flags copied from parent).
///
/// Also applies C++ `setupSpriteUseVis` so each dummy (and the full parent) gets
/// per-use `spriteSkipDrawing` stages — berries vanish / shells appear as uses
/// deplete, not a single static sprite set.
///
/// // Haxe: `CreateAndAddDummyObjectData`
/// // C++: autoGenerateUsedObjects + setupSpriteUseVis on useDummyIDs
pub fn materialize_dummy_object_records(db: &mut ClientContent) {
    let parents: Vec<(i32, Vec<i32>)> = db
        .objects
        .iter()
        .filter(|(_, d)| !d.dummy_ids.is_empty() && d.dummy_parent == 0)
        .map(|(id, d)| (*id, d.dummy_ids.clone()))
        .collect();
    for (pid, dummies) in parents {
        let Some(parent) = db.objects.get(&pid).cloned() else {
            continue;
        };
        let num_uses = parent.num_uses;
        let parent_sprites = parent.sprites.clone();

        // C++: hide all appearing sprites on the full parent object.
        if let Some(p) = db.objects.get_mut(&pid) {
            apply_sprite_use_vis(p, &parent_sprites, num_uses, num_uses);
        }

        for (di, &did) in dummies.iter().enumerate() {
            // C++: useDummyIDs[d-1] with uses remaining d = 1..numUses-1
            let uses_remaining = (di as i32) + 1;
            if let Some(existing) = db.objects.get_mut(&did) {
                // Re-apply vis when record already present (e.g. re-materialize).
                apply_sprite_use_vis(existing, &parent_sprites, num_uses, uses_remaining);
                existing.dummy_parent = pid;
                existing.dummy_ids.clear();
                existing.num_uses = 0;
                existing.variable_dummy_ids.clear();
                existing.variable_dummy_parent = 0;
                existing.is_variable_hidden = false;
                // C++: only last dummy (d==1) keeps creation when initial-only.
                if parent.creation_sound_initial_only && uses_remaining != 1 {
                    existing.creation_sound.clear();
                }
                db.dummy_parent.insert(did, pid);
                continue;
            }
            let mut d = parent.clone();
            d.id = did;
            d.dummy_ids.clear();
            d.dummy_parent = pid;
            // C++: dummies set numUses=0 so we don't recurse.
            d.num_uses = 0;
            d.map_chance = 0.0;
            if parent.creation_sound_initial_only && uses_remaining != 1 {
                d.creation_sound.clear();
            }
            apply_sprite_use_vis(&mut d, &parent_sprites, num_uses, uses_remaining);
            // Variable-dummy fields are parent-only; multi-use dummies are not variable.
            d.variable_dummy_ids.clear();
            d.variable_dummy_parent = 0;
            d.is_variable_hidden = false;
            db.objects.insert(did, d);
            db.dummy_parent.insert(did, pid);
        }
    }
}

// ── variable dummies (C++ autoGenerateVariableObjects / variableDummyIDs) ─────

/// Next free object id after real objects + multi-use + variable dummy lists.
fn next_free_object_id(db: &ClientContent) -> i32 {
    let mut next = db
        .root
        .as_ref()
        .and_then(|r| fs::read_to_string(r.join("objects").join("nextObjectNumber.txt")).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let max_id = db.objects.keys().copied().max().unwrap_or(0);
    if next <= max_id {
        next = max_id + 1;
    }
    for def in db.objects.values() {
        for &did in def.dummy_ids.iter().chain(def.variable_dummy_ids.iter()) {
            if did >= next {
                next = did + 1;
            }
        }
    }
    next
}

/// Allocate C++ `variableDummyIDs` for parents whose description contains `$N` (N≥2).
///
/// Continues the free-id counter after multi-use dummies (same order as C++:
/// `autoGenerateUsedObjects` then `autoGenerateVariableObjects`). Rewrites parent
/// description `$N` → `- ?`. Materializes dummy records with letter or numeral labels.
///
/// // C++: `autoGenerateVariableObjects` + `reAddObject`
/// // TODO: `setupNumericSprites` for `+varNumeral` (needs sprite-bank Numeral# tags)
/// // TODO: `getNextVarSerialNumberChild` / `+varSerialNumber` instance cycling
pub fn assign_variable_dummies(db: &mut ClientContent) {
    // Drop previously materialized variable dummies.
    let stale: Vec<i32> = db
        .objects
        .iter()
        .filter(|(_, d)| d.variable_dummy_parent != 0)
        .map(|(id, _)| *id)
        .collect();
    for id in stale {
        db.objects.remove(&id);
    }
    for def in db.objects.values_mut() {
        def.variable_dummy_ids.clear();
        def.variable_dummy_parent = 0;
        def.is_variable_hidden = false;
    }

    let mut next = next_free_object_id(db);

    // C++ iterates idMap in ascending id order.
    let mut parents: Vec<i32> = db
        .objects
        .iter()
        .filter(|(_, d)| {
            d.dummy_parent == 0
                && d.variable_dummy_parent == 0
                && parse_variable_dollar_count(&d.description).is_some()
        })
        .map(|(id, _)| *id)
        .collect();
    parents.sort_unstable();

    for pid in parents {
        let Some(parent) = db.objects.get(&pid).cloned() else {
            continue;
        };
        let Some((dollar_idx, num_var)) = parse_variable_dollar_count(&parent.description) else {
            continue;
        };
        let target = format!("${num_var}");
        // C++ uses first `$N` only; ensure the scanned N matches the replace target.
        if !parent.description[dollar_idx..].starts_with(&target) {
            continue;
        }
        let numeric_label = description_has_var_numeral(&parent.description);
        let variable_hidden = variable_target_is_hidden(&parent.description, dollar_idx);

        let mut dummies = Vec::with_capacity(num_var as usize);
        for d in 1..=num_var {
            let sub = if numeric_label {
                var_object_numeral(d, num_var)
            } else {
                var_object_label(d)
            };
            let desc = parent.description.replacen(&target, &sub, 1);
            let did = next;
            next += 1;
            dummies.push(did);

            let mut dummy = parent.clone();
            dummy.id = did;
            dummy.description = desc;
            // Keep short name in sync with description head when possible.
            if let Some(head) = dummy.description.split('#').next() {
                let head = head.trim();
                if !head.is_empty() {
                    dummy.name = head.to_string();
                }
            }
            dummy.dummy_ids.clear();
            dummy.dummy_parent = 0;
            dummy.variable_dummy_ids.clear();
            dummy.variable_dummy_parent = pid;
            dummy.is_variable_hidden = variable_hidden;
            // Variable dummies are not multi-use parents.
            dummy.num_uses = parent.num_uses;
            // TODO: setupNumericSprites(dummy, d, num_var) when sprite bank available.
            db.objects.insert(did, dummy);
        }

        if let Some(def) = db.objects.get_mut(&pid) {
            def.variable_dummy_ids = dummies;
            // C++: replace `$N` with "- ?" on the parent description.
            def.description = parent.description.replacen(&target, "- ?", 1);
        }
    }
}

/// Insert object records for stored `variable_dummy_ids` (OLC1 format ≥ 6 load path).
///
/// Parent description is expected to already contain `- ?` (post-assign rewrite).
/// Dummy labels are reconstructed from index + `+varNumeral`.
///
/// // C++: reAddObject path at autoGenerateVariableObjects time
pub fn materialize_variable_dummy_object_records(db: &mut ClientContent) {
    let parents: Vec<(i32, Vec<i32>)> = db
        .objects
        .iter()
        .filter(|(_, d)| !d.variable_dummy_ids.is_empty() && d.variable_dummy_parent == 0)
        .map(|(id, d)| (*id, d.variable_dummy_ids.clone()))
        .collect();

    for (pid, dummies) in parents {
        let Some(parent) = db.objects.get(&pid).cloned() else {
            continue;
        };
        let num_var = dummies.len() as i32;
        if num_var < 1 {
            continue;
        }
        let numeric_label = description_has_var_numeral(&parent.description);
        // Hidden if `- ?` (or residual `$N`) sits after `#`.
        let target_idx = parent
            .description
            .find("- ?")
            .or_else(|| parent.description.find('$'))
            .unwrap_or(0);
        let variable_hidden = variable_target_is_hidden(&parent.description, target_idx);

        for (di, &did) in dummies.iter().enumerate() {
            let d_1based = (di as i32) + 1;
            let sub = if numeric_label {
                var_object_numeral(d_1based, num_var)
            } else {
                var_object_label(d_1based)
            };
            let desc = if parent.description.contains("- ?") {
                parent.description.replacen("- ?", &sub, 1)
            } else if let Some((_, n)) = parse_variable_dollar_count(&parent.description) {
                let target = format!("${n}");
                parent.description.replacen(&target, &sub, 1)
            } else {
                format!("{} {}", parent.description, sub)
            };

            if let Some(existing) = db.objects.get_mut(&did) {
                existing.variable_dummy_parent = pid;
                existing.variable_dummy_ids.clear();
                existing.is_variable_hidden = variable_hidden;
                existing.description = desc;
                continue;
            }
            let mut dummy = parent.clone();
            dummy.id = did;
            dummy.description = desc;
            if let Some(head) = dummy.description.split('#').next() {
                let head = head.trim();
                if !head.is_empty() {
                    dummy.name = head.to_string();
                }
            }
            dummy.dummy_ids.clear();
            // Keep multi-use parent link empty; variable is separate from use dummies.
            dummy.dummy_parent = 0;
            dummy.variable_dummy_ids.clear();
            dummy.variable_dummy_parent = pid;
            dummy.is_variable_hidden = variable_hidden;
            // TODO: setupNumericSprites for +varNumeral numeral digit sprites.
            db.objects.insert(did, dummy);
        }
    }
}

// ── bake / load cache ────────────────────────────────────────────────────────

/// Default cache subdirectory under a content root.
pub fn cache_dir_for(root: &Path) -> PathBuf {
    root.join("cache")
}

/// Bake text content → `out_dir` (`olc1_objects.bin`, `olt1_transitions.bin`,
/// `ola1_anims.bin`, `olg1_ground_index.bin`, `olo1_overlays.bin`,
/// `olsn_sounds.bin`, `ols1_sprites.bin`, `manifest.json`).
///
/// `src` is a OneLifeData7-style root. Assigns multi-use + variable dummies before write.
/// Animations come from `src/animations/` (client-heavy OLA1).
/// Ground index scans `src` + default game-data roots (`groundTileCache/`,
/// `graphics/ground_tN.tga`).
/// Overlay index scans `src/overlays/{tag}/{id}.tga` (OLO1; lazy TGA at runtime).
/// Sound index scans `src/sounds/*.{aiff,ogg}` only (OLSN — no PCM).
/// Sprite meta scans `src/sprites/{id}.txt` (+ TGA header w/h; OLS1 — no pixels).
pub fn bake_content(src: impl AsRef<Path>, out_dir: impl AsRef<Path>) -> Result<BakeResult, String> {
    bake_content_with_progress(src, out_dir, None)
}

/// Full content bake with optional loading UI ticks between phases.
pub fn bake_content_with_progress(
    src: impl AsRef<Path>,
    out_dir: impl AsRef<Path>,
    mut on_progress: crate::load_progress::ProgressCb<'_>,
) -> Result<BakeResult, String> {
    use crate::load_progress::{report_stage, LoadStage};

    let src = src.as_ref();
    let out_dir = out_dir.as_ref();
    let total0 = Instant::now();
    let mut timings = BakeTimings::default();

    let tick = |frac: f32, detail: &str, on_progress: &mut crate::load_progress::ProgressCb<'_>| {
        report_stage(
            LoadStage::Content,
            frac.clamp(0.0, 0.99),
            Some(detail),
            crate::load_progress::reborrow_cb(on_progress),
        );
    };

    tick(0.08, "bake: text objects…", &mut on_progress);
    let t0 = Instant::now();
    let mut db = ClientContent::load_from_dir(src)?;
    timings.text_load = t0.elapsed();

    // C++ order: autoGenerateUsedObjects then autoGenerateVariableObjects.
    tick(0.22, "bake: multi-use / variable dummies…", &mut on_progress);
    let t0 = Instant::now();
    assign_multi_use_dummies(&mut db);
    assign_variable_dummies(&mut db);
    timings.dummies = t0.elapsed();

    let dummy_count = db.dummy_parent.len()
        + db
            .objects
            .values()
            .filter(|d| d.variable_dummy_parent != 0)
            .count();
    let object_count = db
        .objects
        .values()
        .filter(|d| d.dummy_parent == 0 && d.variable_dummy_parent == 0)
        .count();
    let transition_count = db.transitions.len()
        + db.transitions_last_use.len()
        + db.transitions_max_use.len();

    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;

    tick(0.35, "bake: OLC1 objects…", &mut on_progress);
    let t0 = Instant::now();
    let olc1 = write_olc1(&db);
    timings.olc1 = t0.elapsed();

    // switchNumberOfUses already applied in `load_from_dir`; re-apply is idempotent.
    apply_default_switch_number_of_uses_patches(&mut db);

    tick(0.45, "bake: OLT1 transitions…", &mut on_progress);
    let t0 = Instant::now();
    let olt1 = write_olt1(&db);
    timings.olt1 = t0.elapsed();

    tick(0.55, "bake: OLA1 animations…", &mut on_progress);
    let t0 = Instant::now();
    let (ola1, anim_count) = bake_ola1_from_dir(src, db.data_version as u32)?;
    timings.ola1 = t0.elapsed();

    tick(0.65, "bake: OLG1 ground…", &mut on_progress);
    let t0 = Instant::now();
    let (olg1, ground_count) = bake_olg1_from_roots(Some(src), db.data_version as u32);
    timings.olg1 = t0.elapsed();

    tick(0.72, "bake: OLO1 overlays…", &mut on_progress);
    let t0 = Instant::now();
    let (olo1, overlay_count) = bake_olo1_from_root(src, db.data_version as u32);
    timings.olo1 = t0.elapsed();

    tick(0.80, "bake: OLSN sounds…", &mut on_progress);
    let t0 = Instant::now();
    let (olsn, sound_count) = bake_olsn_from_dir(src, db.data_version as u32)?;
    timings.olsn = t0.elapsed();

    tick(0.88, "bake: OLS1 sprite meta…", &mut on_progress);
    let t0 = Instant::now();
    let (ols1, sprite_count) = bake_ols1_from_dir(src, db.data_version as u32)?;
    timings.ols1 = t0.elapsed();

    tick(0.94, "bake: write cache…", &mut on_progress);
    let t0 = Instant::now();
    let olc1_path = out_dir.join("olc1_objects.bin");
    let olt1_path = out_dir.join("olt1_transitions.bin");
    let ola1_path = out_dir.join("ola1_anims.bin");
    let olg1_path = out_dir.join("olg1_ground_index.bin");
    let olo1_path = out_dir.join("olo1_overlays.bin");
    let olsn_path = out_dir.join("olsn_sounds.bin");
    let ols1_path = out_dir.join("ols1_sprites.bin");
    fs::write(&olc1_path, &olc1).map_err(|e| e.to_string())?;
    fs::write(&olt1_path, &olt1).map_err(|e| e.to_string())?;
    fs::write(&ola1_path, &ola1).map_err(|e| e.to_string())?;
    fs::write(&olg1_path, &olg1).map_err(|e| e.to_string())?;
    fs::write(&olo1_path, &olo1).map_err(|e| e.to_string())?;
    fs::write(&olsn_path, &olsn).map_err(|e| e.to_string())?;
    fs::write(&ols1_path, &ols1).map_err(|e| e.to_string())?;

    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let created_utc = format!("unix:{created}");
    let source = src
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("OneLifeData7");

    let manifest = format!(
        r#"{{
  "format": {fmt},
  "data_version": {ver},
  "created_utc": "{created}",
  "source": "{source}",
  "blobs": {{
    "olc1_objects.bin": {{ "sha1": "{olc1_sha}", "bytes": {olc1_len}, "count": {olc1_n} }},
    "olt1_transitions.bin": {{ "sha1": "{olt1_sha}", "bytes": {olt1_len}, "count": {olt1_n} }},
    "ola1_anims.bin": {{ "sha1": "{ola1_sha}", "bytes": {ola1_len}, "count": {ola1_n} }},
    "olg1_ground_index.bin": {{ "sha1": "{olg1_sha}", "bytes": {olg1_len}, "count": {olg1_n} }},
    "olo1_overlays.bin": {{ "sha1": "{olo1_sha}", "bytes": {olo1_len}, "count": {olo1_n} }},
    "olsn_sounds.bin": {{ "sha1": "{olsn_sha}", "bytes": {olsn_len}, "count": {olsn_n} }},
    "ols1_sprites.bin": {{ "sha1": "{ols1_sha}", "bytes": {ols1_len}, "count": {ols1_n} }}
  }}
}}
"#,
        fmt = MANIFEST_FORMAT,
        ver = db.data_version,
        created = created_utc,
        source = source,
        olc1_sha = sha1_hex(&olc1),
        olc1_len = olc1.len(),
        olc1_n = object_count,
        olt1_sha = sha1_hex(&olt1),
        olt1_len = olt1.len(),
        olt1_n = transition_count,
        ola1_sha = sha1_hex(&ola1),
        ola1_len = ola1.len(),
        ola1_n = anim_count,
        olg1_sha = sha1_hex(&olg1),
        olg1_len = olg1.len(),
        olg1_n = ground_count,
        olo1_sha = sha1_hex(&olo1),
        olo1_len = olo1.len(),
        olo1_n = overlay_count,
        olsn_sha = sha1_hex(&olsn),
        olsn_len = olsn.len(),
        olsn_n = sound_count,
        ols1_sha = sha1_hex(&ols1),
        ols1_len = ols1.len(),
        ols1_n = sprite_count,
    );
    fs::write(out_dir.join("manifest.json"), manifest).map_err(|e| e.to_string())?;
    timings.write_blobs = t0.elapsed();
    timings.total = total0.elapsed();

    Ok(BakeResult {
        data_version: db.data_version,
        object_count,
        transition_count,
        anim_count,
        ground_count,
        overlay_count,
        sound_count,
        sprite_count,
        dummy_count,
        cache_dir: out_dir.to_path_buf(),
        olc1_bytes: olc1.len(),
        olt1_bytes: olt1.len(),
        ola1_bytes: ola1.len(),
        olg1_bytes: olg1.len(),
        olo1_bytes: olo1.len(),
        olsn_bytes: olsn.len(),
        ols1_bytes: ols1.len(),
        timings,
    })
}

/// Load from a cache directory written by [`bake_content`].
///
/// Verifies manifest `data_version` against optional `expected_data_version`
/// (pass `None` to skip). Checks blob sha1 when manifest present.
///
/// Note: OLA1 / OLS1 are **not** folded into [`ClientContent`]; load via
/// [`crate::anim_bank::AnimBank::load_prefer_cache`] /
/// [`crate::sprite_bank::SpriteBank::load_prefer_cache`].
pub fn load_from_cache(
    cache_dir: impl AsRef<Path>,
    expected_data_version: Option<i32>,
) -> Result<ClientContent, String> {
    let cache_dir = cache_dir.as_ref();
    let man_path = cache_dir.join("manifest.json");
    let manifest = if man_path.exists() {
        Some(parse_manifest(
            &fs::read_to_string(&man_path).map_err(|e| e.to_string())?,
        )?)
    } else {
        None
    };

    if let (Some(m), Some(exp)) = (manifest.as_ref(), expected_data_version) {
        if m.data_version != exp {
            return Err(format!(
                "cache data_version {} != tree {}",
                m.data_version, exp
            ));
        }
    }

    let olc1_path = cache_dir.join("olc1_objects.bin");
    let olt1_path = cache_dir.join("olt1_transitions.bin");
    if !olc1_path.exists() {
        return Err(format!("missing {}", olc1_path.display()));
    }
    let olc1 = fs::read(&olc1_path).map_err(|e| e.to_string())?;
    if let Some(m) = manifest.as_ref().and_then(|m| m.olc1.as_ref()) {
        let h = sha1_hex(&olc1);
        if h != m.sha1 {
            return Err(format!("olc1 sha1 mismatch (want {}, got {})", m.sha1, h));
        }
    }

    let mut db = ClientContent::new();
    load_olc1(&olc1, &mut db)?;

    if olt1_path.exists() {
        let olt1 = fs::read(&olt1_path).map_err(|e| e.to_string())?;
        if let Some(m) = manifest.as_ref().and_then(|m| m.olt1.as_ref()) {
            let h = sha1_hex(&olt1);
            if h != m.sha1 {
                return Err(format!("olt1 sha1 mismatch (want {}, got {})", m.sha1, h));
            }
        }
        load_olt1(&olt1, &mut db)?;
    }

    // Optional: verify ola1 / olg1 / olo1 sha1 when present (client loads separately).
    let ola1_path = cache_dir.join("ola1_anims.bin");
    if ola1_path.exists() {
        if let Some(m) = manifest.as_ref().and_then(|m| m.ola1.as_ref()) {
            let ola1 = fs::read(&ola1_path).map_err(|e| e.to_string())?;
            let h = sha1_hex(&ola1);
            if h != m.sha1 {
                return Err(format!("ola1 sha1 mismatch (want {}, got {})", m.sha1, h));
            }
        }
    }
    let olg1_path = cache_dir.join("olg1_ground_index.bin");
    if olg1_path.exists() {
        if let Some(m) = manifest.as_ref().and_then(|m| m.olg1.as_ref()) {
            let olg1 = fs::read(&olg1_path).map_err(|e| e.to_string())?;
            let h = sha1_hex(&olg1);
            if h != m.sha1 {
                return Err(format!("olg1 sha1 mismatch (want {}, got {})", m.sha1, h));
            }
        }
    }
    let olo1_path = cache_dir.join("olo1_overlays.bin");
    if olo1_path.exists() {
        if let Some(m) = manifest.as_ref().and_then(|m| m.olo1.as_ref()) {
            let olo1 = fs::read(&olo1_path).map_err(|e| e.to_string())?;
            let h = sha1_hex(&olo1);
            if h != m.sha1 {
                return Err(format!("olo1 sha1 mismatch (want {}, got {})", m.sha1, h));
            }
        }
    }
    // Optional: verify olsn sha1 when present (client loads via SoundBank).
    let olsn_path = cache_dir.join("olsn_sounds.bin");
    if olsn_path.exists() {
        if let Some(m) = manifest.as_ref().and_then(|m| m.olsn.as_ref()) {
            let olsn = fs::read(&olsn_path).map_err(|e| e.to_string())?;
            let h = sha1_hex(&olsn);
            if h != m.sha1 {
                return Err(format!("olsn sha1 mismatch (want {}, got {})", m.sha1, h));
            }
        }
    }
    // Optional: verify ols1 sha1 when present (client loads via SpriteBank).
    let ols1_path = cache_dir.join("ols1_sprites.bin");
    if ols1_path.exists() {
        if let Some(m) = manifest.as_ref().and_then(|m| m.ols1.as_ref()) {
            let ols1 = fs::read(&ols1_path).map_err(|e| e.to_string())?;
            let h = sha1_hex(&ols1);
            if h != m.sha1 {
                return Err(format!("ols1 sha1 mismatch (want {}, got {})", m.sha1, h));
            }
        }
    }

    db.root = cache_dir.parent().map(|p| p.to_path_buf());
    // Categories are text-only (not in OLT1). When OLT1 was baked with
    // `OLT1_F_CATEGORY_EXPANDED`, member/pattern rows are already concrete —
    // only load the CategoryBank for prob-set pick / find_ptrans.
    // Legacy/unexpanded OLT1 re-expands (C++ autoGenerateCategoryTransitions).
    if let Some(root) = db.root.clone() {
        if db.transitions_category_expanded {
            db.maybe_load_category_bank_from_root(root);
        } else {
            db.maybe_load_categories_from_root(root);
        }
    }
    // Legacy OLT1 without bit7 still gets dough/masa switch patches.
    apply_default_switch_number_of_uses_patches(&mut db);
    Ok(db)
}

/// Prefer `root/cache` when manifest data_version matches tree; else text + dummies.
///
/// On stale/missing cache with a writable tree, attempts one auto-rebuild via
/// [`bake_content`] then reloads (Haxe-style invalidate when version drifts).
/// Also rebuilds when on-disk OLC1 format is older than [`OLC1_FORMAT_VERSION`]
/// so sim fields / blocking radii land after a client upgrade.
pub fn load_prefer_cache(root: impl AsRef<Path>) -> Result<ClientContent, String> {
    load_prefer_cache_with_progress(root, None)
}

/// Same as [`load_prefer_cache`] with optional P5#36 progress callback.
pub fn load_prefer_cache_with_progress(
    root: impl AsRef<Path>,
    mut on_progress: crate::load_progress::ProgressCb<'_>,
) -> Result<ClientContent, String> {
    use crate::load_progress::{report_stage, LoadStage};

    let root = root.as_ref();
    report_stage(
        LoadStage::Content,
        0.0,
        Some("prefer_cache"),
        crate::load_progress::reborrow_cb(&mut on_progress),
    );

    let tree_ver = read_data_version(root);
    let cache = cache_dir_for(root);
    let olc1_path = cache.join("olc1_objects.bin");

    // 1) Prefer existing cache if it **loads**. Older OLC1/OLT1 formats are still
    //    valid (shared parse accepts v1..=current). Do **not** force a multi-minute
    //    rebake only because format < OLC1_FORMAT_VERSION — that freezes the loading
    //    UI on "rebake" with no intermediate ticks.
    if olc1_path.exists() {
        match load_from_cache(&cache, tree_ver) {
            Ok(db) => {
                let fmt = fs::read(&olc1_path)
                    .ok()
                    .and_then(|b| peek_olc1_format(&b))
                    .unwrap_or(0);
                let detail = if fmt > 0 && fmt < OLC1_FORMAT_VERSION {
                    format!("cache v{fmt} (ok)")
                } else {
                    "cache".into()
                };
                report_stage(
                    LoadStage::Content,
                    1.0,
                    Some(&detail),
                    crate::load_progress::reborrow_cb(&mut on_progress),
                );
                return Ok(db);
            }
            Err(_e) => {
                // fall through: try loose data_version match ignore
            }
        }
        if let Ok(db) = load_from_cache(&cache, None) {
            report_stage(
                LoadStage::Content,
                1.0,
                Some("legacy_cache"),
                crate::load_progress::reborrow_cb(&mut on_progress),
            );
            return Ok(db);
        }
        // Cache present but unreadable → rebuild with phase progress.
        report_stage(
            LoadStage::Content,
            0.1,
            Some("rebake (cache unreadable)"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        if root.join("objects").is_dir()
            && bake_content_with_progress(root, &cache, crate::load_progress::reborrow_cb(&mut on_progress))
                .is_ok()
        {
            if let Ok(db) = load_from_cache(&cache, tree_ver).or_else(|_| load_from_cache(&cache, None))
            {
                report_stage(
                    LoadStage::Content,
                    1.0,
                    Some("rebaked"),
                    crate::load_progress::reborrow_cb(&mut on_progress),
                );
                return Ok(db);
            }
        }
    } else if root.join("objects").is_dir() {
        report_stage(
            LoadStage::Content,
            0.05,
            Some("bake (no cache)"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        if bake_content_with_progress(root, &cache, crate::load_progress::reborrow_cb(&mut on_progress))
            .is_ok()
        {
            if let Ok(db) = load_from_cache(&cache, tree_ver).or_else(|_| load_from_cache(&cache, None))
            {
                report_stage(
                    LoadStage::Content,
                    1.0,
                    Some("baked"),
                    crate::load_progress::reborrow_cb(&mut on_progress),
                );
                return Ok(db);
            }
        }
    }
    report_stage(
        LoadStage::Content,
        0.5,
        Some("text"),
        crate::load_progress::reborrow_cb(&mut on_progress),
    );
    let mut db = ClientContent::load_from_dir(root)?;
    assign_multi_use_dummies(&mut db);
    materialize_dummy_object_records(&mut db);
    assign_variable_dummies(&mut db);
    report_stage(
        LoadStage::Content,
        1.0,
        Some("text"),
        crate::load_progress::reborrow_cb(&mut on_progress),
    );
    Ok(db)
}

/// Tree `dataVersionNumber.txt` (used by load bench + prefer_cache).
pub fn read_data_version(root: &Path) -> Option<i32> {
    fs::read_to_string(root.join("dataVersionNumber.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Minimal JSON scrape for our manifest (avoids serde dep).
pub fn parse_manifest(text: &str) -> Result<ContentManifest, String> {
    let format = json_u32(text, "format").unwrap_or(0);
    let data_version = json_i32(text, "data_version").unwrap_or(0);
    let created_utc = json_string(text, "created_utc").unwrap_or_default();
    let source = json_string(text, "source").unwrap_or_default();

    let olc1 = json_blob_section(text, "olc1_objects.bin");
    let olt1 = json_blob_section(text, "olt1_transitions.bin");
    let ola1 = json_blob_section(text, "ola1_anims.bin");
    let olg1 = json_blob_section(text, "olg1_ground_index.bin");
    let olo1 = json_blob_section(text, "olo1_overlays.bin");
    let olsn = json_blob_section(text, "olsn_sounds.bin");
    let ols1 = json_blob_section(text, "ols1_sprites.bin");

    Ok(ContentManifest {
        format,
        data_version,
        created_utc,
        source,
        olc1,
        olt1,
        ola1,
        olg1,
        olo1,
        olsn,
        ols1,
    })
}

fn json_u32(text: &str, key: &str) -> Option<u32> {
    json_i32(text, key).map(|v| v as u32)
}

fn json_i32(text: &str, key: &str) -> Option<i32> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = &text[i + pat.len()..];
    let colon = rest.find(':')?;
    let s = rest[colon + 1..].trim_start();
    if s.starts_with('"') {
        return None;
    }
    let end = s
        .find(|c: char| c == ',' || c == '}' || c == '\n' || c.is_whitespace())
        .unwrap_or(s.len());
    s[..end].trim().parse().ok()
}

fn json_string(text: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = &text[i + pat.len()..];
    let colon = rest.find(':')?;
    let s = rest[colon + 1..].trim_start();
    if !s.starts_with('"') {
        return None;
    }
    let s = &s[1..];
    let end = s.find('"')?;
    Some(s[..end].to_string())
}

fn json_blob_section(text: &str, name: &str) -> Option<ManifestBlob> {
    let pat = format!("\"{name}\"");
    let i = text.find(&pat)?;
    let rest = &text[i..];
    let brace = rest.find('{')?;
    let end = rest[brace..].find('}')?;
    let section = &rest[brace..brace + end + 1];
    let sha1 = json_string(section, "sha1")?;
    let bytes = json_i32(section, "bytes").unwrap_or(0) as u64;
    let count = json_u32(section, "count").unwrap_or(0);
    Some(ManifestBlob { sha1, bytes, count })
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{insert_normal_or_max_use, parse_object_txt, parse_transition_txt};
    use ol_binary::{
        push_f32, push_i32, push_str_u16, push_u16, push_u32, write_blob_header, TR_F_LAST_USE_ACTOR,
    };
    use std::collections::HashMap;

    fn sample_db() -> ClientContent {
        let mut db = ClientContent::new();
        db.data_version = 42;
        let mut o = parse_object_txt(
            33,
            "id=33\nGooseberry\nfoodValue=3\npermanent=0\n\
             blocksWalking=1,leftBlockingRadius=2,rightBlockingRadius=1,drawBehindPlayer=0\n\
             mapChance=0.5#biomes_0,3\nheatValue=1\nrValue=0.25\nspeedMult=0.9\n\
             numUses=3,0.5\n\
             sounds=297:0.25,-1:0.0,465:0.5,-1:0.0\n\
             creationSoundInitialOnly=1\ncreationSoundForce=1\n\
             containable=1\nheldOffset=1.5,-2.0\nclothing=n\n\
             spriteID=10\npos=0,0\nrot=0.125\nparent=-1\n\
             invisHolding=0,invisWorn=0,behindSlots=0\n",
        )
        .unwrap();
        o.num_uses = 3;
        o.use_chance = 0.5;
        db.objects.insert(33, o);

        let mut bag = parse_object_txt(
            100,
            "id=100\nBag\nnumSlots=1#timeStretch=1\nslotPos=0,5\nfloor=1\n\
             sounds=-1:0.0,50:0.3,-1:0.0,-1:0.0\n\
             clothing=h\nclothingOffset=1,2\n\
             spriteID=1\npos=0,0\nageRange=0,999\nparent=-1\n",
        )
        .unwrap();
        bag.floor = true;
        db.objects.insert(100, bag);

        let tr = parse_transition_txt(33, 0, "33_0", "0 32\n").unwrap();
        db.transitions.insert((33, 0), tr);
        let tr_norm = parse_transition_txt(10, 20, "10_20", "9 19 0 0 0 0 0 0 0 0 0\n").unwrap();
        db.transitions.insert((10, 20), tr_norm);
        let tr_la = parse_transition_txt(
            10,
            20,
            "10_20_LA",
            "11 21 1.5 1.0 0.5 1 0 2 3 0 1\n",
        )
        .unwrap();
        db.transitions_last_use.insert((10, 20), tr_la);
        db
    }

    #[test]
    fn olc1_roundtrip() {
        let mut db = sample_db();
        assign_multi_use_dummies(&mut db);
        assert_eq!(db.objects.get(&33).unwrap().dummy_ids.len(), 2);
        assert_eq!(db.dummy_parent.len(), 2);

        let bytes = write_olc1(&db);
        assert!(bytes.starts_with(b"OLC1"));
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            OLC1_FORMAT_VERSION
        );
        assert_eq!(peek_olc1_format(&bytes), Some(OLC1_FORMAT_VERSION));
        let mut db2 = ClientContent::new();
        let ver = load_olc1(&bytes, &mut db2).unwrap();
        assert_eq!(ver, 42);
        // 2 real + 2 materialized dummies
        assert_eq!(db2.objects.len(), 4);
        let g = db2.get(33).unwrap();
        assert_eq!(g.food_value, 3);
        assert_eq!(g.num_uses, 3);
        assert!((g.use_chance - 0.5).abs() < 1e-5);
        assert_eq!(g.left_blocking_radius, 2);
        assert_eq!(g.right_blocking_radius, 1);
        assert!(g.draw_behind_player, "wide forces behind");
        assert!((g.map_chance - 0.5).abs() < 1e-5);
        assert_eq!(g.biomes, vec![0, 3]);
        assert!((g.heat_value - 1.0).abs() < 1e-5);
        assert!((g.speed_mult - 0.9).abs() < 1e-5);
        assert!((g.r_value - 0.25).abs() < 1e-5);
        assert_eq!(g.dummy_ids.len(), 2);
        assert!((g.held_offset.0 - 1.5).abs() < 1e-5);
        assert_eq!(g.sprites.len(), 1);
        assert!((g.sprites[0].rot - 0.125).abs() < 1e-5);
        let bag = db2.get(100).unwrap();
        assert!(bag.floor);
        assert_eq!(bag.clothing, 'h');
        assert_eq!(bag.slot_pos.len(), 1);
        assert!((bag.speed_mult - 1.0).abs() < 1e-5);
        // OLC1 v4 sound fields + creation flags
        assert_eq!(g.creation_sound, "297:0.25");
        assert_eq!(g.using_sound, "-1:0.0");
        assert_eq!(g.eating_sound, "465:0.5");
        assert!(g.creation_sound_initial_only);
        assert!(g.creation_sound_force);
        assert_eq!(bag.using_sound, "50:0.3");
        for (di, &did) in g.dummy_ids.iter().enumerate() {
            assert_eq!(db2.dummy_parent.get(&did), Some(&33));
            let dum = db2.get(did).unwrap();
            assert_eq!(dum.dummy_parent, 33);
            // Dummy inherits sim/radii from parent.
            assert_eq!(dum.left_blocking_radius, 2);
            assert_eq!(dum.biomes, vec![0, 3]);
            // C++ creationSoundInitialOnly: only uses_remaining==1 keeps creation.
            let uses_remaining = (di as i32) + 1;
            if uses_remaining == 1 {
                assert_eq!(dum.creation_sound, "297:0.25");
            } else {
                assert!(
                    dum.creation_sound.is_empty(),
                    "dummy uses={uses_remaining} should clear initial-only creation"
                );
            }
        }
    }

    /// P4#25: OLC1 v5 round-trips use vanish/appear + materialize applies skip_drawing.
    #[test]
    fn olc1_sprite_use_vis_on_dummies() {
        let mut db = ClientContent::new();
        db.data_version = 1;
        // 1 base + 3 vanish berries, numUses=3 → 2 dummies (uses 1 and 2).
        let o = parse_object_txt(
            50,
            "id=50\nBerryBush\npermanent=1\n\
             spriteID=1\npos=0,0\nparent=-1\n\
             spriteID=2\npos=0,0\nparent=-1\n\
             spriteID=3\npos=0,0\nparent=-1\n\
             spriteID=4\npos=0,0\nparent=-1\n\
             numUses=3\nuseVanishIndex=1,2,3\nuseAppearIndex=-1\n",
        )
        .unwrap();
        assert_eq!(o.num_uses, 3);
        assert!(o.sprites[1].use_vanish && o.sprites[2].use_vanish && o.sprites[3].use_vanish);
        db.objects.insert(50, o);

        assign_multi_use_dummies(&mut db);
        materialize_dummy_object_records(&mut db);
        let parent = db.get(50).unwrap();
        assert_eq!(parent.dummy_ids.len(), 2);
        // Full parent: no vanish skipped.
        assert!(!parent.sprites[1].skip_drawing);
        assert!(!parent.sprites[2].skip_drawing);
        assert!(!parent.sprites[3].skip_drawing);

        let d_uses1 = db.get(parent.dummy_ids[0]).unwrap();
        let d_uses2 = db.get(parent.dummy_ids[1]).unwrap();
        let vis = |d: &ClientObjectDef| -> usize {
            d.sprites
                .iter()
                .skip(1)
                .filter(|s| !s.skip_drawing)
                .count()
        };
        assert!(vis(d_uses1) >= 1, "last dummy keeps ≥1 berry");
        assert!(
            vis(d_uses2) > vis(d_uses1),
            "higher uses remaining shows more berries"
        );
        assert!(vis(d_uses2) < 3, "first dummy fewer berries than full");

        // Round-trip OLC1 v5 and re-materialize.
        let bytes = write_olc1(&db);
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            OLC1_FORMAT_VERSION
        );
        let mut db2 = ClientContent::new();
        load_olc1(&bytes, &mut db2).unwrap();
        let p2 = db2.get(50).unwrap();
        assert!(p2.sprites[1].use_vanish);
        assert_eq!(p2.dummy_ids.len(), 2);
        let d1 = db2.get(p2.dummy_ids[0]).unwrap();
        let d2 = db2.get(p2.dummy_ids[1]).unwrap();
        assert!(vis(d1) >= 1);
        assert!(vis(d2) > vis(d1));
        // skip_drawing differs between stages (not a static sprite set).
        assert_ne!(
            d1.sprites
                .iter()
                .map(|s| s.skip_drawing)
                .collect::<Vec<_>>(),
            d2.sprites
                .iter()
                .map(|s| s.skip_drawing)
                .collect::<Vec<_>>(),
        );
    }

    /// OLC1-DISTANCES: client bake writes real use/deadly/moves (not defaults).
    // Haxe: ObjectData.writeToFile deadlyDistance/useDistance
    #[test]
    fn olc1_v7_client_bake_distances_roundtrip() {
        let mut db = ClientContent::new();
        db.data_version = 1;
        let bow = parse_object_txt(
            152,
            "id=152\nBow and Arrow\ndeadlyDistance=3\nuseDistance=5\n",
        )
        .unwrap();
        assert_eq!(bow.use_distance, 5);
        assert!((bow.deadly_distance - 3.0).abs() < 1e-5);
        db.objects.insert(152, bow);
        let wolf = parse_object_txt(
            418,
            "id=418\nWolf\ndeadlyDistance=1\nuseDistance=1\nmoves=2\n",
        )
        .unwrap();
        db.objects.insert(418, wolf);

        let bytes = write_olc1(&db);
        // Write path always emits current OLC1 format (v8+); v7 fields still round-trip.
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            OLC1_FORMAT_VERSION
        );
        let mut db2 = ClientContent::new();
        load_olc1(&bytes, &mut db2).unwrap();
        let b = db2.get(152).unwrap();
        assert_eq!(b.use_distance, 5);
        assert!((b.deadly_distance - 3.0).abs() < 1e-5);
        assert_eq!(b.moves, 0);
        let w = db2.get(418).unwrap();
        assert_eq!(w.use_distance, 1);
        assert!((w.deadly_distance - 1.0).abs() < 1e-5);
        assert_eq!(w.moves, 2);
    }

    /// P4#26: C++ variableDummyIDs from `$N` description (letter + numeral labels).
    #[test]
    fn variable_dummy_ids_assign_and_olc1_roundtrip() {
        let mut db = ClientContent::new();
        db.data_version = 1;
        db.root = None;
        // Letter labels (no +varNumeral): Lock and Key $10# removed
        let lock = parse_object_txt(
            1000,
            "id=1000\nLock and Key $10# removed\npermanent=0\ncontainable=1\n",
        )
        .unwrap();
        assert_eq!(
            parse_variable_dollar_count(&lock.description),
            Some((/* dollar after "Lock and Key " */ lock.description.find('$').unwrap(), 10))
        );
        db.objects.insert(1000, lock);

        // Numeral labels: Red Sports Car $30# +varNumeral
        let car = parse_object_txt(
            4659,
            "id=4659\nRed Sports Car $30# +varNumeral\npermanent=1\n",
        )
        .unwrap();
        db.objects.insert(4659, car);

        // Multi-use first (C++ order), then variable — ids continue past multi-use.
        let berry = parse_object_txt(33, "id=33\nBerry\nnumUses=3\n").unwrap();
        db.objects.insert(33, berry);

        assign_multi_use_dummies(&mut db);
        materialize_dummy_object_records(&mut db);
        assign_variable_dummies(&mut db);

        let lock = db.get(1000).unwrap();
        assert_eq!(lock.variable_dummy_ids.len(), 10);
        assert!(
            lock.description.contains("- ?"),
            "parent rewritten: {}",
            lock.description
        );
        assert!(!lock.description.contains("$10"));
        // First dummy: letter A
        let d0 = db.get(lock.variable_dummy_ids[0]).unwrap();
        assert_eq!(d0.variable_dummy_parent, 1000);
        assert!(
            d0.description.contains("- A"),
            "letter label: {}",
            d0.description
        );
        assert!(!d0.is_variable_hidden, "$10 before # → not hidden");
        // 10th dummy: letter J
        let d9 = db.get(lock.variable_dummy_ids[9]).unwrap();
        assert!(
            d9.description.contains("- J"),
            "10th letter: {}",
            d9.description
        );
        assert_eq!(db.base_object_id(lock.variable_dummy_ids[3]), 1000);

        let car = db.get(4659).unwrap();
        assert_eq!(car.variable_dummy_ids.len(), 30);
        let c0 = db.get(car.variable_dummy_ids[0]).unwrap();
        assert!(
            c0.description.contains("- 01"),
            "numeral pad: {}",
            c0.description
        );
        let c29 = db.get(car.variable_dummy_ids[29]).unwrap();
        assert!(
            c29.description.contains("- 30"),
            "last numeral: {}",
            c29.description
        );

        // Multi-use still present; variable ids after multi-use pool.
        let berry = db.get(33).unwrap();
        assert_eq!(berry.dummy_ids.len(), 2);
        let max_use_dummy = *berry.dummy_ids.iter().max().unwrap();
        let min_var = *lock
            .variable_dummy_ids
            .iter()
            .chain(car.variable_dummy_ids.iter())
            .min()
            .unwrap();
        assert!(
            min_var > max_use_dummy,
            "variable ids after multi-use: var={min_var} use={max_use_dummy}"
        );

        // OLC1 v6 round-trip preserves variable_dummy_ids + rematerializes.
        let bytes = write_olc1(&db);
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            OLC1_FORMAT_VERSION
        );
        let mut db2 = ClientContent::new();
        load_olc1(&bytes, &mut db2).unwrap();
        let lock2 = db2.get(1000).unwrap();
        assert_eq!(lock2.variable_dummy_ids, lock.variable_dummy_ids);
        assert!(lock2.description.contains("- ?"));
        let d0b = db2.get(lock2.variable_dummy_ids[0]).unwrap();
        assert_eq!(d0b.variable_dummy_parent, 1000);
        assert!(d0b.description.contains("- A"));
        let car2 = db2.get(4659).unwrap();
        assert_eq!(car2.variable_dummy_ids.len(), 30);
        assert!(db2
            .get(car2.variable_dummy_ids[0])
            .unwrap()
            .description
            .contains("- 01"));
        // Parents only in write count (no dummy records serialized).
        assert_eq!(db2.objects.values().filter(|d| d.variable_dummy_parent == 0 && d.dummy_parent == 0).count(), 3);
    }

    #[test]
    fn olc1_v2_legacy_load_defaults_sim() {
        // Hand-built format-2 blob: use_chance present, no sim trailer.
        let mut out = Vec::new();
        write_blob_header(&mut out, OLC1_MAGIC, OLC1_FORMAT_VERSION_V2, 5, 1, 0);
        push_i32(&mut out, 9);
        push_str_u16(&mut out, "Rock");
        push_str_u16(&mut out, "Rock");
        push_u32(&mut out, OBJ_F_BLOCKS_WALKING);
        push_i32(&mut out, 0);
        push_i32(&mut out, 0);
        push_f32(&mut out, 0.0);
        push_i32(&mut out, 0);
        push_f32(&mut out, 0.0);
        push_f32(&mut out, 0.0);
        out.push(b'n');
        push_f32(&mut out, 0.0);
        push_f32(&mut out, 0.0);
        push_i32(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_i32(&mut out, 0);
        push_f32(&mut out, 0.33); // use_chance
        let mut db = ClientContent::new();
        load_olc1(&out, &mut db).unwrap();
        let o = db.get(9).unwrap();
        assert!(o.blocks_walking);
        assert!((o.use_chance - 0.33).abs() < 1e-5);
        assert_eq!(o.left_blocking_radius, 0);
        assert_eq!(o.right_blocking_radius, 0);
        assert!((o.map_chance - 0.0).abs() < 1e-5);
        assert!(o.biomes.is_empty());
        assert!((o.speed_mult - 1.0).abs() < 1e-5);
        assert!((o.decay_factor - 1.0).abs() < 1e-5);
    }

    #[test]
    fn olt1_roundtrip() {
        let db = sample_db();
        let bytes = write_olt1(&db);
        assert!(bytes.starts_with(b"OLT1"));
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 2);
        // sample_db did not expand categories → flag clear
        assert_eq!(peek_blob_flags(&bytes) & OLT1_F_CATEGORY_EXPANDED, 0);
        assert!(olt1_lacks_category_expanded(&bytes));
        let mut db2 = ClientContent::new();
        load_olt1(&bytes, &mut db2).unwrap();
        assert!(!db2.transitions_category_expanded);
        assert_eq!(db2.transitions.len(), 2);
        assert_eq!(db2.transitions_last_use.len(), 1);
        let tr = db2.transitions.get(&(33, 0)).unwrap();
        assert_eq!(tr.new_target_id, 32);
        let norm = db2.transitions.get(&(10, 20)).unwrap();
        assert_eq!(norm.new_actor_id, 9);
        let la = db2.transitions_last_use.get(&(10, 20)).unwrap();
        assert!(la.last_use_actor);
        assert_eq!(la.new_actor_id, 11);
        assert!(la.reverse_use_actor);
        assert!(la.no_use_target);
        assert_eq!(la.move_dist, 2);
        assert!(db2.find_transition(10, 20).is_some());
        assert!(db2.find_transition_last_use(10, 20).is_some());
    }

    /// P4#29: max-use + switch flags survive OLT1 write/load.
    #[test]
    fn olt1_max_use_and_switch_roundtrip() {
        let mut db = ClientContent::new();
        db.data_version = 5;
        // Primary: target remains
        db.transitions.insert(
            (33, 1096),
            ClientTransition {
                actor_id: 33,
                target_id: 1096,
                new_actor_id: 0,
                new_target_id: 1096,
                reverse_use_target: true,
                ..Default::default()
            },
        );
        // Max-use complete
        db.transitions_max_use.insert(
            (33, 1096),
            ClientTransition {
                actor_id: 33,
                target_id: 1096,
                new_actor_id: 0,
                new_target_id: 3963,
                ..Default::default()
            },
        );
        // Switch patch key
        db.transitions.insert(
            (252, 3371),
            ClientTransition {
                actor_id: 252,
                target_id: 3371,
                new_actor_id: 252,
                new_target_id: 3371,
                switch_number_of_uses: true,
                ..Default::default()
            },
        );
        let bytes = write_olt1(&db);
        assert!(bytes.starts_with(b"OLT1"));
        let mut db2 = ClientContent::new();
        load_olt1(&bytes, &mut db2).unwrap();
        assert_eq!(db2.transitions.len(), 2);
        assert_eq!(db2.transitions_max_use.len(), 1);
        assert_eq!(
            db2.find_transition(33, 1096).unwrap().new_target_id,
            1096
        );
        assert!(db2.find_transition(33, 1096).unwrap().reverse_use_target);
        assert_eq!(
            db2.find_transition_max_use(33, 1096).unwrap().new_target_id,
            3963
        );
        assert!(
            db2.find_transition(252, 3371)
                .unwrap()
                .switch_number_of_uses
        );
    }

    /// P4#29: bake from text with dual remains/complete files + switch patches.
    #[test]
    fn olt1_bake_max_use_from_text() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olt1_maxuse_{}",
            std::process::id()
        ));
        let src = tmp.join("src");
        let cache = src.join("cache");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(src.join("objects")).unwrap();
        fs::create_dir_all(src.join("transitions")).unwrap();
        fs::create_dir_all(src.join("animations")).unwrap();
        fs::write(src.join("dataVersionNumber.txt"), "12\n").unwrap();
        fs::write(src.join("objects").join("nextObjectNumber.txt"), "5000\n").unwrap();
        fs::write(
            src.join("objects").join("33.txt"),
            "id=33\nStone\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(
            src.join("objects").join("1096.txt"),
            "id=1096\nWellSite\nnumUses=5\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(
            src.join("objects").join("3963.txt"),
            "id=3963\nCompleteWell\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(
            src.join("objects").join("252.txt"),
            "id=252\nDough\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(
            src.join("objects").join("3371.txt"),
            "id=3371\nTable\nfoodValue=0\n",
        )
        .unwrap();
        // Two files same actor_target stem cannot coexist — Haxe uses same key from
        // category expansion or distinct file bodies loaded under same (actor,target).
        // Simulate via one remains file; inject complete via second path by writing
        // remains then complete under different intermediate names is not possible.
        // Use remains as primary file; complete goes through insert after load in unit
        // above. Here: single remains + dough/table for switch flag bake.
        fs::write(
            src.join("transitions").join("33_1096.txt"),
            "0 1096 0 0 0 0 1 0 0 0 0\n",
        )
        .unwrap();
        fs::write(
            src.join("transitions").join("252_3371.txt"),
            "252 3371\n",
        )
        .unwrap();

        // Programmatic max-use pair after text load: re-open and inject complete.
        let mut db = ClientContent::load_from_dir(&src).unwrap();
        assert!(
            db.find_transition(252, 3371)
                .unwrap()
                .switch_number_of_uses,
            "switch patch after load_from_dir"
        );
        assert!(insert_normal_or_max_use(
            &mut db,
            ClientTransition {
                actor_id: 33,
                target_id: 1096,
                new_actor_id: 0,
                new_target_id: 3963,
                ..Default::default()
            },
        ));
        assert_eq!(
            db.find_transition_max_use(33, 1096).unwrap().new_target_id,
            3963
        );

        fs::create_dir_all(&cache).unwrap();
        // Bake via write path on mutated db (same as bake_content write_olt1).
        assign_multi_use_dummies(&mut db);
        let olt1 = write_olt1(&db);
        fs::write(cache.join("olt1_transitions.bin"), &olt1).unwrap();
        let mut db2 = ClientContent::new();
        load_olt1(&olt1, &mut db2).unwrap();
        assert_eq!(
            db2.find_transition_max_use(33, 1096).unwrap().new_target_id,
            3963
        );
        assert!(
            db2.find_transition(252, 3371)
                .unwrap()
                .switch_number_of_uses
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    /// P4#28: bake writes OLT1 with CATEGORY_EXPANDED; cache load skips re-expand
    /// but still has concrete member transitions + CategoryBank for pick.
    #[test]
    fn olt1_bake_expanded_skips_reexpand_on_load() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olt1_exp_{}",
            std::process::id()
        ));
        let src = tmp.join("src");
        // Real layout: cache under content root so load_from_cache finds categories/.
        let cache = src.join("cache");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(src.join("objects")).unwrap();
        fs::create_dir_all(src.join("transitions")).unwrap();
        fs::create_dir_all(src.join("categories")).unwrap();
        fs::create_dir_all(src.join("animations")).unwrap();
        fs::write(src.join("dataVersionNumber.txt"), "11\n").unwrap();
        fs::write(src.join("objects").join("nextObjectNumber.txt"), "900\n").unwrap();
        // Parent category 722 → members 34, 502 (lite expand of 722+36).
        fs::write(
            src.join("categories").join("722.txt"),
            "parentID=722\nnumObjects=2\n34\n502\n",
        )
        .unwrap();
        fs::write(
            src.join("objects").join("34.txt"),
            "id=34\nSharpStone\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(
            src.join("objects").join("36.txt"),
            "id=36\nSeedingWildCarrot\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(
            src.join("objects").join("39.txt"),
            "id=39\nDugCarrot\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(
            src.join("objects").join("722.txt"),
            "id=722\n@ShallowDigger\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(
            src.join("transitions").join("722_36.txt"),
            "722 39\n",
        )
        .unwrap();

        let res = bake_content(&src, &cache).unwrap();
        assert!(res.transition_count >= 3, "base + 2 member expands");
        let olt1 = fs::read(cache.join("olt1_transitions.bin")).unwrap();
        assert!(
            !olt1_lacks_category_expanded(&olt1),
            "bake must set OLT1_F_CATEGORY_EXPANDED"
        );
        assert_eq!(
            peek_blob_flags(&olt1) & OLT1_F_CATEGORY_EXPANDED,
            OLT1_F_CATEGORY_EXPANDED
        );

        // Direct cache load: expanded concrete keys present without re-expand path.
        let db = load_from_cache(&cache, Some(11)).unwrap();
        assert!(db.transitions_category_expanded);
        assert!(
            db.find_transition(34, 36).is_some(),
            "member 34+36 must be baked into OLT1"
        );
        assert!(db.find_transition(502, 36).is_some());
        assert!(db.find_transition(722, 36).is_some());
        // Category bank still loaded for reverse lookups / prob-set.
        assert!(db.categories.get_category(722).is_some());
        assert_eq!(db.categories.get_category_for_object(34, 0), 722);

        // Legacy OLT1 (flag cleared): re-expand from text restores members.
        let mut legacy = olt1.clone();
        legacy[16..20].copy_from_slice(&0u32.to_le_bytes());
        assert!(olt1_lacks_category_expanded(&legacy));
        let mut db_legacy = ClientContent::new();
        load_olt1(&legacy, &mut db_legacy).unwrap();
        assert!(!db_legacy.transitions_category_expanded);
        // Strip expanded member keys to prove re-expand restores them.
        db_legacy.transitions.retain(|k, _| *k == (722, 36));
        let added = db_legacy.maybe_load_categories_from_root(&src);
        assert!(added >= 2);
        assert!(db_legacy.find_transition(34, 36).is_some());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn olt1_v1_legacy_load() {
        let mut out = Vec::new();
        write_blob_header(&mut out, OLT1_MAGIC, 1, 7, 1, 0);
        push_i32(&mut out, 1);
        push_i32(&mut out, 2);
        push_i32(&mut out, 3);
        push_i32(&mut out, 4);
        out.push(TR_F_LAST_USE_ACTOR);
        push_f32(&mut out, 0.0);
        let mut db = ClientContent::new();
        load_olt1(&out, &mut db).unwrap();
        assert!(db.transitions.is_empty());
        let la = db.transitions_last_use.get(&(1, 2)).unwrap();
        assert_eq!(la.new_actor_id, 3);
        assert!(la.last_use_actor);
    }

    #[test]
    fn double_bake_deterministic_sha1() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olc1_det_{}",
            std::process::id()
        ));
        let src = tmp.join("src");
        let c1 = tmp.join("c1");
        let c2 = tmp.join("c2");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(src.join("objects")).unwrap();
        fs::create_dir_all(src.join("transitions")).unwrap();
        fs::create_dir_all(src.join("animations")).unwrap();
        fs::write(src.join("dataVersionNumber.txt"), "3\n").unwrap();
        fs::write(src.join("objects").join("nextObjectNumber.txt"), "500\n").unwrap();
        fs::write(
            src.join("objects").join("7.txt"),
            "id=7\nStick\nnumUses=2,0.1\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(src.join("transitions").join("7_0.txt"), "0 8\n").unwrap();
        fs::write(src.join("transitions").join("7_0_LA.txt"), "0 9\n").unwrap();
        bake_content(&src, &c1).unwrap();
        bake_content(&src, &c2).unwrap();
        assert_eq!(
            fs::read(c1.join("olc1_objects.bin")).unwrap(),
            fs::read(c2.join("olc1_objects.bin")).unwrap()
        );
        assert_eq!(
            fs::read(c1.join("olt1_transitions.bin")).unwrap(),
            fs::read(c2.join("olt1_transitions.bin")).unwrap()
        );
        let db = load_from_cache(&c1, Some(3)).unwrap();
        assert_eq!(db.transitions.len(), 1);
        assert_eq!(db.transitions_last_use.len(), 1);
        assert!((db.get(7).unwrap().use_chance - 0.1).abs() < 1e-5);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn bake_and_load_cache_dir() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olc1_test_{}",
            std::process::id()
        ));
        let src = tmp.join("src");
        let cache = tmp.join("cache");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(src.join("objects")).unwrap();
        fs::create_dir_all(src.join("transitions")).unwrap();
        fs::create_dir_all(src.join("animations")).unwrap();
        fs::write(src.join("dataVersionNumber.txt"), "99\n").unwrap();
        fs::write(
            src.join("objects").join("nextObjectNumber.txt"),
            "1000\n",
        )
        .unwrap();
        fs::write(
            src.join("objects").join("55.txt"),
            "id=55\nBerry\nfoodValue=2\nnumUses=4\npermanent=0\nblocksWalking=0\n",
        )
        .unwrap();
        fs::write(
            src.join("transitions").join("55_0.txt"),
            "0 56\nautoDecaySeconds=0\n",
        )
        .unwrap();
        fs::write(
            src.join("animations").join("55_0.txt"),
            "id=55\ntype=0,randStartPhase=0\nforceZeroStart=0\nnumSounds=0\n\
             numSprites=1\nnumSlots=0\n\
             offset=(0.000000,0.000000)\nstartPause=0.000000\n\
             animParam=1.000000 5.000000 0.000000 0.000000 0.000000 0.000000 \
             (0.000000,0.000000) 0.000000 0.000000 0.000000 0.000000 0.000000 \
             1.000000 0.000000 0.000000 0.000000 0.000000 1.000000 0.000000\n",
        )
        .unwrap();

        let res = bake_content(&src, &cache).unwrap();
        assert_eq!(res.data_version, 99);
        assert_eq!(res.object_count, 1);
        assert_eq!(res.transition_count, 1);
        assert_eq!(res.anim_count, 1);
        assert_eq!(res.dummy_count, 3); // numUses=4 → 3 dummies
        assert!(cache.join("manifest.json").exists());
        assert!(cache.join("olc1_objects.bin").exists());
        assert!(cache.join("ola1_anims.bin").exists());
        assert!(cache.join("olg1_ground_index.bin").exists());
        assert!(cache.join("olo1_overlays.bin").exists());
        assert!(cache.join("olsn_sounds.bin").exists());
        assert!(cache.join("ols1_sprites.bin").exists());
        assert!(res.ola1_bytes > 24);
        assert!(res.olg1_bytes >= 32);
        assert!(res.olo1_bytes >= 24);
        assert!(res.olsn_bytes >= 24);
        assert!(res.ols1_bytes >= 24);
        // OLG1/OLO1/OLSN/OLS1 may be empty of assets in tiny fixture
        let man = parse_manifest(&fs::read_to_string(cache.join("manifest.json")).unwrap()).unwrap();
        assert!(man.olg1.is_some());
        assert!(man.olo1.is_some());
        assert!(man.olsn.is_some());
        assert!(man.ols1.is_some());

        let db = load_from_cache(&cache, Some(99)).unwrap();
        assert_eq!(db.data_version, 99);
        let b = db.get(55).unwrap();
        assert_eq!(b.food_value, 2);
        assert_eq!(b.dummy_ids.len(), 3);
        assert_eq!(b.dummy_ids[0], 1000);
        assert_eq!(db.transitions.get(&(55, 0)).unwrap().new_target_id, 56);

        // AnimBank loads OLA1 from same cache
        let mut anims = crate::anim_bank::AnimBank::load_prefer_cache(&src);
        // cache is under tmp/cache not src — copy path: bake wrote to `cache`, not src/cache
        let mut anims_cache = crate::anim_bank::AnimBank::from_ola1(
            &fs::read(cache.join("ola1_anims.bin")).unwrap(),
            &src,
        )
        .unwrap();
        let a = anims_cache.get(55, 0).unwrap();
        assert!((a.sprite_params[0].x_amp - 5.0).abs() < 1e-5);
        let _ = &mut anims;

        // version mismatch rejects
        assert!(load_from_cache(&cache, Some(1)).is_err());

        let cache2 = src.join("cache");
        bake_content(&src, &cache2).unwrap();
        let db2 = load_prefer_cache(&src).unwrap();
        assert_eq!(db2.get(55).unwrap().food_value, 2);
        let mut anims2 = crate::anim_bank::AnimBank::load_prefer_cache(&src);
        assert!(anims2.get(55, 0).is_some());
        // P4#31: OLS1 prefer_cache from same cache tree
        let sprites2 = crate::sprite_bank::SpriteBank::load_prefer_cache(&src);
        assert!(sprites2.index_loaded || sprites2.meta_count() == 0);
        assert!(src.join("cache").join("ols1_sprites.bin").exists());

        let _ = fs::remove_dir_all(&tmp);
    }

    /// P4#35: multi-use dummy id sequence matches C++/server algorithm (oracle).
    ///
    /// Server `assign_multi_use_dummies`: sort multi-use parent ids, allocate
    /// `numUses-1` sequential free ids from `nextObjectNumber` (or max+1).
    #[test]
    fn golden_dummy_id_sequence_synthetic() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_golden_dummy_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("objects")).unwrap();
        fs::create_dir_all(tmp.join("transitions")).unwrap();
        fs::write(tmp.join("dataVersionNumber.txt"), "1\n").unwrap();
        // Free id pool starts at 1000.
        fs::write(tmp.join("objects").join("nextObjectNumber.txt"), "1000\n").unwrap();
        // Parent ids out of order on disk; allocation must be sorted by id.
        fs::write(
            tmp.join("objects").join("30.txt"),
            "id=30\nB\nnumUses=2\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("objects").join("10.txt"),
            "id=10\nA\nnumUses=3\nfoodValue=0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("objects").join("20.txt"),
            "id=20\nC\nnumUses=4\nfoodValue=0\n",
        )
        .unwrap();
        // Single-use must not consume free ids.
        fs::write(
            tmp.join("objects").join("5.txt"),
            "id=5\nPlain\nnumUses=1\nfoodValue=0\n",
        )
        .unwrap();

        let mut db = ClientContent::load_from_dir(&tmp).unwrap();
        // load_from_dir already assigns dummies; re-run is idempotent for multi-use.
        assign_multi_use_dummies(&mut db);

        // Sorted parents: 10 → 1000,1001; 20 → 1002,1003,1004; 30 → 1005
        assert_eq!(db.get(10).unwrap().dummy_ids, vec![1000, 1001]);
        assert_eq!(db.get(20).unwrap().dummy_ids, vec![1002, 1003, 1004]);
        assert_eq!(db.get(30).unwrap().dummy_ids, vec![1005]);
        assert!(db.get(5).unwrap().dummy_ids.is_empty());
        assert_eq!(db.dummy_parent.get(&1000), Some(&10));
        assert_eq!(db.dummy_parent.get(&1005), Some(&30));
        // Uniqueness: 2+3+1 = 6 dummies
        assert_eq!(db.dummy_parent.len(), 6);

        // OLC1 bake preserves lists (materialize path).
        let bytes = write_olc1(&db);
        let mut db2 = ClientContent::new();
        load_olc1(&bytes, &mut db2).unwrap();
        assert_eq!(db2.get(10).unwrap().dummy_ids, vec![1000, 1001]);
        assert_eq!(db2.get(20).unwrap().dummy_ids, vec![1002, 1003, 1004]);
        assert_eq!(db2.get(30).unwrap().dummy_ids, vec![1005]);

        let _ = fs::remove_dir_all(&tmp);
    }

    /// P4#35: live OneLifeData7 multi-use sequence matches server-style oracle.
    ///
    /// Skips cleanly when content tree is absent (CI without data).
    #[test]
    fn golden_dummy_id_sequence_onelife_data7() {
        let roots = [
            PathBuf::from(r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7"),
            PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7"),
        ];
        let root = roots.into_iter().find(|p| p.join("objects").is_dir());
        let Some(root) = root else {
            return;
        };

        let next: i32 = fs::read_to_string(root.join("objects").join("nextObjectNumber.txt"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);

        // Text load then multi-use assign (same path as bake_content / server boot).
        let mut db = ClientContent::load_from_dir(&root).expect("load OneLifeData7");
        assign_multi_use_dummies(&mut db);

        // Oracle for multi-use only (server algorithm; variable dummies are client-extra).
        let mut multi: Vec<(i32, i32)> = db
            .objects
            .iter()
            .filter(|(_, d)| d.num_uses >= 2 && d.dummy_parent == 0 && d.variable_dummy_parent == 0)
            .map(|(id, d)| (*id, d.num_uses))
            .collect();
        multi.sort_by_key(|(id, _)| *id);
        let max_real = db
            .objects
            .iter()
            .filter(|(_, d)| d.dummy_parent == 0 && d.variable_dummy_parent == 0)
            .map(|(id, _)| *id)
            .max()
            .unwrap_or(0);
        let mut free = if next > max_real { next } else { max_real + 1 };
        let mut expected: HashMap<i32, Vec<i32>> = HashMap::new();
        for (id, n) in &multi {
            let mut dummies = Vec::new();
            for _ in 0..(n - 1) {
                dummies.push(free);
                free += 1;
            }
            expected.insert(*id, dummies);
        }

        // Client multi-use lists must match oracle (variable dummies allocate after).
        for (id, want) in &expected {
            let got = &db.get(*id).expect("parent").dummy_ids;
            assert_eq!(
                got, want,
                "multi-use dummy_ids for object {id}: client={got:?} oracle={want:?}"
            );
        }

        // Sample a few well-known multi-use objects when present (berries / bowls).
        for sample in [30i32, 31, 33, 55, 1261] {
            if let Some(def) = db.get(sample) {
                if def.num_uses >= 2 {
                    assert_eq!(
                        def.dummy_ids.len(),
                        (def.num_uses - 1) as usize,
                        "sample id={sample}"
                    );
                    assert_eq!(
                        expected.get(&sample).map(|v| v.as_slice()),
                        Some(def.dummy_ids.as_slice())
                    );
                }
            }
        }

        // All multi-use dummy ids unique and mapped.
        let mut seen = std::collections::HashSet::new();
        for (id, list) in &expected {
            for did in list {
                assert!(seen.insert(*did), "duplicate multi-use dummy {did}");
                assert_eq!(db.dummy_parent.get(did), Some(id));
            }
        }
    }

    #[test]
    fn manifest_parse() {
        let t = r#"{
  "format": 1,
  "data_version": 437,
  "created_utc": "unix:1",
  "source": "OneLifeData7",
  "blobs": {
    "olc1_objects.bin": { "sha1": "abc", "bytes": 10, "count": 2 },
    "olt1_transitions.bin": { "sha1": "def", "bytes": 20, "count": 3 },
    "ola1_anims.bin": { "sha1": "ghi", "bytes": 30, "count": 4 },
    "olg1_ground_index.bin": { "sha1": "jkl", "bytes": 40, "count": 5 },
    "olo1_overlays.bin": { "sha1": "mno", "bytes": 50, "count": 6 },
    "ols1_sprites.bin": { "sha1": "pqr", "bytes": 60, "count": 7 }
  }
}"#;
        let m = parse_manifest(t).unwrap();
        assert_eq!(m.format, 1);
        assert_eq!(m.data_version, 437);
        assert_eq!(m.olc1.as_ref().unwrap().sha1, "abc");
        assert_eq!(m.olt1.as_ref().unwrap().count, 3);
        assert_eq!(m.ola1.as_ref().unwrap().count, 4);
        assert_eq!(m.olg1.as_ref().unwrap().count, 5);
        assert_eq!(m.olg1.as_ref().unwrap().sha1, "jkl");
        assert_eq!(m.olo1.as_ref().unwrap().count, 6);
        assert_eq!(m.olo1.as_ref().unwrap().sha1, "mno");
        assert_eq!(m.ols1.as_ref().unwrap().count, 7);
        assert_eq!(m.ols1.as_ref().unwrap().sha1, "pqr");
    }

    #[test]
    fn bad_magic_rejected() {
        let mut db = ClientContent::new();
        assert!(load_olc1(b"XXXX................", &mut db).is_err());
        assert!(load_olt1(b"YYYY................", &mut db).is_err());
    }
}
