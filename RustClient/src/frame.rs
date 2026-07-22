//! `#`-terminated ASCII framing used by the OHOL wire protocol.
//!
//! MAP_CHUNK (`MC`) and COMPRESSED_MESSAGE (`CM`) place raw binary **after** the `#`,
//! so a naive split-on-`#` desyncs. [`WireReader`] skips those payloads using the
//! declared compressed size.

use std::io::{self, Read, Write};

/// Append `#` if not already present and return owned message bytes (no newline).
pub fn encode_raw(message_without_hash: &str) -> String {
    if message_without_hash.ends_with('#') {
        message_without_hash.to_string()
    } else {
        format!("{message_without_hash}#")
    }
}

/// Incremental reader that yields complete `#`-terminated **text** messages
/// (body without `#`), and consumes trailing binary for `MC` / `CM`.
#[derive(Debug, Default)]
pub struct FrameReader {
    buf: Vec<u8>,
    /// Bytes of binary payload still to discard after an MC/CM header frame.
    binary_remaining: usize,
}

impl FrameReader {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            binary_remaining: 0,
        }
    }

    /// Push raw socket bytes; return complete text message bodies (no `#`).
    pub fn push(&mut self, data: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        loop {
            if self.binary_remaining > 0 {
                let take = self.binary_remaining.min(self.buf.len());
                if take == 0 {
                    break;
                }
                self.buf.drain(..take);
                self.binary_remaining -= take;
                continue;
            }

            let Some(pos) = self.buf.iter().position(|&b| b == b'#') else {
                break;
            };
            let msg_bytes: Vec<u8> = self.buf.drain(..=pos).collect();
            let body = &msg_bytes[..msg_bytes.len() - 1];
            let s = String::from_utf8_lossy(body).into_owned();

            if let Some(n) = binary_payload_size(&s) {
                self.binary_remaining = n;
            }
            out.push(s);
        }
        out
    }

    pub fn buffered_len(&self) -> usize {
        self.buf.len()
    }

    pub fn binary_remaining(&self) -> usize {
        self.binary_remaining
    }
}

/// For `MC` / `CM` header bodies, return compressed byte count that follows `#`.
///
/// MC:
/// ```text
/// MC
/// sizeX sizeY x y
/// binary_raw_size binary_compressed_size
/// ```
/// CM:
/// ```text
/// CM
/// binary_raw_size binary_compressed_size
/// ```
pub fn binary_payload_size(body: &str) -> Option<usize> {
    let mut lines = body.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
    let tag = lines.next()?;
    match tag {
        "MC" => {
            let _dims = lines.next()?; // sizeX sizeY x y
            let sizes = lines.next()?;
            parse_compressed_size(sizes)
        }
        "CM" => {
            let sizes = lines.next()?;
            parse_compressed_size(sizes)
        }
        _ => None,
    }
}

fn parse_compressed_size(sizes: &str) -> Option<usize> {
    let mut it = sizes.split_whitespace();
    let _raw: usize = it.next()?.parse().ok()?;
    let compressed: usize = it.next()?.parse().ok()?;
    Some(compressed)
}

/// Write a full `#`-terminated message (adds `#` if missing).
pub fn write_message<W: Write>(w: &mut W, message: &str) -> io::Result<()> {
    let framed = encode_raw(message);
    w.write_all(framed.as_bytes())?;
    w.flush()
}

/// Read until `#`, returning the message body without `#`. Blocks.
/// Does not handle MC binary on its own beyond FrameReader logic when enough
/// data is pushed in subsequent reads via a session loop.
pub fn read_message<R: Read>(r: &mut R) -> io::Result<String> {
    let mut reader = FrameReader::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = r.read(&mut chunk)?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "connection closed before complete # frame",
            ));
        }
        let msgs = reader.push(&chunk[..n]);
        if let Some(m) = msgs.into_iter().next() {
            // leftover messages discarded here — prefer ClientSession for multi-frame
            return Ok(m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_raw_adds_hash() {
        assert_eq!(encode_raw("KA 0 0"), "KA 0 0#");
        assert_eq!(encode_raw("KA 0 0#"), "KA 0 0#");
    }

    #[test]
    fn frame_reader_splits_multiple() {
        let mut fr = FrameReader::new();
        let msgs = fr.push(b"SN\n1/20\nch\n184#ACCEPTED#partial");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0], "SN\n1/20\nch\n184");
        assert_eq!(msgs[1], "ACCEPTED");
        assert_eq!(fr.buffered_len(), "partial".len());
        let more = fr.push(b"#");
        assert_eq!(more, vec!["partial".to_string()]);
    }

    #[test]
    fn mc_skips_binary_payload() {
        // MC header then 5 bytes binary (may contain #) then PU frame
        let mut fr = FrameReader::new();
        let header = b"MC\n32 30 0 0\n10 5\n#";
        let binary = b"ab#cd"; // 5 bytes with embedded #
        let after = b"PU\n1 2 3\n#";
        let mut stream = Vec::new();
        stream.extend_from_slice(header);
        stream.extend_from_slice(binary);
        stream.extend_from_slice(after);

        let msgs = fr.push(&stream);
        assert_eq!(msgs.len(), 2);
        assert!(msgs[0].starts_with("MC\n"));
        assert!(msgs[1].starts_with("PU\n"));
        assert_eq!(fr.binary_remaining(), 0);
        assert_eq!(fr.buffered_len(), 0);
    }

    #[test]
    fn binary_payload_size_mc() {
        let body = "MC\n32 30 472 473\n6544 608\n";
        assert_eq!(binary_payload_size(body), Some(608));
    }
}
