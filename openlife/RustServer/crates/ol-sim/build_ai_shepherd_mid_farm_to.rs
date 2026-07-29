//! Fix farm_to_shepherd exhaustiveness after FarmAction::DeferSheepHerding lands.
use std::path::Path;

pub fn patch(src: &Path) -> bool {
    let path = src.join("shepherd_profession.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    let old = r#"fn farm_to_shepherd(a: FarmAction) -> ShepherdAction {
    match a {
        FarmAction::None => ShepherdAction::None,
        FarmAction::Abort => ShepherdAction::Abort,
        FarmAction::ShortCraft { actor, target } => ShepherdAction::ShortCraft { actor, target },
        FarmAction::CraftItem { object_id } => ShepherdAction::CraftItem { object_id },
    }
}"#;
    let new = r#"fn farm_to_shepherd(a: FarmAction) -> ShepherdAction {
    match a {
        FarmAction::None | FarmAction::DeferSheepHerding { .. } => ShepherdAction::None,
        FarmAction::Abort => ShepherdAction::Abort,
        FarmAction::ShortCraft { actor, target } => ShepherdAction::ShortCraft { actor, target },
        FarmAction::CraftItem { object_id } => ShepherdAction::CraftItem { object_id },
    }
}"#;
    if t.contains("FarmAction::DeferSheepHerding { .. } => ShepherdAction::None") {
        return true;
    }
    if !t.contains(old) {
        // already has other arms or different formatting
        if t.contains("FarmAction::DeferSheepHerding") && t.contains("fn farm_to_shepherd") {
            return true;
        }
        // try looser: None arm only
        if let Some(i) = t.find("fn farm_to_shepherd(a: FarmAction)") {
            if let Some(rel) = t[i..].find("FarmAction::None => ShepherdAction::None,") {
                let at = i + rel;
                t.replace_range(
                    at..at + "FarmAction::None => ShepherdAction::None,".len(),
                    "FarmAction::None | FarmAction::DeferSheepHerding { .. } => ShepherdAction::None,",
                );
                return std::fs::write(&path, t).is_ok();
            }
        }
        return false;
    }
    t = t.replacen(old, new, 1);
    std::fs::write(&path, t).is_ok()
}
