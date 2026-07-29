//! OLC1 / OLT1 binary cache load via shared [`ol_binary`] crate.
//!
//! See client `docs/port/CONTENT_BINARY.md`. Text remains source of truth for
//! authoring; blobs are a fast-start cache. Server loads a subset of object
//! fields from OLC1 (no sprites) and full transition fields from OLT1 v1/v2,
//! including **max-use** (bit6) and **switch_number_of_uses** (bit7) via the
//! shared parse path (P4#30).
//!
//! OLC1 format 1..=7 accepted (sprites / sound / use-vis / variableDummy trailers
//! consumed; format ≥ 7 maps use_distance/deadly_distance/moves into ObjectDef).
//! Format ≥ 3 fills map_chance/biomes/heat/speed/decay so [`load_prefer_cache`]
//! can stay on the binary path.
//!
//! // Haxe: ObjectBake offline dummies + Resource packed content
//! // C++: folderCache / regenerateCaches (different on-disk format; same idea)
//! // Haxe: ObjectData.useDistance / deadlyDistance / moves (OLC1-DISTANCES)

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use ol_binary::{
    parse_olc1, parse_olt1, Olt1Table, OBJ_F_BLOCKS_WALKING, OBJ_F_CONTAINABLE, OBJ_F_FLOOR,
    OBJ_F_PERMANENT, OLC1_FORMAT_VERSION, OLT1_FORMAT_VERSION,
};
use sha1::{Digest, Sha1};
use tracing::{info, warn};

use crate::{
    apply_animal_moves_from_transitions, apply_default_animal_deadly_distance_patches,
    apply_default_combat_damage_patches, apply_default_contain_size_patches,
    apply_default_decay_object_patches, apply_default_ai_should_ignore_patches,
    apply_default_alternative_outcome_patches, apply_default_horse_transition_patches,
    apply_default_second_time_outcomes, apply_default_switch_number_of_uses_patches,
    apply_default_use_chance_patches, apply_default_weapon_range_patches,
    change_tool_transitions, expand_category_transitions, load_categories_into, ContentDb,
    ContentError, ObjectDef, Transition,
};

/// Shared magics / format caps (server accepts full shared write path v7/v2).
pub use ol_binary::{OLC1_MAGIC, OLT1_MAGIC};
pub const OLC1_FORMAT_MAX: u32 = OLC1_FORMAT_VERSION;
pub const OLT1_FORMAT_MAX: u32 = OLT1_FORMAT_VERSION;

// ── binary helpers ───────────────────────────────────────────────────────────

fn sha1_hex(data: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(data);
    let dig = h.finalize();
    let mut s = String::with_capacity(40);
    for b in dig {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

// ── OLC1 ─────────────────────────────────────────────────────────────────────

/// Load OLC1 object records into `db` (replaces objects + dummy_parent).
/// Sprites/slots/sounds are skipped (server ObjectDef has no draw fields).
/// Format ≥ 3 fills map_chance/biomes/heat/speed/decay; rebuilds biome_spawn.
/// Format ≥ 7 fills use_distance / deadly_distance / moves (IS-CLOSE / animals).
/// Format 1..=7 accepted via shared [`ol_binary::parse_olc1`].
pub fn load_olc1(data: &[u8], db: &mut ContentDb) -> Result<u32, String> {
    let blob = parse_olc1(data)?;
    db.objects.clear();
    db.dummy_parent.clear();
    db.biome_spawn.clear();
    db.data_version = blob.header.data_version as i32;
    for rec in blob.records {
        let def = olc1_record_to_object(rec);
        for &did in &def.dummy_ids {
            db.dummy_parent.insert(did, def.id);
        }
        if def.map_chance > 0.0 && !def.biomes.is_empty() {
            for &b in &def.biomes {
                let table = db.biome_spawn.entry(b).or_default();
                table.total_chance += def.map_chance;
                table.entries.push((def.id, def.map_chance));
            }
        }
        db.objects.insert(def.id, def);
    }
    Ok(blob.header.data_version)
}

fn olc1_record_to_object(rec: ol_binary::Olc1Record) -> ObjectDef {
    let mut def = ObjectDef::empty(rec.id);
    def.name = rec.name;
    def.description = rec.description;
    def.permanent = rec.flags & OBJ_F_PERMANENT != 0;
    def.blocks_walking = rec.flags & OBJ_F_BLOCKS_WALKING != 0;
    def.containable = rec.flags & OBJ_F_CONTAINABLE != 0;
    def.floor = rec.flags & OBJ_F_FLOOR != 0;
    def.food_value = rec.food_value;
    def.num_uses = rec.num_uses;
    def.num_slots = rec.num_slots;
    def.dummy_ids = rec.dummy_ids;
    def.use_chance = rec.use_chance;
    def.map_chance = rec.map_chance;
    def.biomes = rec.biomes;
    def.heat_value = rec.heat_value;
    def.speed_mult = rec.speed_mult;
    def.r_value = rec.r_value;
    def.decay_factor = rec.decay_factor;
    def.decays_to_obj = rec.decays_to_obj;
    def.winter_decay_factor = rec.winter_decay_factor;
    def.spring_regrow_factor = rec.spring_regrow_factor;
    // OLC1-DISTANCES / binary_use_dist — format ≥ 7; older formats keep ObjectDef defaults.
    // Haxe: ObjectData.deadlyDistance / useDistance / moves
    def.deadly_distance = rec.deadly_distance;
    def.use_distance = rec.use_distance;
    def.moves = rec.moves;
    // CLOTHING-CONTAIN-SIZE / contain_slot_size — format ≥ 8; older keep defaults 0 / 1.
    // Haxe: ObjectData.containSize / slotSize
    def.contain_size = rec.contain_size;
    def.slot_size = rec.slot_size;
    def.clothing = {
        let c = rec.clothing as char;
        if c == '\0' {
            "n".into()
        } else {
            c.to_string()
        }
    };
    def
}

// ── OLT1 ─────────────────────────────────────────────────────────────────────

/// Load OLT1 into transitions / transitions_last_use / transitions_max_use / auto_decays.
///
/// Shared path restores **bit6** → `transitions_max_use` and **bit7** →
/// `switch_number_of_uses` (P4#29 residual closed by P4#30).
pub fn load_olt1(data: &[u8], db: &mut ContentDb) -> Result<u32, String> {
    let blob = parse_olt1(data)?;
    db.transitions.clear();
    db.transitions_last_use.clear();
    db.transitions_max_use.clear();
    db.auto_decays.clear();
    if db.data_version == 0 {
        db.data_version = blob.header.data_version as i32;
    }
    let mut loaded = 0usize;
    let mut loaded_last = 0usize;
    for rec in blob.records {
        let tr = olt1_record_to_transition(&rec);
        // Auto-decay / animal move: actor -1
        if tr.actor_id < 0 && tr.auto_decay_seconds != 0.0 {
            db.auto_decays.insert(tr.target_id, tr.clone());
        }
        let key = (rec.actor_id, rec.target_id);
        match rec.table() {
            Olt1Table::LastUse => {
                db.transitions_last_use.insert(key, tr);
                loaded_last += 1;
            }
            Olt1Table::MaxUse => {
                db.transitions_max_use.insert(key, tr);
            }
            Olt1Table::Normal => {
                db.transitions.insert(key, tr);
                loaded += 1;
            }
        }
    }
    db.transition_count = loaded;
    db.last_use_transition_count = loaded_last;
    Ok(blob.header.data_version)
}

fn olt1_record_to_transition(rec: &ol_binary::Olt1Record) -> Transition {
    Transition {
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
        target_number_of_uses: -1,
        is_pickup_or_drop: false,
    }
}

// ── manifest + cache load ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ManifestBlob {
    sha1: String,
}

#[derive(Debug, Clone)]
struct ContentManifest {
    data_version: i32,
    olc1: Option<ManifestBlob>,
    olt1: Option<ManifestBlob>,
}

fn parse_manifest(text: &str) -> Result<ContentManifest, String> {
    let data_version = json_i32(text, "data_version").unwrap_or(0);
    let olc1 = json_blob_sha1(text, "olc1_objects.bin");
    let olt1 = json_blob_sha1(text, "olt1_transitions.bin");
    Ok(ContentManifest {
        data_version,
        olc1,
        olt1,
    })
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

fn json_blob_sha1(text: &str, name: &str) -> Option<ManifestBlob> {
    let pat = format!("\"{name}\"");
    let i = text.find(&pat)?;
    let rest = &text[i..];
    let brace = rest.find('{')?;
    let end = rest[brace..].find('}')?;
    let section = &rest[brace..brace + end + 1];
    let sha1 = json_string(section, "sha1")?;
    Some(ManifestBlob { sha1 })
}

/// Default cache subdirectory under a content root.
pub fn cache_dir_for(root: &Path) -> PathBuf {
    root.join("cache")
}

/// Load from a cache directory written by client `bake_content`.
///
/// Verifies optional `expected_data_version` and manifest sha1 when present.
/// Does **not** load categories or apply ServerSettings patches — call
/// [`finish_cache_boot`] or use [`load_prefer_cache`].
pub fn load_from_cache(
    cache_dir: impl AsRef<Path>,
    expected_data_version: Option<i32>,
) -> Result<ContentDb, ContentError> {
    let cache_dir = cache_dir.as_ref();
    let man_path = cache_dir.join("manifest.json");
    let manifest = if man_path.exists() {
        Some(
            parse_manifest(
                &fs::read_to_string(&man_path).map_err(|e| ContentError::Binary(e.to_string()))?,
            )
            .map_err(ContentError::Binary)?,
        )
    } else {
        None
    };

    if let (Some(m), Some(exp)) = (manifest.as_ref(), expected_data_version) {
        if m.data_version != exp {
            return Err(ContentError::Binary(format!(
                "cache data_version {} != tree {}",
                m.data_version, exp
            )));
        }
    }

    let olc1_path = cache_dir.join("olc1_objects.bin");
    if !olc1_path.exists() {
        return Err(ContentError::Binary(format!(
            "missing {}",
            olc1_path.display()
        )));
    }
    let olc1 = fs::read(&olc1_path).map_err(|e| ContentError::Binary(e.to_string()))?;
    if let Some(m) = manifest.as_ref().and_then(|m| m.olc1.as_ref()) {
        let h = sha1_hex(&olc1);
        if h != m.sha1 {
            return Err(ContentError::Binary(format!(
                "olc1 sha1 mismatch (want {}, got {})",
                m.sha1, h
            )));
        }
    }

    let t0 = Instant::now();
    let mut db = ContentDb::default();
    load_olc1(&olc1, &mut db).map_err(ContentError::Binary)?;
    db.load_objects_ms = t0.elapsed().as_millis() as u64;

    let olt1_path = cache_dir.join("olt1_transitions.bin");
    if olt1_path.exists() {
        let t1 = Instant::now();
        let olt1 = fs::read(&olt1_path).map_err(|e| ContentError::Binary(e.to_string()))?;
        if let Some(m) = manifest.as_ref().and_then(|m| m.olt1.as_ref()) {
            let h = sha1_hex(&olt1);
            if h != m.sha1 {
                return Err(ContentError::Binary(format!(
                    "olt1 sha1 mismatch (want {}, got {})",
                    m.sha1, h
                )));
            }
        }
        load_olt1(&olt1, &mut db).map_err(ContentError::Binary)?;
        db.load_transitions_ms = t1.elapsed().as_millis() as u64;
    }
    db.load_total_ms = t0.elapsed().as_millis() as u64;
    Ok(db)
}

/// After binary load: categories from text + default ServerSettings patches.
pub fn finish_cache_boot(db: &mut ContentDb, root: &Path) {
    load_categories_into(db, &root.join("categories"));
    expand_category_transitions(db);
    // Haxe TransitionImporter.changeToolTransitions (after category expand).
    change_tool_transitions(db);
    apply_default_second_time_outcomes(db);
    apply_default_decay_object_patches(db);
    // CLOTHING-CONTAIN-SIZE: ServerSettings.PatchObjectData containSize / containable.
    apply_default_contain_size_patches(db);
    apply_default_use_chance_patches(db);
    apply_default_switch_number_of_uses_patches(db);
    apply_default_horse_transition_patches(db);
    // TH-ALT-OUTCOME: alternativeTransitionOutcome + fortification tables
    apply_default_alternative_outcome_patches(db);
    // C-SS-AI-IGNORE: ServerSettings.PatchTransitions aiShouldIgnore
    apply_default_ai_should_ignore_patches(db);
    apply_default_weapon_range_patches(db);
    apply_default_animal_deadly_distance_patches(db);
    apply_default_combat_damage_patches(db);
    apply_animal_moves_from_transitions(db);
}

fn read_data_version(root: &Path) -> Option<i32> {
    fs::read_to_string(root.join("dataVersionNumber.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Prefer `root/cache` OLC1/OLT1 when valid **and** objects carry enough sim
/// fields for boot; else full text [`crate::load_content`].
///
/// OLC1 format ≥ 3 stores `map_chance`/biomes/heat/speed/decay. When every
/// object has `map_chance == 0` (legacy format 1/2 cache), fall back to text so
/// world gen still works. Direct [`load_from_cache`] remains available for tools.
pub fn load_prefer_cache(root: impl AsRef<Path>) -> Result<ContentDb, ContentError> {
    let root = root.as_ref();
    let tree_ver = read_data_version(root);
    let cache = cache_dir_for(root);
    if cache.join("olc1_objects.bin").exists() {
        match load_from_cache(&cache, tree_ver) {
            Ok(mut db) => {
                let has_spawn_meta = db.objects.values().any(|o| o.map_chance > 0.0);
                if !has_spawn_meta && root.join("objects").is_dir() {
                    warn!(
                        path = %cache.display(),
                        "OLC1 lacks map_chance/biomes — falling back to text for full sim (rebuild with client --bake-content for OLC1 v3+)"
                    );
                } else {
                    finish_cache_boot(&mut db, root);
                    info!(
                        objects = db.object_count(),
                        transitions = db.transition_count,
                        last_use = db.last_use_transition_count,
                        max_use = db.transitions_max_use.len(),
                        biomes = db.biome_spawn.len(),
                        version = db.data_version,
                        objects_ms = db.load_objects_ms,
                        transitions_ms = db.load_transitions_ms,
                        path = %cache.display(),
                        "content ready (binary cache)"
                    );
                    return Ok(db);
                }
            }
            Err(e) => {
                warn!(error = %e, "binary cache rejected — falling back to text");
            }
        }
    }
    crate::load_content(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_binary::{
        encode_olc1, encode_olt1, push_f32, push_i32, push_str_u16, push_u16, push_u32, push_u8,
        write_blob_header, Olc1Record, Olt1Record, OBJ_F_CONTAINABLE, OBJ_F_PERMANENT,
        OLT1_F_CATEGORY_EXPANDED, TR_F_LAST_USE_ACTOR, TR_F_MAX_USE_TARGET,
        TR_F_REVERSE_USE_ACTOR, TR_F_SWITCH_NUMBER_OF_USES,
    };

    fn header(out: &mut Vec<u8>, magic: &[u8; 4], fmt: u32, ver: u32, count: u32) {
        write_blob_header(out, magic, fmt, ver, count, 0);
    }

    fn sample_olc1_rec() -> Olc1Record {
        Olc1Record {
            id: 30,
            name: "Bush".into(),
            description: "Wild".into(),
            flags: OBJ_F_PERMANENT,
            food_value: 0,
            num_uses: 0,
            min_pickup_age: 0.0,
            person: 0,
            held_x: 0.0,
            held_y: 0.0,
            clothing: b'n',
            cloth_x: 0.0,
            cloth_y: 0.0,
            num_slots: 0,
            slot_pos: vec![],
            sprites: vec![],
            dummy_ids: vec![],
            dummy_parent: 0,
            use_chance: 0.0,
            left_blocking_radius: 0,
            right_blocking_radius: 0,
            map_chance: 0.5,
            heat_value: 1.0,
            speed_mult: 1.0,
            r_value: 0.0,
            decay_factor: 1.0,
            decays_to_obj: 0,
            winter_decay_factor: 0.0,
            spring_regrow_factor: 0.0,
            biomes: vec![1],
            creation_sound: String::new(),
            using_sound: String::new(),
            eating_sound: String::new(),
            decay_sound: String::new(),
            use_vanish_idx: vec![],
            use_appear_idx: vec![],
            variable_dummy_ids: vec![9001],
            deadly_distance: 0.0,
            use_distance: 1,
            moves: 0,
            contain_size: 0.0,
            slot_size: 1.0,
        }
    }

    #[test]
    fn olc1_v2_server_load() {
        let mut out = Vec::new();
        header(&mut out, OLC1_MAGIC, 2, 11, 1);
        push_i32(&mut out, 55);
        push_str_u16(&mut out, "Berry");
        push_str_u16(&mut out, "Berry bush");
        push_u32(&mut out, OBJ_F_CONTAINABLE);
        push_i32(&mut out, 2);
        push_i32(&mut out, 4);
        push_f32(&mut out, 0.0);
        push_i32(&mut out, 0);
        push_f32(&mut out, 0.0);
        push_f32(&mut out, 0.0);
        push_u8(&mut out, b'n');
        push_f32(&mut out, 0.0);
        push_f32(&mut out, 0.0);
        push_i32(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 2);
        push_i32(&mut out, 1000);
        push_i32(&mut out, 1001);
        push_i32(&mut out, 0);
        push_f32(&mut out, 0.25);

        let mut db = ContentDb::default();
        load_olc1(&out, &mut db).unwrap();
        let o = db.get(55).unwrap();
        assert_eq!(o.food_value, 2);
        assert_eq!(o.num_uses, 4);
        assert_eq!(o.dummy_ids, vec![1000, 1001]);
        assert!((o.use_chance - 0.25).abs() < 1e-5);
        assert!((o.map_chance - 0.0).abs() < 1e-5, "v2 has no spawn meta");
        assert_eq!(db.dummy_parent.get(&1000), Some(&55));
        assert_eq!(db.wire_id_for_uses(55, 1), 1000);
        assert_eq!(db.wire_id_for_uses(55, 4), 55);
        // format < 7 → ObjectDef distance defaults
        assert_eq!(o.use_distance, 1);
        assert!((o.deadly_distance - 0.0).abs() < 1e-5);
        assert_eq!(o.moves, 0);
    }

    #[test]
    fn olc1_v3_server_sim_fields() {
        let mut out = Vec::new();
        header(&mut out, OLC1_MAGIC, 3, 12, 1);
        push_i32(&mut out, 30);
        push_str_u16(&mut out, "Bush");
        push_str_u16(&mut out, "Wild Gooseberry Bush");
        push_u32(&mut out, OBJ_F_PERMANENT);
        push_i32(&mut out, 0);
        push_i32(&mut out, 0);
        push_f32(&mut out, 3.0);
        push_i32(&mut out, 0);
        push_f32(&mut out, 0.0);
        push_f32(&mut out, 0.0);
        push_u8(&mut out, b'n');
        push_f32(&mut out, 0.0);
        push_f32(&mut out, 0.0);
        push_i32(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_i32(&mut out, 0);
        push_f32(&mut out, 0.0);
        push_i32(&mut out, 1);
        push_i32(&mut out, 2);
        push_f32(&mut out, 1.0);
        push_f32(&mut out, 2.0);
        push_f32(&mut out, 0.8);
        push_f32(&mut out, 0.5);
        push_f32(&mut out, 0.1);
        push_i32(&mut out, 618);
        push_f32(&mut out, 0.0);
        push_f32(&mut out, 0.0);
        push_u16(&mut out, 2);
        push_i32(&mut out, 0);
        push_i32(&mut out, 3);

        let mut db = ContentDb::default();
        load_olc1(&out, &mut db).unwrap();
        let o = db.get(30).unwrap();
        assert!(o.permanent);
        assert!((o.map_chance - 1.0).abs() < 1e-5);
        assert_eq!(o.biomes, vec![0, 3]);
        assert!((o.heat_value - 2.0).abs() < 1e-5);
        assert!((o.speed_mult - 0.8).abs() < 1e-5);
        assert!((o.r_value - 0.5).abs() < 1e-5);
        assert!((o.decay_factor - 0.1).abs() < 1e-5);
        assert_eq!(o.decays_to_obj, 618);
        assert!(db.biome_spawn.contains_key(&0));
        assert!(db.biome_spawn.contains_key(&3));
        assert!((db.biome_spawn[&0].total_chance - 1.0).abs() < 1e-5);
        assert_eq!(db.biome_spawn[&0].entries[0].0, 30);
    }

    #[test]
    fn olc1_v7_distances_server_load() {
        // OLC1-DISTANCES / binary_use_dist
        let mut rec = sample_olc1_rec();
        rec.id = 152; // Bow and Arrow
        rec.name = "Bow".into();
        rec.deadly_distance = 4.0;
        rec.use_distance = 5;
        rec.moves = 0;
        rec.variable_dummy_ids = vec![];
        let wolf = Olc1Record {
            id: 418,
            name: "Wolf".into(),
            description: "Wolf".into(),
            flags: 0,
            food_value: 0,
            num_uses: 0,
            min_pickup_age: 0.0,
            person: 0,
            held_x: 0.0,
            held_y: 0.0,
            clothing: b'n',
            cloth_x: 0.0,
            cloth_y: 0.0,
            num_slots: 0,
            slot_pos: vec![],
            sprites: vec![],
            dummy_ids: vec![],
            dummy_parent: 0,
            use_chance: 0.0,
            left_blocking_radius: 0,
            right_blocking_radius: 0,
            map_chance: 0.0,
            heat_value: 0.0,
            speed_mult: 1.0,
            r_value: 0.0,
            decay_factor: 1.0,
            decays_to_obj: 0,
            winter_decay_factor: 0.0,
            spring_regrow_factor: 0.0,
            biomes: vec![],
            creation_sound: String::new(),
            using_sound: String::new(),
            eating_sound: String::new(),
            decay_sound: String::new(),
            use_vanish_idx: vec![],
            use_appear_idx: vec![],
            variable_dummy_ids: vec![],
            deadly_distance: 0.5,
            use_distance: 1,
            moves: 2,
            contain_size: 0.0,
            slot_size: 1.0,
        };
        let bytes = encode_olc1(437, 0, &[rec, wolf]);
        assert_eq!(bytes[4..8], 8u32.to_le_bytes()); // write path OLC1 v8
        let mut db = ContentDb::default();
        load_olc1(&bytes, &mut db).unwrap();
        let bow = db.get(152).unwrap();
        assert_eq!(bow.use_distance, 5);
        assert!((bow.deadly_distance - 4.0).abs() < 1e-5);
        assert_eq!(bow.effective_use_distance(), 5);
        // v8 defaults when unset on sample_olc1_rec
        assert!((bow.contain_size - 0.0).abs() < 1e-5);
        assert!((bow.slot_size - 1.0).abs() < 1e-5);
        let w = db.get(418).unwrap();
        assert_eq!(w.moves, 2);
        assert!(w.is_animal());
        assert!((w.deadly_distance - 0.5).abs() < 1e-5);
    }

    /// Default-baked OLC1 (0/1/0) + finish_cache_boot weapon/animal patches.
    // Haxe: ServerSettings.PatchObjectData weapon + animal deadlyDistance
    #[test]
    fn finish_cache_boot_weapon_and_animal_distance_patches() {
        // Client historically baked defaults; boot patches restore IS-CLOSE ranges.
        let mut bow = sample_olc1_rec();
        bow.id = 152;
        bow.name = "Bow".into();
        bow.deadly_distance = 0.0;
        bow.use_distance = 1;
        bow.moves = 0;
        bow.variable_dummy_ids = vec![];
        let mut wolf = sample_olc1_rec();
        wolf.id = 418;
        wolf.name = "Wolf".into();
        wolf.deadly_distance = 1.0; // object-file value before animal factor
        wolf.use_distance = 1;
        wolf.moves = 0; // moves stamped from transitions when present
        wolf.variable_dummy_ids = vec![];
        let bytes = encode_olc1(1, 0, &[bow, wolf]);
        let mut db = ContentDb::default();
        load_olc1(&bytes, &mut db).unwrap();
        // No categories root needed for range patches alone.
        apply_default_weapon_range_patches(&mut db);
        apply_default_animal_deadly_distance_patches(&mut db);
        let b = db.get(152).unwrap();
        assert_eq!(b.use_distance, 5);
        assert!((b.deadly_distance - 4.0).abs() < 1e-5);
        let w = db.get(418).unwrap();
        assert!((w.deadly_distance - 0.5).abs() < 1e-5);
    }

    #[test]
    fn olc1_v6_shared_encode_server_load() {
        // encode_olc1 now writes v8; distances/sizes default when unset on record.
        let rec = sample_olc1_rec();
        let bytes = encode_olc1(437, 0, &[rec]);
        let mut db = ContentDb::default();
        load_olc1(&bytes, &mut db).unwrap();
        let o = db.get(30).unwrap();
        assert!((o.map_chance - 0.5).abs() < 1e-5);
        assert_eq!(o.biomes, vec![1]);
        // variable dummies not stored on ObjectDef; parse must still consume trailer.
        assert_eq!(db.objects.len(), 1);
        assert_eq!(o.use_distance, 1);
        assert_eq!(o.moves, 0);
    }

    #[test]
    fn olt1_v2_last_use_split() {
        let mut out = Vec::new();
        header(&mut out, OLT1_MAGIC, 2, 1, 2);
        push_i32(&mut out, 10);
        push_i32(&mut out, 20);
        push_i32(&mut out, 9);
        push_i32(&mut out, 19);
        out.push(0);
        push_f32(&mut out, 0.0);
        push_f32(&mut out, 0.0);
        push_f32(&mut out, 0.0);
        push_i32(&mut out, 0);
        push_i32(&mut out, 0);
        push_i32(&mut out, 10);
        push_i32(&mut out, 20);
        push_i32(&mut out, 11);
        push_i32(&mut out, 21);
        out.push(TR_F_LAST_USE_ACTOR | TR_F_REVERSE_USE_ACTOR);
        push_f32(&mut out, 1.5);
        push_f32(&mut out, 1.0);
        push_f32(&mut out, 0.5);
        push_i32(&mut out, 2);
        push_i32(&mut out, 3);

        let mut db = ContentDb::default();
        load_olt1(&out, &mut db).unwrap();
        assert_eq!(db.transitions.len(), 1);
        assert_eq!(db.transitions_last_use.len(), 1);
        assert_eq!(db.find_transition(10, 20).unwrap().new_actor_id, 9);
        let la = db.find_transition_last_use(10, 20).unwrap();
        assert_eq!(la.new_actor_id, 11);
        assert!(la.reverse_use_actor);
        assert_eq!(la.move_dist, 2);
    }

    #[test]
    fn olt1_max_use_and_switch_via_shared() {
        let normal = Olt1Record {
            actor_id: 33,
            target_id: 1096,
            new_actor_id: 0,
            new_target_id: 1096,
            flags: 0,
            auto_decay_seconds: 0.0,
            actor_min_use_fraction: 0.0,
            target_min_use_fraction: 0.0,
            move_dist: 0,
            desired_move_dist: 0,
        };
        let max_use = Olt1Record {
            actor_id: 33,
            target_id: 1096,
            new_actor_id: 0,
            new_target_id: 1097,
            flags: TR_F_MAX_USE_TARGET | TR_F_SWITCH_NUMBER_OF_USES,
            auto_decay_seconds: 0.0,
            actor_min_use_fraction: 0.0,
            target_min_use_fraction: 1.0,
            move_dist: 0,
            desired_move_dist: 0,
        };
        let bytes = encode_olt1(1, OLT1_F_CATEGORY_EXPANDED, &[normal, max_use]);
        let mut db = ContentDb::default();
        load_olt1(&bytes, &mut db).unwrap();
        assert_eq!(db.transitions.len(), 1);
        assert_eq!(db.transitions_max_use.len(), 1);
        assert_eq!(
            db.transitions_max_use.get(&(33, 1096)).unwrap().new_target_id,
            1097
        );
        assert!(
            db.transitions_max_use
                .get(&(33, 1096))
                .unwrap()
                .switch_number_of_uses
        );
        // normal row does not steal max-use
        assert_eq!(db.find_transition(33, 1096).unwrap().new_target_id, 1096);
    }

    #[test]
    fn cache_dir_roundtrip_files() {
        let tmp = std::env::temp_dir().join(format!(
            "ol_content_bin_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mut olc1 = Vec::new();
        header(&mut olc1, OLC1_MAGIC, 1, 99, 1);
        push_i32(&mut olc1, 1);
        push_str_u16(&mut olc1, "A");
        push_str_u16(&mut olc1, "A");
        push_u32(&mut olc1, 0);
        push_i32(&mut olc1, 0);
        push_i32(&mut olc1, 0);
        push_f32(&mut olc1, 0.0);
        push_i32(&mut olc1, 0);
        push_f32(&mut olc1, 0.0);
        push_f32(&mut olc1, 0.0);
        push_u8(&mut olc1, b'n');
        push_f32(&mut olc1, 0.0);
        push_f32(&mut olc1, 0.0);
        push_i32(&mut olc1, 0);
        push_u16(&mut olc1, 0);
        push_u16(&mut olc1, 0);
        push_u16(&mut olc1, 0);
        push_i32(&mut olc1, 0);

        let mut olt1 = Vec::new();
        header(&mut olt1, OLT1_MAGIC, 1, 99, 1);
        push_i32(&mut olt1, 1);
        push_i32(&mut olt1, 0);
        push_i32(&mut olt1, 0);
        push_i32(&mut olt1, 2);
        olt1.push(0);
        push_f32(&mut olt1, 0.0);

        let olc1_sha = sha1_hex(&olc1);
        let olt1_sha = sha1_hex(&olt1);
        fs::write(tmp.join("olc1_objects.bin"), &olc1).unwrap();
        fs::write(tmp.join("olt1_transitions.bin"), &olt1).unwrap();
        fs::write(
            tmp.join("manifest.json"),
            format!(
                r#"{{
  "format": 1,
  "data_version": 99,
  "created_utc": "unix:0",
  "source": "test",
  "blobs": {{
    "olc1_objects.bin": {{ "sha1": "{olc1_sha}", "bytes": {}, "count": 1 }},
    "olt1_transitions.bin": {{ "sha1": "{olt1_sha}", "bytes": {}, "count": 1 }}
  }}
}}"#,
                olc1.len(),
                olt1.len()
            ),
        )
        .unwrap();

        let db = load_from_cache(&tmp, Some(99)).unwrap();
        assert_eq!(db.data_version, 99);
        assert!(db.get(1).is_some());
        assert_eq!(db.find_transition(1, 0).unwrap().new_target_id, 2);
        assert!(load_from_cache(&tmp, Some(1)).is_err());
        let _ = fs::remove_dir_all(&tmp);
    }
}
