//! Little-endian binary helpers shared by OLC1 / OLT1.

use std::fmt;

/// Error from truncated or malformed blob IO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ParseError {}

impl From<ParseError> for String {
    fn from(e: ParseError) -> Self {
        e.0
    }
}

impl From<String> for ParseError {
    fn from(s: String) -> Self {
        ParseError(s)
    }
}

impl From<&str> for ParseError {
    fn from(s: &str) -> Self {
        ParseError(s.to_string())
    }
}

/// Parsed 24-byte OL* blob header (magic checked by caller).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlobHeader {
    pub format: u32,
    pub data_version: u32,
    pub count: usize,
    pub flags: u32,
}

pub fn push_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}
pub fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}
pub fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
pub fn push_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}
pub fn push_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}
pub fn push_str_u16(out: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    let len = (b.len().min(u16::MAX as usize)) as u16;
    push_u16(out, len);
    out.extend_from_slice(&b[..len as usize]);
}

pub fn read_u16(data: &[u8], off: &mut usize) -> Result<u16, String> {
    if *off + 2 > data.len() {
        return Err("truncated u16".into());
    }
    let v = u16::from_le_bytes(data[*off..*off + 2].try_into().unwrap());
    *off += 2;
    Ok(v)
}
pub fn read_u32(data: &[u8], off: &mut usize) -> Result<u32, String> {
    if *off + 4 > data.len() {
        return Err("truncated u32".into());
    }
    let v = u32::from_le_bytes(data[*off..*off + 4].try_into().unwrap());
    *off += 4;
    Ok(v)
}
pub fn read_i32(data: &[u8], off: &mut usize) -> Result<i32, String> {
    if *off + 4 > data.len() {
        return Err("truncated i32".into());
    }
    let v = i32::from_le_bytes(data[*off..*off + 4].try_into().unwrap());
    *off += 4;
    Ok(v)
}
pub fn read_f32(data: &[u8], off: &mut usize) -> Result<f32, String> {
    if *off + 4 > data.len() {
        return Err("truncated f32".into());
    }
    let v = f32::from_le_bytes(data[*off..*off + 4].try_into().unwrap());
    *off += 4;
    Ok(v)
}
pub fn read_u8(data: &[u8], off: &mut usize) -> Result<u8, String> {
    if *off >= data.len() {
        return Err("truncated u8".into());
    }
    let v = data[*off];
    *off += 1;
    Ok(v)
}
pub fn read_str_u16(data: &[u8], off: &mut usize) -> Result<String, String> {
    let len = read_u16(data, off)? as usize;
    if *off + len > data.len() {
        return Err("truncated string".into());
    }
    let s = String::from_utf8_lossy(&data[*off..*off + len]).into_owned();
    *off += len;
    Ok(s)
}

/// Write 24-byte OL* header (crc reserved 0).
pub fn write_blob_header(
    out: &mut Vec<u8>,
    magic: &[u8; 4],
    format: u32,
    data_version: u32,
    count: u32,
    flags: u32,
) {
    out.extend_from_slice(magic);
    push_u32(out, format);
    push_u32(out, data_version);
    push_u32(out, count);
    push_u32(out, flags);
    push_u32(out, 0); // header_crc32 reserved
}

/// Read blob header flags (offset 16). Returns 0 if header too short.
pub fn peek_blob_flags(data: &[u8]) -> u32 {
    if data.len() < 20 {
        return 0;
    }
    u32::from_le_bytes(data[16..20].try_into().unwrap())
}

/// Peek format_version (offset 4). None if too short or magic mismatch.
pub fn peek_format(data: &[u8], magic: &[u8; 4]) -> Option<u32> {
    if data.len() < 8 || &data[0..4] != magic {
        return None;
    }
    Some(u32::from_le_bytes(data[4..8].try_into().ok()?))
}

/// Parse 24-byte blob header. Accepts `format` in `1..=max_format`.
pub fn parse_blob_header(
    data: &[u8],
    magic: &[u8; 4],
    max_format: u32,
) -> Result<BlobHeader, String> {
    if data.len() < 24 {
        return Err("blob too short".into());
    }
    if &data[0..4] != magic {
        return Err(format!(
            "bad magic (want {:?}, got {:?})",
            std::str::from_utf8(magic).unwrap_or("?"),
            String::from_utf8_lossy(&data[0..4])
        ));
    }
    let format = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if format < 1 || format > max_format {
        return Err(format!(
            "unsupported format {format} (want 1..={max_format})"
        ));
    }
    let data_version = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let flags = u32::from_le_bytes(data[16..20].try_into().unwrap());
    Ok(BlobHeader {
        format,
        data_version,
        count,
        flags,
    })
}
