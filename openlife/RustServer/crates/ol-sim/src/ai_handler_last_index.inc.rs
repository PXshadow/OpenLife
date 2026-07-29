/// Haxe `lastIndexOf(sep, maxLen)` — last occurrence of `sep` starting at index ≤ max_len.
fn last_index_of_before(s: &str, sep: &str, max_len: usize) -> Option<usize> {
    if sep.is_empty() || s.is_empty() {
        return None;
    }
    // Highest index i where i <= max_len and s[i..] starts with sep (Haxe lastIndexOf fromIndex).
    let mut best: Option<usize> = None;
    let mut start = 0usize;
    while start <= max_len && start < s.len() {
        if let Some(rel) = s[start..].find(sep) {
            let abs = start + rel;
            if abs <= max_len {
                best = Some(abs);
                start = abs + 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    best
}
