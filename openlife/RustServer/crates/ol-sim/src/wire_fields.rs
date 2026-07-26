//! Pure string wire-field parsers for OHOL-style ASCII protocol.
//!
//! Complements `ol_protocol::parse_message` with small helpers for coordinates,
//! integer lists, `#`-terminated frames, and `key=value` object-file lines.
//! No I/O and no world state.

/// Parse `x y` (whitespace-separated) into integers.
pub fn parse_xy(s: &str) -> Option<(i32, i32)> {
    let mut it = s.split_whitespace().filter(|t| !t.is_empty());
    let x = it.next()?.parse().ok()?;
    let y = it.next()?.parse().ok()?;
    Some((x, y))
}

/// Parse `x y` and require no trailing non-empty tokens.
pub fn parse_xy_exact(s: &str) -> Option<(i32, i32)> {
    let mut it = s.split_whitespace().filter(|t| !t.is_empty());
    let x = it.next()?.parse().ok()?;
    let y = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some((x, y))
}

/// Parse `x y z` triple.
pub fn parse_xyz(s: &str) -> Option<(i32, i32, i32)> {
    let mut it = s.split_whitespace().filter(|t| !t.is_empty());
    let x = it.next()?.parse().ok()?;
    let y = it.next()?.parse().ok()?;
    let z = it.next()?.parse().ok()?;
    Some((x, y, z))
}

/// Split on whitespace into owned tokens (empty input → empty vec).
pub fn split_tokens(s: &str) -> Vec<&str> {
    s.split_whitespace().filter(|t| !t.is_empty()).collect()
}

/// Parse a whitespace-separated list of integers; skips bad tokens if `strict` is false.
pub fn parse_i32_list(s: &str, strict: bool) -> Option<Vec<i32>> {
    let mut out = Vec::new();
    for tok in split_tokens(s) {
        match tok.parse::<i32>() {
            Ok(v) => out.push(v),
            Err(_) if strict => return None,
            Err(_) => {}
        }
    }
    Some(out)
}

/// Parse comma-separated integers (`"1,2,3"`); empty segments skipped.
pub fn parse_csv_i32(s: &str) -> Vec<i32> {
    s.split(',')
        .filter_map(|t| {
            let t = t.trim();
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        })
        .collect()
}

/// Extract complete `#`-terminated frames from a UTF-8 string buffer.
///
/// Returns `(frames_without_hash, remainder_without_leading_consumed)`.
/// Trailing incomplete data is returned as remainder.
pub fn extract_hash_frames(buf: &str) -> (Vec<String>, String) {
    let mut frames = Vec::new();
    let mut start = 0;
    let bytes = buf.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'#' {
            let frame = buf[start..i].trim().to_string();
            if !frame.is_empty() {
                frames.push(frame);
            }
            start = i + 1;
        }
    }
    let rem = buf[start..].to_string();
    (frames, rem)
}

/// Parse `key=value` (first `=` only). Trims key and value.
pub fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (k, v) = line.split_once('=')?;
    let k = k.trim();
    let v = v.trim();
    if k.is_empty() {
        return None;
    }
    Some((k, v))
}

/// Parse `key=value` where value is `i32`.
pub fn parse_key_i32(line: &str) -> Option<(&str, i32)> {
    let (k, v) = parse_key_value(line)?;
    // value may be `1.000000` or `1,flag=0` — take leading integer token
    let head = v
        .split(|c: char| c == ',' || c == '#' || c.is_whitespace())
        .next()
        .unwrap_or(v);
    let n = if head.contains('.') {
        head.parse::<f64>().ok()? as i32
    } else {
        head.parse().ok()?
    };
    Some((k, n))
}

/// Parse `key=value` where value is `f32`.
pub fn parse_key_f32(line: &str) -> Option<(&str, f32)> {
    let (k, v) = parse_key_value(line)?;
    let head = v
        .split(|c: char| c == ',' || c == '#' || c.is_whitespace())
        .next()
        .unwrap_or(v);
    Some((k, head.parse().ok()?))
}

/// Strip a trailing `#` comment from a wire/object line (`a=b#c` → `a=b`).
pub fn strip_line_comment(s: &str) -> &str {
    match s.find('#') {
        Some(i) => s[..i].trim_end(),
        None => s.trim_end(),
    }
}

/// Format `x y` for MOVE/USE-style payloads.
pub fn format_xy(x: i32, y: i32) -> String {
    format!("{x} {y}")
}

/// Join integers with commas (map-object style flat list).
pub fn format_csv_i32(ids: &[i32]) -> String {
    ids.iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xy_ok() {
        assert_eq!(parse_xy("10  -3"), Some((10, -3)));
        assert_eq!(parse_xy_exact("1 2"), Some((1, 2)));
        assert_eq!(parse_xy_exact("1 2 3"), None);
        assert_eq!(parse_xy(""), None);
    }

    #[test]
    fn xyz_ok() {
        assert_eq!(parse_xyz("1 2 3"), Some((1, 2, 3)));
    }

    #[test]
    fn i32_list_strict() {
        assert_eq!(parse_i32_list("1 2 3", true), Some(vec![1, 2, 3]));
        assert_eq!(parse_i32_list("1 x 3", true), None);
        assert_eq!(parse_i32_list("1 x 3", false), Some(vec![1, 3]));
    }

    #[test]
    fn csv_roundtrip() {
        assert_eq!(parse_csv_i32("391,33,40"), vec![391, 33, 40]);
        assert_eq!(format_csv_i32(&[391, 33, 40]), "391,33,40");
        assert!(parse_csv_i32("").is_empty());
    }

    #[test]
    fn hash_frames() {
        let (frames, rem) = extract_hash_frames("KA#MOVE 1 2# partial");
        assert_eq!(frames, vec!["KA".to_string(), "MOVE 1 2".to_string()]);
        assert_eq!(rem, " partial");
    }

    #[test]
    fn key_value_object_lines() {
        assert_eq!(
            parse_key_value("foodValue=3"),
            Some(("foodValue", "3"))
        );
        assert_eq!(parse_key_i32("foodValue=3"), Some(("foodValue", 3)));
        assert_eq!(parse_key_i32("mapChance=1.000000#biomes_0,3"), Some(("mapChance", 1)));
        assert_eq!(parse_key_f32("speedMult=1.500000"), Some(("speedMult", 1.5)));
        assert_eq!(
            parse_key_i32("permanent=1,minPickupAge=3"),
            Some(("permanent", 1))
        );
    }

    #[test]
    fn strip_comment() {
        assert_eq!(strip_line_comment("numSlots=0#timeStretch=1"), "numSlots=0");
        assert_eq!(strip_line_comment("plain"), "plain");
    }

    #[test]
    fn format_xy_shape() {
        assert_eq!(format_xy(-1, 5), "-1 5");
    }
}
