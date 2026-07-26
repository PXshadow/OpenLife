//! LOOK relative tile query (Haxe look-at subset).

/// Format look reply without leading p_id.
/// `LOOK dx dy biome floor obj`
pub fn format_look(
    dx: i32,
    dy: i32,
    biome: u8,
    floor: u16,
    object_id: i32,
    object_name: &str,
) -> String {
    if object_id == 0 {
        format!("LOOK {dx} {dy} biome={biome} floor={floor} obj=0")
    } else if object_name.is_empty() {
        format!("LOOK {dx} {dy} biome={biome} floor={floor} obj={object_id}")
    } else {
        format!("LOOK {dx} {dy} biome={biome} floor={floor} obj={object_id} {object_name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_named() {
        assert!(format_look(1, 0, 2, 0, 0, "").contains("obj=0"));
        let s = format_look(0, 1, 3, 1, 33, "Gooseberry");
        assert!(s.contains("Gooseberry"));
        assert!(s.contains("obj=33"));
    }
}
