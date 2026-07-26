//! OHOL object **description-line** tag parser (pure).
//!
//! Object `.txt` files put the human name on line 2, optionally followed by
//! `#` comments, `$N` category ids, and `+tag` / `+tag_value` tokens:
//!
//! ```text
//! Stakes# +tool
//! Lock and Key $10# removed
//! Shallow Tilled Row# groundOnly +biomeBlock4
//! Plate of Squash Chunks# +contFoodDish +containOffsetBottomY_-36
//! @ Free Lock
//! ```
//!
//! This module does not load content — callers pass the raw description string.

/// Parsed pieces of an object description line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectDescription {
    /// Display name with leading `@` stripped and trailing tags removed.
    pub display_name: String,
    /// True when the raw line starts with `@` (dummy / no-spawn style).
    pub is_dummy: bool,
    /// `$N` category ids in order of appearance.
    pub categories: Vec<i32>,
    /// Comment / free text after `#` that is **not** a `+tag` token.
    pub comment_tokens: Vec<String>,
    /// `+tag` names without the leading `+` (may include `_value` suffixes).
    pub plus_tags: Vec<String>,
}

impl ObjectDescription {
    /// True if any `+tag` equals `name` (case-sensitive, without `+`).
    pub fn has_tag(&self, name: &str) -> bool {
        self.plus_tags.iter().any(|t| t == name)
    }

    /// True if any plus-tag starts with `prefix` (e.g. `"biomeBlock"`, `"containOffset"`).
    pub fn has_tag_prefix(&self, prefix: &str) -> bool {
        self.plus_tags.iter().any(|t| t.starts_with(prefix))
    }

    /// First integer suffix after `prefix` + optional `_` (e.g. `biomeBlock` → 4 from `+biomeBlock4`).
    pub fn tag_int_suffix(&self, prefix: &str) -> Option<i32> {
        for t in &self.plus_tags {
            if let Some(rest) = t.strip_prefix(prefix) {
                let rest = rest.strip_prefix('_').unwrap_or(rest);
                if rest.is_empty() {
                    continue;
                }
                // trailing numeric, possibly after more letters was already stripped by prefix
                if let Ok(v) = rest.parse::<i32>() {
                    return Some(v);
                }
                // e.g. containOffsetBottomY_-36 → try last `_` segment
                if let Some(last) = rest.rsplit('_').next() {
                    if let Ok(v) = last.parse::<i32>() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }
}

/// Parse a raw object description line (line 2 of an object file, or ContentDb name+desc).
pub fn parse_object_description(raw: &str) -> ObjectDescription {
    let raw = raw.trim();
    let is_dummy = raw.starts_with('@');
    let body = if is_dummy {
        raw.trim_start_matches('@').trim_start()
    } else {
        raw
    };

    // Split display vs rest at first `#` or bare `$N` / trailing `$N` on name.
    // OHOL uses: `Name $10# comment +tags` or `Name# +tags` or `Name# comment +tags`.
    let (name_part, after_hash) = match body.find('#') {
        Some(i) => (&body[..i], Some(body[i + 1..].trim())),
        None => (body, None),
    };

    let mut categories = Vec::new();
    let display_name = strip_categories(name_part.trim(), &mut categories);

    let mut comment_tokens = Vec::new();
    let mut plus_tags = Vec::new();

    if let Some(rest) = after_hash {
        for tok in rest.split_whitespace() {
            if tok.is_empty() {
                continue;
            }
            if let Some(tag) = tok.strip_prefix('+') {
                if !tag.is_empty() {
                    plus_tags.push(tag.to_string());
                }
            } else if let Some(cat) = tok.strip_prefix('$') {
                if let Ok(v) = cat.parse::<i32>() {
                    categories.push(v);
                } else {
                    comment_tokens.push(tok.to_string());
                }
            } else {
                comment_tokens.push(tok.to_string());
            }
        }
    }

    // Also scan name_part for embedded +tags (rare) and leftover $N.
    // Categories already stripped from display_name via strip_categories.

    ObjectDescription {
        display_name,
        is_dummy,
        categories,
        comment_tokens,
        plus_tags,
    }
}

/// Remove `$N` tokens from a name segment; push parsed categories.
fn strip_categories(name: &str, categories: &mut Vec<i32>) -> String {
    let mut out = Vec::new();
    for tok in name.split_whitespace() {
        if let Some(cat) = tok.strip_prefix('$') {
            if let Ok(v) = cat.parse::<i32>() {
                categories.push(v);
                continue;
            }
        }
        out.push(tok);
    }
    out.join(" ")
}

/// Extract only `+tag` names from a description (convenience).
pub fn plus_tags_only(raw: &str) -> Vec<String> {
    parse_object_description(raw).plus_tags
}

/// Short wire / log summary: `NAME tags=+a,+b cat=$10 dummy=0`.
pub fn format_object_tags_summary(raw: &str) -> String {
    let d = parse_object_description(raw);
    let tags = if d.plus_tags.is_empty() {
        "-".to_string()
    } else {
        d.plus_tags
            .iter()
            .map(|t| format!("+{t}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let cats = if d.categories.is_empty() {
        "-".to_string()
    } else {
        d.categories
            .iter()
            .map(|c| format!("${c}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "OBJTAGS name={} tags={} cat={} dummy={}",
        d.display_name,
        tags,
        cats,
        if d.is_dummy { 1 } else { 0 }
    )
}

/// `SAY ?TAGS` body for a held object id + raw description line.
///
/// Empty hands → `TAGS 0`. Unknown/empty description → `TAGS {id}`.
/// Otherwise: `TAGS {id} name=… tags=… cat=… dummy=…`.
pub fn format_held_tags_query(held_id: i32, description: Option<&str>) -> String {
    if held_id == 0 {
        return "TAGS 0".into();
    }
    match description {
        Some(raw) if !raw.trim().is_empty() => {
            let summary = format_object_tags_summary(raw);
            // Strip leading `OBJTAGS ` so chat reads `TAGS id name=…`.
            let body = summary
                .strip_prefix("OBJTAGS ")
                .unwrap_or(summary.as_str());
            format!("TAGS {held_id} {body}")
        }
        _ => format!("TAGS {held_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_name() {
        let d = parse_object_description("White Pine Tree with Needles");
        assert_eq!(d.display_name, "White Pine Tree with Needles");
        assert!(!d.is_dummy);
        assert!(d.plus_tags.is_empty());
        assert!(d.categories.is_empty());
    }

    #[test]
    fn tool_tag_after_hash() {
        let d = parse_object_description("Stakes# +tool");
        assert_eq!(d.display_name, "Stakes");
        assert!(d.has_tag("tool"));
        assert_eq!(d.plus_tags, vec!["tool".to_string()]);
    }

    #[test]
    fn category_and_comment() {
        let d = parse_object_description("Lock and Key $10# removed");
        assert_eq!(d.display_name, "Lock and Key");
        assert_eq!(d.categories, vec![10]);
        assert_eq!(d.comment_tokens, vec!["removed".to_string()]);
    }

    #[test]
    fn biome_block_suffix() {
        let d = parse_object_description("Shallow Tilled Row# groundOnly +biomeBlock4");
        assert_eq!(d.display_name, "Shallow Tilled Row");
        assert_eq!(d.comment_tokens, vec!["groundOnly".to_string()]);
        assert!(d.has_tag_prefix("biomeBlock"));
        assert_eq!(d.tag_int_suffix("biomeBlock"), Some(4));
    }

    #[test]
    fn cont_food_and_offset() {
        let d = parse_object_description(
            "Plate of Squash Chunks# +contFoodDish +containOffsetBottomY_-36",
        );
        assert!(d.has_tag("contFoodDish"));
        assert_eq!(d.tag_int_suffix("containOffsetBottomY"), Some(-36));
    }

    #[test]
    fn dummy_at_prefix() {
        let d = parse_object_description("@ Free Lock");
        assert!(d.is_dummy);
        assert_eq!(d.display_name, "Free Lock");
    }

    #[test]
    fn plus_tags_only_helper() {
        assert_eq!(
            plus_tags_only("Needle and Ball of Thread# +toolSewing"),
            vec!["toolSewing".to_string()]
        );
    }

    #[test]
    fn format_summary_shape() {
        let s = format_object_tags_summary("Stakes# +tool");
        assert!(s.contains("name=Stakes"));
        assert!(s.contains("tags=+tool"));
        assert!(s.contains("dummy=0"));
    }

    #[test]
    fn format_held_tags_query_shapes() {
        assert_eq!(format_held_tags_query(0, None), "TAGS 0");
        assert_eq!(format_held_tags_query(99, None), "TAGS 99");
        assert_eq!(format_held_tags_query(99, Some("")), "TAGS 99");
        let s = format_held_tags_query(10, Some("Stakes# +tool"));
        assert!(s.starts_with("TAGS 10 "), "got {s}");
        assert!(s.contains("name=Stakes"));
        assert!(s.contains("tags=+tool"));
        assert!(!s.contains("OBJTAGS"));
        let s2 = format_held_tags_query(7, Some("Lock and Key $10# removed"));
        assert!(s2.contains("cat=$10"));
        assert!(s2.contains("name=Lock and Key"));
    }
}
