//! OLC1 object bank binary format (v1..=v8).

use crate::io::{
    parse_blob_header, push_f32, push_i32, push_str_u16, push_u16, push_u32, push_u8, read_f32,
    read_i32, read_str_u16, read_u16, read_u32, read_u8, write_blob_header, BlobHeader,
};

/// OLC1 magic — object bank cache.
pub const OLC1_MAGIC: &[u8; 4] = b"OLC1";

/// OLC1 write path format (v8 = contain_size / slot_size trailer).
///
/// // Haxe: ObjectData.containSize + slotSize; text keys containSize / slotsSize
pub const OLC1_FORMAT_VERSION: u32 = 8;
pub const OLC1_FORMAT_VERSION_V1: u32 = 1;
pub const OLC1_FORMAT_VERSION_V2: u32 = 2;
pub const OLC1_FORMAT_VERSION_V3: u32 = 3;
pub const OLC1_FORMAT_VERSION_V4: u32 = 4;
pub const OLC1_FORMAT_VERSION_V5: u32 = 5;
pub const OLC1_FORMAT_VERSION_V6: u32 = 6;
pub const OLC1_FORMAT_VERSION_V7: u32 = 7;
pub const OLC1_FORMAT_VERSION_V8: u32 = 8;

// Object flags (OLC1 record).
pub const OBJ_F_PERMANENT: u32 = 1 << 0;
pub const OBJ_F_BLOCKS_WALKING: u32 = 1 << 1;
pub const OBJ_F_CONTAINABLE: u32 = 1 << 2;
pub const OBJ_F_FLOOR: u32 = 1 << 3;
pub const OBJ_F_DRAW_BEHIND_PLAYER: u32 = 1 << 4;
pub const OBJ_F_HELD_IN_HAND: u32 = 1 << 5;
pub const OBJ_F_RIDEABLE: u32 = 1 << 6;
pub const OBJ_F_SIDE_ACCESS: u32 = 1 << 7;
pub const OBJ_F_NO_BACK_ACCESS: u32 = 1 << 8;
pub const OBJ_F_CREATION_SOUND_INITIAL_ONLY: u32 = 1 << 9;
pub const OBJ_F_CREATION_SOUND_FORCE: u32 = 1 << 10;
pub const OBJ_F_FLOOR_HUGGING: u32 = 1 << 11;

// Sprite flags.
pub const SPR_F_H_FLIP: u8 = 1 << 0;
pub const SPR_F_INVIS_HOLDING: u8 = 1 << 1;
pub const SPR_F_INVIS_WORN: u8 = 1 << 2;
pub const SPR_F_BEHIND_SLOTS: u8 = 1 << 3;
pub const SPR_F_BEHIND_PLAYER: u8 = 1 << 4;
pub const SPR_BODY_PART_SHIFT: u8 = 5;
pub const SPR_BODY_PART_MASK: u8 = 0b111 << SPR_BODY_PART_SHIFT;
pub const SPR_PART_NONE: u8 = 0;
pub const SPR_PART_BODY: u8 = 1;
pub const SPR_PART_HEAD: u8 = 2;
pub const SPR_PART_BACK_FOOT: u8 = 3;
pub const SPR_PART_FRONT_FOOT: u8 = 4;

/// One sprite entry inside an OLC1 object record.
#[derive(Debug, Clone, PartialEq)]
pub struct Olc1Sprite {
    pub sprite_id: i32,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
    /// Packed sprite flags + body-part nibble (bits 5–7).
    pub flags: u8,
    pub age_start: f32,
    pub age_end: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub parent: i32,
}

impl Olc1Sprite {
    pub fn h_flip(&self) -> bool {
        self.flags & SPR_F_H_FLIP != 0
    }
    pub fn invis_holding(&self) -> bool {
        self.flags & SPR_F_INVIS_HOLDING != 0
    }
    pub fn invis_worn(&self) -> bool {
        self.flags & SPR_F_INVIS_WORN != 0
    }
    pub fn behind_slots(&self) -> bool {
        self.flags & SPR_F_BEHIND_SLOTS != 0
    }
    pub fn behind_player(&self) -> bool {
        self.flags & SPR_F_BEHIND_PLAYER != 0
    }
    pub fn body_part(&self) -> u8 {
        (self.flags & SPR_BODY_PART_MASK) >> SPR_BODY_PART_SHIFT
    }
}

/// Dense OLC1 object record (all format trailers normalized; missing = defaults).
#[derive(Debug, Clone, PartialEq)]
pub struct Olc1Record {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub flags: u32,
    pub food_value: i32,
    pub num_uses: i32,
    pub min_pickup_age: f32,
    pub person: i32,
    pub held_x: f32,
    pub held_y: f32,
    pub clothing: u8,
    pub cloth_x: f32,
    pub cloth_y: f32,
    pub num_slots: i32,
    pub slot_pos: Vec<(f32, f32)>,
    pub sprites: Vec<Olc1Sprite>,
    pub dummy_ids: Vec<i32>,
    pub dummy_parent: i32,
    /// Format ≥ 2.
    pub use_chance: f32,
    /// Format ≥ 3.
    pub left_blocking_radius: i32,
    pub right_blocking_radius: i32,
    pub map_chance: f32,
    pub heat_value: f32,
    pub speed_mult: f32,
    pub r_value: f32,
    pub decay_factor: f32,
    pub decays_to_obj: i32,
    pub winter_decay_factor: f32,
    pub spring_regrow_factor: f32,
    pub biomes: Vec<i32>,
    /// Format ≥ 4.
    pub creation_sound: String,
    pub using_sound: String,
    pub eating_sound: String,
    pub decay_sound: String,
    /// Format ≥ 5 sparse indices into `sprites`.
    pub use_vanish_idx: Vec<i32>,
    pub use_appear_idx: Vec<i32>,
    /// Format ≥ 6.
    pub variable_dummy_ids: Vec<i32>,
    /// Format ≥ 7 — Haxe `ObjectData.deadlyDistance` (float tiles).
    pub deadly_distance: f32,
    /// Format ≥ 7 — Haxe `ObjectData.useDistance` (int tiles; default 1).
    pub use_distance: i32,
    /// Format ≥ 7 — Haxe `ObjectData.moves` (animal walk; often stamped from time-move).
    pub moves: i32,
    /// Format ≥ 8 — Haxe `ObjectData.containSize` (default 0).
    pub contain_size: f32,
    /// Format ≥ 8 — Haxe `ObjectData.slotSize` (default 1; text key `slotsSize`).
    pub slot_size: f32,
}

/// Parsed OLC1 blob.
#[derive(Debug, Clone)]
pub struct Olc1Blob {
    pub header: BlobHeader,
    pub records: Vec<Olc1Record>,
}

pub fn parse_olc1(data: &[u8]) -> Result<Olc1Blob, String> {
    let header = parse_blob_header(data, OLC1_MAGIC, OLC1_FORMAT_VERSION)?;
    let mut off = 24usize;
    let mut records = Vec::with_capacity(header.count);
    for _ in 0..header.count {
        records.push(read_olc1_record(data, &mut off, header.format)?);
    }
    Ok(Olc1Blob { header, records })
}

pub fn encode_olc1(data_version: u32, flags: u32, records: &[Olc1Record]) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + records.len() * 128);
    write_blob_header(
        &mut out,
        OLC1_MAGIC,
        OLC1_FORMAT_VERSION,
        data_version,
        records.len() as u32,
        flags,
    );
    for r in records {
        write_olc1_record(&mut out, r);
    }
    out
}

pub fn read_olc1_record(data: &[u8], off: &mut usize, format: u32) -> Result<Olc1Record, String> {
    let id = read_i32(data, off)?;
    let name = read_str_u16(data, off)?;
    let description = read_str_u16(data, off)?;
    let flags = read_u32(data, off)?;
    let food_value = read_i32(data, off)?;
    let num_uses = read_i32(data, off)?;
    let min_pickup_age = read_f32(data, off)?;
    let person = read_i32(data, off)?;
    let held_x = read_f32(data, off)?;
    let held_y = read_f32(data, off)?;
    let clothing = read_u8(data, off)?;
    let cloth_x = read_f32(data, off)?;
    let cloth_y = read_f32(data, off)?;
    let num_slots = read_i32(data, off)?;

    let n_slots = read_u16(data, off)? as usize;
    let mut slot_pos = Vec::with_capacity(n_slots);
    for _ in 0..n_slots {
        let x = read_f32(data, off)?;
        let y = read_f32(data, off)?;
        slot_pos.push((x, y));
    }

    let n_spr = read_u16(data, off)? as usize;
    let mut sprites = Vec::with_capacity(n_spr);
    for _ in 0..n_spr {
        sprites.push(read_olc1_sprite(data, off)?);
    }

    let n_dum = read_u16(data, off)? as usize;
    let mut dummy_ids = Vec::with_capacity(n_dum);
    for _ in 0..n_dum {
        dummy_ids.push(read_i32(data, off)?);
    }
    let dummy_parent = read_i32(data, off)?;

    let use_chance = if format >= OLC1_FORMAT_VERSION_V2 {
        read_f32(data, off)?
    } else {
        0.0
    };

    let (
        left_blocking_radius,
        right_blocking_radius,
        map_chance,
        heat_value,
        speed_mult,
        r_value,
        decay_factor,
        decays_to_obj,
        winter_decay_factor,
        spring_regrow_factor,
        biomes,
    ) = if format >= OLC1_FORMAT_VERSION_V3 {
        let left = read_i32(data, off)?;
        let right = read_i32(data, off)?;
        let map_chance = read_f32(data, off)?;
        let heat_value = read_f32(data, off)?;
        let speed_mult = read_f32(data, off)?;
        let r_value = read_f32(data, off)?;
        let decay_factor = read_f32(data, off)?;
        let decays_to_obj = read_i32(data, off)?;
        let winter_decay_factor = read_f32(data, off)?;
        let spring_regrow_factor = read_f32(data, off)?;
        let n_bio = read_u16(data, off)? as usize;
        let mut biomes = Vec::with_capacity(n_bio);
        for _ in 0..n_bio {
            biomes.push(read_i32(data, off)?);
        }
        (
            left,
            right,
            map_chance,
            heat_value,
            speed_mult,
            r_value,
            decay_factor,
            decays_to_obj,
            winter_decay_factor,
            spring_regrow_factor,
            biomes,
        )
    } else {
        (0, 0, 0.0, 0.0, 1.0, 0.0, 1.0, 0, 0.0, 0.0, Vec::new())
    };

    let (creation_sound, using_sound, eating_sound, decay_sound) =
        if format >= OLC1_FORMAT_VERSION_V4 {
            (
                read_str_u16(data, off)?,
                read_str_u16(data, off)?,
                read_str_u16(data, off)?,
                read_str_u16(data, off)?,
            )
        } else {
            (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            )
        };

    let (use_vanish_idx, use_appear_idx) = if format >= OLC1_FORMAT_VERSION_V5 {
        let n_v = read_u16(data, off)? as usize;
        let mut vanish = Vec::with_capacity(n_v);
        for _ in 0..n_v {
            vanish.push(read_i32(data, off)?);
        }
        let n_a = read_u16(data, off)? as usize;
        let mut appear = Vec::with_capacity(n_a);
        for _ in 0..n_a {
            appear.push(read_i32(data, off)?);
        }
        (vanish, appear)
    } else {
        (Vec::new(), Vec::new())
    };

    let variable_dummy_ids = if format >= OLC1_FORMAT_VERSION_V6 {
        let n_var = read_u16(data, off)? as usize;
        let mut ids = Vec::with_capacity(n_var);
        for _ in 0..n_var {
            ids.push(read_i32(data, off)?);
        }
        ids
    } else {
        Vec::new()
    };

    // Haxe: ObjectData.writeToFile deadlyDistance (float) then useDistance (i32)
    // moves is server-side animal walk class (not in Haxe client pack; OLC1 v7 cache field)
    let (deadly_distance, use_distance, moves) = if format >= OLC1_FORMAT_VERSION_V7 {
        let deadly = read_f32(data, off)?;
        let use_d = read_i32(data, off)?;
        let moves = read_i32(data, off)?;
        (deadly, use_d, moves)
    } else {
        (0.0, 1, 0)
    };

    // Haxe: ObjectData.containSize / slotSize — OLC1 v8 trailer
    let (contain_size, slot_size) = if format >= OLC1_FORMAT_VERSION_V8 {
        let cs = read_f32(data, off)?;
        let ss = read_f32(data, off)?;
        (cs, ss)
    } else {
        (0.0, 1.0)
    };

    Ok(Olc1Record {
        id,
        name,
        description,
        flags,
        food_value,
        num_uses,
        min_pickup_age,
        person,
        held_x,
        held_y,
        clothing,
        cloth_x,
        cloth_y,
        num_slots,
        slot_pos,
        sprites,
        dummy_ids,
        dummy_parent,
        use_chance,
        left_blocking_radius,
        right_blocking_radius,
        map_chance,
        heat_value,
        speed_mult,
        r_value,
        decay_factor,
        decays_to_obj,
        winter_decay_factor,
        spring_regrow_factor,
        biomes,
        creation_sound,
        using_sound,
        eating_sound,
        decay_sound,
        use_vanish_idx,
        use_appear_idx,
        variable_dummy_ids,
        deadly_distance,
        use_distance,
        moves,
        contain_size,
        slot_size,
    })
}

fn read_olc1_sprite(data: &[u8], off: &mut usize) -> Result<Olc1Sprite, String> {
    Ok(Olc1Sprite {
        sprite_id: read_i32(data, off)?,
        x: read_f32(data, off)?,
        y: read_f32(data, off)?,
        rot: read_f32(data, off)?,
        flags: read_u8(data, off)?,
        age_start: read_f32(data, off)?,
        age_end: read_f32(data, off)?,
        r: read_f32(data, off)?,
        g: read_f32(data, off)?,
        b: read_f32(data, off)?,
        parent: read_i32(data, off)?,
    })
}

/// Write one OLC1 record at current write-path format (v8 trailers always present).
pub fn write_olc1_record(out: &mut Vec<u8>, def: &Olc1Record) {
    push_i32(out, def.id);
    push_str_u16(out, &def.name);
    push_str_u16(out, &def.description);
    push_u32(out, def.flags);
    push_i32(out, def.food_value);
    push_i32(out, def.num_uses);
    push_f32(out, def.min_pickup_age);
    push_i32(out, def.person);
    push_f32(out, def.held_x);
    push_f32(out, def.held_y);
    push_u8(out, def.clothing);
    push_f32(out, def.cloth_x);
    push_f32(out, def.cloth_y);
    push_i32(out, def.num_slots);

    let n_slots = def.slot_pos.len().min(u16::MAX as usize) as u16;
    push_u16(out, n_slots);
    for i in 0..n_slots as usize {
        push_f32(out, def.slot_pos[i].0);
        push_f32(out, def.slot_pos[i].1);
    }

    let n_spr = def.sprites.len().min(u16::MAX as usize) as u16;
    push_u16(out, n_spr);
    for i in 0..n_spr as usize {
        write_olc1_sprite(out, &def.sprites[i]);
    }

    let n_dum = def.dummy_ids.len().min(u16::MAX as usize) as u16;
    push_u16(out, n_dum);
    for i in 0..n_dum as usize {
        push_i32(out, def.dummy_ids[i]);
    }
    push_i32(out, def.dummy_parent);
    // format 2
    push_f32(out, def.use_chance);
    // format 3
    push_i32(out, def.left_blocking_radius);
    push_i32(out, def.right_blocking_radius);
    push_f32(out, def.map_chance);
    push_f32(out, def.heat_value);
    push_f32(out, def.speed_mult);
    push_f32(out, def.r_value);
    push_f32(out, def.decay_factor);
    push_i32(out, def.decays_to_obj);
    push_f32(out, def.winter_decay_factor);
    push_f32(out, def.spring_regrow_factor);
    let n_bio = def.biomes.len().min(u16::MAX as usize) as u16;
    push_u16(out, n_bio);
    for i in 0..n_bio as usize {
        push_i32(out, def.biomes[i]);
    }
    // format 4
    push_str_u16(out, &def.creation_sound);
    push_str_u16(out, &def.using_sound);
    push_str_u16(out, &def.eating_sound);
    push_str_u16(out, &def.decay_sound);
    // format 5
    let n_v = def.use_vanish_idx.len().min(u16::MAX as usize) as u16;
    push_u16(out, n_v);
    for i in 0..n_v as usize {
        push_i32(out, def.use_vanish_idx[i]);
    }
    let n_a = def.use_appear_idx.len().min(u16::MAX as usize) as u16;
    push_u16(out, n_a);
    for i in 0..n_a as usize {
        push_i32(out, def.use_appear_idx[i]);
    }
    // format 6
    let n_var = def.variable_dummy_ids.len().min(u16::MAX as usize) as u16;
    push_u16(out, n_var);
    for i in 0..n_var as usize {
        push_i32(out, def.variable_dummy_ids[i]);
    }
    // format 7 — deadlyDistance (f32), useDistance (i32), moves (i32)
    // Haxe: ObjectData.writeToFile / readFromFile
    push_f32(out, def.deadly_distance);
    push_i32(out, def.use_distance);
    push_i32(out, def.moves);
    // format 8 — containSize (f32), slotSize (f32)
    // Haxe: ObjectData.containSize / slotSize
    push_f32(out, def.contain_size);
    push_f32(out, def.slot_size);
}

fn write_olc1_sprite(out: &mut Vec<u8>, s: &Olc1Sprite) {
    push_i32(out, s.sprite_id);
    push_f32(out, s.x);
    push_f32(out, s.y);
    push_f32(out, s.rot);
    push_u8(out, s.flags);
    push_f32(out, s.age_start);
    push_f32(out, s.age_end);
    push_f32(out, s.r);
    push_f32(out, s.g);
    push_f32(out, s.b);
    push_i32(out, s.parent);
}
