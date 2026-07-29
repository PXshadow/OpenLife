//! Shared OLC1 / OLT1 binary content-cache format.
//!
//! Pure little-endian loaders/writers used by both:
//! - RustClient `content_binary` (full client object/transition mapping)
//! - RustServer `ol-content` (server sim subset)
//!
//! Layout: `docs/port/CONTENT_BINARY.md` (OpenLife RustClient tree).
//! Text `OneLifeData7` remains authoring SoT; these blobs are a fast-start cache.
//!
//! No heavy deps — only `std`. Consumers keep sha1/manifest/bake policy.

#![forbid(unsafe_code)]

mod io;
mod olc1;
mod olt1;

pub use io::{
    peek_blob_flags, peek_format, push_f32, push_i32, push_str_u16, push_u16, push_u32, push_u8,
    read_f32, read_i32, read_str_u16, read_u16, read_u32, read_u8, write_blob_header,
    BlobHeader, ParseError,
};
pub use olc1::{
    read_olc1_record, write_olc1_record, Olc1Blob, Olc1Record, Olc1Sprite, OBJ_F_BLOCKS_WALKING,
    OBJ_F_CONTAINABLE, OBJ_F_CREATION_SOUND_FORCE, OBJ_F_CREATION_SOUND_INITIAL_ONLY,
    OBJ_F_DRAW_BEHIND_PLAYER, OBJ_F_FLOOR, OBJ_F_FLOOR_HUGGING, OBJ_F_HELD_IN_HAND, OBJ_F_NO_BACK_ACCESS,
    OBJ_F_PERMANENT, OBJ_F_RIDEABLE, OBJ_F_SIDE_ACCESS, OLC1_FORMAT_VERSION, OLC1_FORMAT_VERSION_V1,
    OLC1_FORMAT_VERSION_V2, OLC1_FORMAT_VERSION_V3, OLC1_FORMAT_VERSION_V4, OLC1_FORMAT_VERSION_V5,
    OLC1_FORMAT_VERSION_V6, OLC1_FORMAT_VERSION_V7, OLC1_FORMAT_VERSION_V8, OLC1_MAGIC, SPR_BODY_PART_MASK, SPR_BODY_PART_SHIFT,
    SPR_F_BEHIND_PLAYER, SPR_F_BEHIND_SLOTS, SPR_F_H_FLIP, SPR_F_INVIS_HOLDING, SPR_F_INVIS_WORN,
    SPR_PART_BACK_FOOT, SPR_PART_BODY, SPR_PART_FRONT_FOOT, SPR_PART_HEAD, SPR_PART_NONE,
};
pub use olt1::{
    olt1_lacks_category_expanded, read_olt1_record, write_olt1_record, Olt1Blob, Olt1Record,
    Olt1Table, OLT1_FORMAT_VERSION, OLT1_FORMAT_VERSION_V1, OLT1_F_CATEGORY_EXPANDED, OLT1_MAGIC,
    TR_F_LAST_USE_ACTOR, TR_F_LAST_USE_TARGET, TR_F_MAX_USE_TARGET, TR_F_NO_USE_ACTOR,
    TR_F_NO_USE_TARGET, TR_F_REVERSE_USE_ACTOR, TR_F_REVERSE_USE_TARGET, TR_F_SWITCH_NUMBER_OF_USES,
};

/// Parse a full OLC1 blob into header + records (format 1..=7).
pub fn parse_olc1(data: &[u8]) -> Result<Olc1Blob, String> {
    olc1::parse_olc1(data)
}

/// Parse a full OLT1 blob into header + records (format 1..=2).
///
/// Record **bit6** (`TR_F_MAX_USE_TARGET`) and **bit7** (`TR_F_SWITCH_NUMBER_OF_USES`)
/// are preserved on each [`Olt1Record`] for consumers to route max-use / switch tables.
pub fn parse_olt1(data: &[u8]) -> Result<Olt1Blob, String> {
    olt1::parse_olt1(data)
}

/// Serialize OLC1 records (write path format = [`OLC1_FORMAT_VERSION`]).
pub fn encode_olc1(data_version: u32, flags: u32, records: &[Olc1Record]) -> Vec<u8> {
    olc1::encode_olc1(data_version, flags, records)
}

/// Serialize OLT1 records (write path format = [`OLT1_FORMAT_VERSION`]).
pub fn encode_olt1(data_version: u32, flags: u32, records: &[Olt1Record]) -> Vec<u8> {
    olt1::encode_olt1(data_version, flags, records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rec() -> Olc1Record {
        Olc1Record {
            id: 55,
            name: "Berry".into(),
            description: "Berry bush".into(),
            flags: OBJ_F_CONTAINABLE | OBJ_F_PERMANENT,
            food_value: 2,
            num_uses: 4,
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
            dummy_ids: vec![1000, 1001],
            dummy_parent: 0,
            use_chance: 0.25,
            left_blocking_radius: 0,
            right_blocking_radius: 0,
            map_chance: 1.0,
            heat_value: 0.0,
            speed_mult: 1.0,
            r_value: 0.0,
            decay_factor: 1.0,
            decays_to_obj: 0,
            winter_decay_factor: 0.0,
            spring_regrow_factor: 0.0,
            biomes: vec![0, 3],
            creation_sound: String::new(),
            using_sound: String::new(),
            eating_sound: String::new(),
            decay_sound: String::new(),
            use_vanish_idx: vec![],
            use_appear_idx: vec![],
            variable_dummy_ids: vec![2000],
            deadly_distance: 0.0,
            use_distance: 1,
            moves: 0,
            contain_size: 0.0,
            slot_size: 1.0,
        }
    }

    #[test]
    fn olt1_bit6_bit7_roundtrip() {
        let rec = Olt1Record {
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
        assert_eq!(rec.table(), Olt1Table::MaxUse);
        assert!(rec.switch_number_of_uses());
        let bytes = encode_olt1(99, OLT1_F_CATEGORY_EXPANDED, &[rec.clone()]);
        let blob = parse_olt1(&bytes).unwrap();
        assert_eq!(blob.header.data_version, 99);
        assert_eq!(blob.header.flags & OLT1_F_CATEGORY_EXPANDED, OLT1_F_CATEGORY_EXPANDED);
        assert_eq!(blob.records.len(), 1);
        let r = &blob.records[0];
        assert_eq!(r.table(), Olt1Table::MaxUse);
        assert!(r.switch_number_of_uses());
        assert_eq!(r.new_target_id, 1097);
        assert!((r.target_min_use_fraction - 1.0).abs() < 1e-5);
    }

    #[test]
    fn olc1_v7_roundtrip_minimal() {
        // Legacy name kept: write path is v8; still checks distance fields.
        let mut rec = sample_rec();
        rec.deadly_distance = 4.0;
        rec.use_distance = 5;
        rec.moves = 2;
        rec.contain_size = 2.0;
        rec.slot_size = 1.5;
        let bytes = encode_olc1(437, 0, &[rec]);
        assert_eq!(&bytes[0..4], OLC1_MAGIC);
        let blob = parse_olc1(&bytes).unwrap();
        assert_eq!(blob.header.format, OLC1_FORMAT_VERSION);
        assert_eq!(blob.header.format, OLC1_FORMAT_VERSION_V8);
        assert_eq!(blob.records.len(), 1);
        let o = &blob.records[0];
        assert_eq!(o.id, 55);
        assert_eq!(o.dummy_ids, vec![1000, 1001]);
        assert_eq!(o.variable_dummy_ids, vec![2000]);
        assert!((o.use_chance - 0.25).abs() < 1e-5);
        assert!((o.map_chance - 1.0).abs() < 1e-5);
        assert_eq!(o.biomes, vec![0, 3]);
        assert!(o.flags & OBJ_F_CONTAINABLE != 0);
        assert!((o.deadly_distance - 4.0).abs() < 1e-5);
        assert_eq!(o.use_distance, 5);
        assert_eq!(o.moves, 2);
        assert!((o.contain_size - 2.0).abs() < 1e-5);
        assert!((o.slot_size - 1.5).abs() < 1e-5);
    }

    #[test]
    fn olc1_v8_contain_slot_size_roundtrip() {
        let mut rec = sample_rec();
        rec.contain_size = 2.0;
        rec.slot_size = 0.5;
        let bytes = encode_olc1(1, 0, &[rec]);
        let blob = parse_olc1(&bytes).unwrap();
        assert_eq!(blob.header.format, OLC1_FORMAT_VERSION_V8);
        let o = &blob.records[0];
        assert!((o.contain_size - 2.0).abs() < 1e-5);
        assert!((o.slot_size - 0.5).abs() < 1e-5);
    }

    #[test]
    fn olc1_legacy_v2_loads() {
        let mut out = Vec::new();
        write_blob_header(&mut out, OLC1_MAGIC, 2, 11, 1, 0);
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
        let blob = parse_olc1(&out).unwrap();
        assert_eq!(blob.header.format, 2);
        let o = &blob.records[0];
        assert_eq!(o.dummy_ids, vec![1000, 1001]);
        assert!((o.map_chance - 0.0).abs() < 1e-5);
        assert!(o.variable_dummy_ids.is_empty());
        // format < 7 defaults
        assert!((o.deadly_distance - 0.0).abs() < 1e-5);
        assert_eq!(o.use_distance, 1);
        assert_eq!(o.moves, 0);
    }

    /// Legacy v6 (variableDummy only) still loads with distance defaults.
    #[test]
    fn olc1_legacy_v6_defaults_distances() {
        let mut rec = sample_rec();
        rec.deadly_distance = 9.0; // ignored when writing raw v6 below
        rec.use_distance = 9;
        rec.moves = 9;
        // Encode as current (v8), then rewrite header format to 6 and drop v7+v8 trailers.
        let mut bytes = encode_olc1(1, 0, &[rec.clone()]);
        // Overwrite format to 6
        bytes[4..8].copy_from_slice(&6u32.to_le_bytes());
        // Strip last 20 bytes: v7 (f32+i32+i32=12) + v8 (f32+f32=8)
        assert!(bytes.len() > 24 + 20);
        bytes.truncate(bytes.len() - 20);
        let blob = parse_olc1(&bytes).unwrap();
        assert_eq!(blob.header.format, 6);
        let o = &blob.records[0];
        assert_eq!(o.variable_dummy_ids, vec![2000]);
        assert!((o.deadly_distance - 0.0).abs() < 1e-5);
        assert_eq!(o.use_distance, 1);
        assert_eq!(o.moves, 0);
        assert!((o.contain_size - 0.0).abs() < 1e-5);
        assert!((o.slot_size - 1.0).abs() < 1e-5);
    }

    /// Legacy v7 still loads with contain/slot size defaults.
    #[test]
    fn olc1_legacy_v7_defaults_contain_slot() {
        let mut rec = sample_rec();
        rec.contain_size = 9.0;
        rec.slot_size = 9.0;
        let mut bytes = encode_olc1(1, 0, &[rec]);
        bytes[4..8].copy_from_slice(&7u32.to_le_bytes());
        // Drop v8 trailer only (2×f32 = 8)
        assert!(bytes.len() > 24 + 8);
        bytes.truncate(bytes.len() - 8);
        let blob = parse_olc1(&bytes).unwrap();
        assert_eq!(blob.header.format, 7);
        let o = &blob.records[0];
        assert!((o.contain_size - 0.0).abs() < 1e-5);
        assert!((o.slot_size - 1.0).abs() < 1e-5);
    }
}
