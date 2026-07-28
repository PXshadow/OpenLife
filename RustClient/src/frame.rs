//! `#`-terminated ASCII framing used by the OHOL wire protocol.
//!
//! MAP_CHUNK (`MC`) and COMPRESSED_MESSAGE (`CM`) place raw binary **after** the `#`,
//! so a naive split-on-`#` desyncs. [`FrameReader`] collects MC zlib for L-MAP decode,
//! and **inflates CM** zlib into the inner text message.
//!
//! C++: `LivingLifePage.cpp` `getNextServerMessageRaw` pendingCMData path.
//! Haxe: `Client.hx` `compressInput` / `compressProcess`.

use std::io::{self, Read, Write};

use flate2::read::ZlibDecoder;

/// Append `#` if not already present and return owned message bytes (no newline).
pub fn encode_raw(message_without_hash: &str) -> String {
    if message_without_hash.ends_with('#') {
        message_without_hash.to_string()
    } else {
        format!("{message_without_hash}#")
    }
}

/// One complete frame from the socket (text body without `#`, or MC with binary).
#[derive(Debug, Clone)]
pub enum FramedMessage {
    /// Normal ASCII message body (or CM-inflated inner body).
    Text(String),
    /// MAP_CHUNK header text + compressed zlib payload (decode in [`crate::client_map`]).
    MapChunk {
        header: String,
        compressed: Vec<u8>,
    },
}

impl FramedMessage {
    /// Text body for dispatch when not MC binary, or MC header only.
    pub fn as_dispatch_text(&self) -> &str {
        match self {
            Self::Text(s) => s,
            Self::MapChunk { header, .. } => header,
        }
    }
}

/// Kind of binary payload following a `#` frame.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BinaryMode {
    /// Collect MC zlib for L-MAP.
    CollectMc { header: String },
    /// Collect + zlib inflate, yield decompressed text (CM).
    InflateCm { raw_size: usize },
}

/// Incremental reader that yields complete framed messages.
#[derive(Debug, Default)]
pub struct FrameReader {
    buf: Vec<u8>,
    binary_remaining: usize,
    binary_mode: Option<BinaryMode>,
    binary_acc: Vec<u8>,
}

impl FrameReader {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            binary_remaining: 0,
            binary_mode: None,
            binary_acc: Vec::new(),
        }
    }

    /// Push raw socket bytes; return complete frames.
    pub fn push_framed(&mut self, data: &[u8]) -> Vec<FramedMessage> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        loop {
            if self.binary_remaining > 0 {
                let take = self.binary_remaining.min(self.buf.len());
                if take == 0 {
                    break;
                }
                let chunk: Vec<u8> = self.buf.drain(..take).collect();
                self.binary_remaining -= take;
                self.binary_acc.extend_from_slice(&chunk);
                if self.binary_remaining == 0 {
                    match self.binary_mode.take() {
                        Some(BinaryMode::CollectMc { header }) => {
                            out.push(FramedMessage::MapChunk {
                                header,
                                compressed: std::mem::take(&mut self.binary_acc),
                            });
                        }
                        Some(BinaryMode::InflateCm { raw_size }) => {
                            match inflate_cm(&self.binary_acc, raw_size) {
                                Ok(text) => out.push(FramedMessage::Text(text)),
                                Err(_) => {
                                    out.push(FramedMessage::Text(format!(
                                        "CM\n{} {}\n",
                                        raw_size,
                                        self.binary_acc.len()
                                    )));
                                }
                            }
                            self.binary_acc.clear();
                        }
                        None => {
                            self.binary_acc.clear();
                        }
                    }
                }
                continue;
            }

            let Some(pos) = self.buf.iter().position(|&b| b == b'#') else {
                break;
            };
            let msg_bytes: Vec<u8> = self.buf.drain(..=pos).collect();
            let body = &msg_bytes[..msg_bytes.len() - 1];
            let s = String::from_utf8_lossy(body).into_owned();

            if let Some(spec) = binary_payload_spec(&s) {
                self.binary_acc.clear();
                if spec.compressed_size == 0 {
                    // Empty binary (fixture / empty chunk) — emit immediately.
                    match spec.mode {
                        BinaryMode::CollectMc { header } => {
                            out.push(FramedMessage::MapChunk {
                                header,
                                compressed: Vec::new(),
                            });
                        }
                        BinaryMode::InflateCm { raw_size } => {
                            out.push(FramedMessage::Text(format!("CM\n{raw_size} 0\n")));
                        }
                    }
                } else {
                    self.binary_remaining = spec.compressed_size;
                    self.binary_mode = Some(spec.mode);
                }
            } else {
                out.push(FramedMessage::Text(s));
            }
        }
        out
    }

    /// Backward-compatible: text bodies only; MC becomes header string (binary dropped
    /// for callers that only use [`Self::push`] — prefer [`Self::push_framed`]).
    pub fn push(&mut self, data: &[u8]) -> Vec<String> {
        self.push_framed(data)
            .into_iter()
            .map(|f| match f {
                FramedMessage::Text(s) => s,
                FramedMessage::MapChunk { header, .. } => header,
            })
            .collect()
    }

    pub fn buffered_len(&self) -> usize {
        self.buf.len()
    }

    pub fn binary_remaining(&self) -> usize {
        self.binary_remaining
    }
}

struct BinarySpec {
    compressed_size: usize,
    mode: BinaryMode,
}

/// For `MC` / `CM` header bodies, describe trailing binary after `#`.
fn binary_payload_spec(body: &str) -> Option<BinarySpec> {
    let mut lines = body.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
    let tag = lines.next()?;
    match tag {
        "MC" => {
            let _dims = lines.next()?; // sizeX sizeY x y
            let sizes = lines.next()?;
            let (raw, compressed) = parse_size_pair(sizes)?;
            let _ = raw;
            Some(BinarySpec {
                compressed_size: compressed,
                mode: BinaryMode::CollectMc {
                    header: body.to_string(),
                },
            })
        }
        "CM" => {
            let sizes = lines.next()?;
            let (raw, compressed) = parse_size_pair(sizes)?;
            Some(BinarySpec {
                compressed_size: compressed,
                mode: BinaryMode::InflateCm { raw_size: raw },
            })
        }
        _ => None,
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
    binary_payload_spec(body).map(|s| s.compressed_size)
}

fn parse_size_pair(sizes: &str) -> Option<(usize, usize)> {
    let mut it = sizes.split_whitespace();
    let raw: usize = it.next()?.parse().ok()?;
    let compressed: usize = it.next()?.parse().ok()?;
    Some((raw, compressed))
}

/// zlib-inflate CM payload. C++: `zipDecompress` (miniz zlib).
///
/// Decompressed bytes are the inner message body **without** trailing `#`.
pub fn inflate_cm(compressed: &[u8], expected_raw: usize) -> io::Result<String> {
    let mut dec = ZlibDecoder::new(compressed);
    let mut out = Vec::with_capacity(expected_raw.max(64));
    dec.read_to_end(&mut out)?;
    // Tolerate slight size mismatch (some servers pad); trim at first NUL if any.
    if let Some(nul) = out.iter().position(|&b| b == 0) {
        out.truncate(nul);
    }
    String::from_utf8(out).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Compress a message body with zlib (for unit tests / fixture peers).
pub fn compress_cm_payload(raw: &[u8]) -> io::Result<Vec<u8>> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(raw)?;
    enc.finish()
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
    fn mc_collects_binary_payload() {
        // MC header then 5 bytes binary (may contain #) then PU frame
        let mut fr = FrameReader::new();
        let header = b"MC\n32 30 0 0\n10 5\n#";
        let binary = b"ab#cd"; // 5 bytes with embedded #
        let after = b"PU\n1 2 3\n#";
        let mut stream = Vec::new();
        stream.extend_from_slice(header);
        stream.extend_from_slice(binary);
        stream.extend_from_slice(after);

        let msgs = fr.push_framed(&stream);
        assert_eq!(msgs.len(), 2);
        match &msgs[0] {
            FramedMessage::MapChunk {
                header,
                compressed,
            } => {
                assert!(header.starts_with("MC\n"));
                assert_eq!(compressed, b"ab#cd");
            }
            _ => panic!("expected MapChunk"),
        }
        match &msgs[1] {
            FramedMessage::Text(s) => assert!(s.starts_with("PU\n")),
            _ => panic!("expected PU text"),
        }
        assert_eq!(fr.binary_remaining(), 0);
        assert_eq!(fr.buffered_len(), 0);
    }

    #[test]
    fn binary_payload_size_mc() {
        let body = "MC\n32 30 472 473\n6544 608\n";
        assert_eq!(binary_payload_size(body), Some(608));
    }

    #[test]
    fn cm_inflates_to_inner_message() {
        // C++: CM header + zlib(body) → yield body only
        let inner = b"PU\n7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let compressed = compress_cm_payload(inner).unwrap();
        let header = format!("CM\n{} {}\n#", inner.len(), compressed.len());
        let mut stream = Vec::new();
        stream.extend_from_slice(header.as_bytes());
        stream.extend_from_slice(&compressed);
        stream.extend_from_slice(b"FM\n#");

        let mut fr = FrameReader::new();
        let msgs = fr.push(&stream);
        assert_eq!(msgs.len(), 2);
        assert!(msgs[0].starts_with("PU\n"), "got {:?}", msgs[0]);
        assert_eq!(msgs[1], "FM\n");
        assert_eq!(fr.binary_remaining(), 0);
    }

    #[test]
    fn binary_payload_size_cm() {
        assert_eq!(binary_payload_size("CM\n100 40\n"), Some(40));
    }
}
