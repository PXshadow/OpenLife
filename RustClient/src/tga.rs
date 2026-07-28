//! TGA loader (Haxe `TgaData` / OHOL sprites `*.tga`).
//!
//! Supports uncompressed and RLE true-color 24/32-bit TGA.

use std::io::{self, Cursor, Read};
use std::path::Path;

/// Decoded image as premultiplied-friendly RGBA8 (straight alpha).
#[derive(Debug, Clone)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    /// Row-major RGBA, top-to-bottom (TGA often bottom-up — we flip).
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [0, 0, 0, 0];
        }
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ]
    }
}

/// Load TGA from path.
pub fn load_tga_path(path: impl AsRef<Path>) -> io::Result<RgbaImage> {
    let data = std::fs::read(path)?;
    load_tga_bytes(&data)
}

/// Load TGA from memory.
pub fn load_tga_bytes(data: &[u8]) -> io::Result<RgbaImage> {
    if data.len() < 18 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "TGA too short"));
    }
    let id_len = data[0] as usize;
    let color_map_type = data[1];
    let image_type = data[2];
    // color map skip
    let cm_len = u16::from_le_bytes([data[5], data[6]]) as usize;
    let cm_entry = data[7] as usize;
    let cm_bytes = cm_len * ((cm_entry + 7) / 8);

    let width = u16::from_le_bytes([data[12], data[13]]) as u32;
    let height = u16::from_le_bytes([data[14], data[15]]) as u32;
    let bpp = data[16];
    let descriptor = data[17];
    let top_origin = (descriptor & 0x20) != 0;

    if width == 0 || height == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "empty TGA"));
    }
    if color_map_type != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "color-mapped TGA not supported",
        ));
    }

    let pixel_bytes = match bpp {
        24 => 3,
        32 => 4,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported bpp {bpp}"),
            ))
        }
    };

    let data_offset = 18 + id_len + cm_bytes;
    if data.len() < data_offset {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "TGA header overrun"));
    }
    let mut src = Cursor::new(&data[data_offset..]);

    let n_pixels = (width * height) as usize;
    let mut raw = vec![0u8; n_pixels * pixel_bytes];

    match image_type {
        2 => {
            // Uncompressed true-color
            src.read_exact(&mut raw)?;
        }
        10 => {
            // RLE true-color
            decode_rle(&mut src, &mut raw, pixel_bytes)?;
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported TGA type {image_type}"),
            ))
        }
    }

    // Convert BGR(A) → RGBA and flip if bottom-up
    let mut pixels = vec![0u8; n_pixels * 4];
    for y in 0..height {
        let src_y = if top_origin { y } else { height - 1 - y };
        for x in 0..width {
            let si = ((src_y * width + x) as usize) * pixel_bytes;
            let di = ((y * width + x) as usize) * 4;
            pixels[di] = raw[si + 2]; // R
            pixels[di + 1] = raw[si + 1]; // G
            pixels[di + 2] = raw[si]; // B
            pixels[di + 3] = if pixel_bytes == 4 { raw[si + 3] } else { 255 };
        }
    }

    Ok(RgbaImage {
        width,
        height,
        pixels,
    })
}

fn decode_rle(src: &mut impl Read, out: &mut [u8], pixel_bytes: usize) -> io::Result<()> {
    let mut written = 0usize;
    while written < out.len() {
        let mut hdr = [0u8; 1];
        src.read_exact(&mut hdr)?;
        let count = (hdr[0] & 0x7F) as usize + 1;
        if hdr[0] & 0x80 != 0 {
            // RLE packet
            let mut px = vec![0u8; pixel_bytes];
            src.read_exact(&mut px)?;
            for _ in 0..count {
                if written + pixel_bytes > out.len() {
                    break;
                }
                out[written..written + pixel_bytes].copy_from_slice(&px);
                written += pixel_bytes;
            }
        } else {
            // raw packet
            let n = count * pixel_bytes;
            if written + n > out.len() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "RLE overrun"));
            }
            src.read_exact(&mut out[written..written + n])?;
            written += n;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_sprite_tga_if_present() {
        let p = r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7\sprites\144.tga";
        if !std::path::Path::new(p).exists() {
            return;
        }
        let img = load_tga_path(p).expect("tga");
        assert!(img.width > 0 && img.height > 0);
        assert_eq!(img.pixels.len(), (img.width * img.height * 4) as usize);
    }
}
