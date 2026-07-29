//! OLT1 transition bank binary format (v1..=v2).

use crate::io::{
    parse_blob_header, peek_blob_flags, push_f32, push_i32, read_f32, read_i32, read_u8,
    write_blob_header, BlobHeader,
};

/// OLT1 magic — transition bank cache.
pub const OLT1_MAGIC: &[u8; 4] = b"OLT1";

/// OLT1 write path format (v2 = full server fields).
pub const OLT1_FORMAT_VERSION: u32 = 2;
/// Legacy OLT1 craft subset.
pub const OLT1_FORMAT_VERSION_V1: u32 = 1;

/// OLT1 header flags (blob offset 16): lite+pattern category transitions baked in.
pub const OLT1_F_CATEGORY_EXPANDED: u32 = 1 << 0;

// Transition flags (OLT1 record).
pub const TR_F_LAST_USE_ACTOR: u8 = 1 << 0;
pub const TR_F_LAST_USE_TARGET: u8 = 1 << 1;
pub const TR_F_REVERSE_USE_ACTOR: u8 = 1 << 2;
pub const TR_F_REVERSE_USE_TARGET: u8 = 1 << 3;
pub const TR_F_NO_USE_ACTOR: u8 = 1 << 4;
pub const TR_F_NO_USE_TARGET: u8 = 1 << 5;
/// Record belongs in `transitions_max_use` (Haxe maxUseTarget table).
pub const TR_F_MAX_USE_TARGET: u8 = 1 << 6;
/// Haxe `switchNumberOfUses` (ServerSettings patches; dough/masa).
pub const TR_F_SWITCH_NUMBER_OF_USES: u8 = 1 << 7;

/// Which transition table a loaded record should land in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Olt1Table {
    Normal,
    LastUse,
    MaxUse,
}

/// One OLT1 transition record.
#[derive(Debug, Clone, PartialEq)]
pub struct Olt1Record {
    pub actor_id: i32,
    pub target_id: i32,
    pub new_actor_id: i32,
    pub new_target_id: i32,
    pub flags: u8,
    pub auto_decay_seconds: f32,
    /// Format ≥ 2.
    pub actor_min_use_fraction: f32,
    pub target_min_use_fraction: f32,
    pub move_dist: i32,
    pub desired_move_dist: i32,
}

impl Olt1Record {
    pub fn last_use_actor(&self) -> bool {
        self.flags & TR_F_LAST_USE_ACTOR != 0
    }
    pub fn last_use_target(&self) -> bool {
        self.flags & TR_F_LAST_USE_TARGET != 0
    }
    pub fn reverse_use_actor(&self) -> bool {
        self.flags & TR_F_REVERSE_USE_ACTOR != 0
    }
    pub fn reverse_use_target(&self) -> bool {
        self.flags & TR_F_REVERSE_USE_TARGET != 0
    }
    pub fn no_use_actor(&self) -> bool {
        self.flags & TR_F_NO_USE_ACTOR != 0
    }
    pub fn no_use_target(&self) -> bool {
        self.flags & TR_F_NO_USE_TARGET != 0
    }
    pub fn is_max_use(&self) -> bool {
        self.flags & TR_F_MAX_USE_TARGET != 0
    }
    pub fn switch_number_of_uses(&self) -> bool {
        self.flags & TR_F_SWITCH_NUMBER_OF_USES != 0
    }

    /// Route table: last-use bits win over max-use bit (matches client load order).
    pub fn table(&self) -> Olt1Table {
        if self.last_use_actor() || self.last_use_target() {
            Olt1Table::LastUse
        } else if self.is_max_use() {
            Olt1Table::MaxUse
        } else {
            Olt1Table::Normal
        }
    }

    /// Build packed flags from field bits.
    pub fn pack_flags(
        last_use_actor: bool,
        last_use_target: bool,
        reverse_use_actor: bool,
        reverse_use_target: bool,
        no_use_actor: bool,
        no_use_target: bool,
        max_use_table: bool,
        switch_number_of_uses: bool,
    ) -> u8 {
        let mut f = 0u8;
        if last_use_actor {
            f |= TR_F_LAST_USE_ACTOR;
        }
        if last_use_target {
            f |= TR_F_LAST_USE_TARGET;
        }
        if reverse_use_actor {
            f |= TR_F_REVERSE_USE_ACTOR;
        }
        if reverse_use_target {
            f |= TR_F_REVERSE_USE_TARGET;
        }
        if no_use_actor {
            f |= TR_F_NO_USE_ACTOR;
        }
        if no_use_target {
            f |= TR_F_NO_USE_TARGET;
        }
        if max_use_table {
            f |= TR_F_MAX_USE_TARGET;
        }
        if switch_number_of_uses {
            f |= TR_F_SWITCH_NUMBER_OF_USES;
        }
        f
    }
}

/// Parsed OLT1 blob.
#[derive(Debug, Clone)]
pub struct Olt1Blob {
    pub header: BlobHeader,
    pub records: Vec<Olt1Record>,
}

/// True when OLT1 bytes lack [`OLT1_F_CATEGORY_EXPANDED`] (legacy or unexpanded bake).
pub fn olt1_lacks_category_expanded(data: &[u8]) -> bool {
    if data.len() < 24 || &data[0..4] != OLT1_MAGIC {
        return true;
    }
    peek_blob_flags(data) & OLT1_F_CATEGORY_EXPANDED == 0
}

pub fn parse_olt1(data: &[u8]) -> Result<Olt1Blob, String> {
    let header = parse_blob_header(data, OLT1_MAGIC, OLT1_FORMAT_VERSION)?;
    let mut off = 24usize;
    let mut records = Vec::with_capacity(header.count);
    for _ in 0..header.count {
        records.push(read_olt1_record(data, &mut off, header.format)?);
    }
    Ok(Olt1Blob { header, records })
}

pub fn encode_olt1(data_version: u32, flags: u32, records: &[Olt1Record]) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + records.len() * 40);
    write_blob_header(
        &mut out,
        OLT1_MAGIC,
        OLT1_FORMAT_VERSION,
        data_version,
        records.len() as u32,
        flags,
    );
    for r in records {
        write_olt1_record(&mut out, r);
    }
    out
}

pub fn read_olt1_record(data: &[u8], off: &mut usize, format: u32) -> Result<Olt1Record, String> {
    let actor_id = read_i32(data, off)?;
    let target_id = read_i32(data, off)?;
    let new_actor_id = read_i32(data, off)?;
    let new_target_id = read_i32(data, off)?;
    let flags = read_u8(data, off)?;
    let auto_decay_seconds = read_f32(data, off)?;
    let (actor_min, target_min, move_dist, desired_move) = if format >= OLT1_FORMAT_VERSION {
        (
            read_f32(data, off)?,
            read_f32(data, off)?,
            read_i32(data, off)?,
            read_i32(data, off)?,
        )
    } else {
        (0.0, 0.0, 0, 0)
    };
    Ok(Olt1Record {
        actor_id,
        target_id,
        new_actor_id,
        new_target_id,
        flags,
        auto_decay_seconds,
        actor_min_use_fraction: actor_min,
        target_min_use_fraction: target_min,
        move_dist,
        desired_move_dist: desired_move,
    })
}

/// Write one OLT1 record at format-2 layout.
pub fn write_olt1_record(out: &mut Vec<u8>, tr: &Olt1Record) {
    push_i32(out, tr.actor_id);
    push_i32(out, tr.target_id);
    push_i32(out, tr.new_actor_id);
    push_i32(out, tr.new_target_id);
    out.push(tr.flags);
    push_f32(out, tr.auto_decay_seconds);
    push_f32(out, tr.actor_min_use_fraction);
    push_f32(out, tr.target_min_use_fraction);
    push_i32(out, tr.move_dist);
    push_i32(out, tr.desired_move_dist);
}
