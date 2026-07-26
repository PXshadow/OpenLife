//! Skill / recipe familiarity beyond tool slots (Haxe skill XP subset).

use std::collections::HashMap;

/// XP awarded per successful craft/use of a recipe key.
pub const XP_PER_CRAFT: u32 = 1;

/// Level thresholds: level = max n where thresholds[n] <= xp.
pub const LEVEL_THRESHOLDS: &[u32] = &[0, 3, 8, 15, 25, 40, 60, 90];

/// One skill track keyed by a stable recipe/object id.
#[derive(Debug, Clone, Default)]
pub struct SkillTrack {
    pub xp: u32,
}

impl SkillTrack {
    pub fn level(&self) -> u32 {
        let mut lvl = 0u32;
        for (i, &th) in LEVEL_THRESHOLDS.iter().enumerate() {
            if self.xp >= th {
                lvl = i as u32;
            }
        }
        lvl
    }

    pub fn add_xp(&mut self, amount: u32) {
        self.xp = self.xp.saturating_add(amount);
    }
}

/// Per-player skill book: skill_key → track.
#[derive(Debug, Default, Clone)]
pub struct SkillBook {
    pub tracks: HashMap<i32, SkillTrack>,
}

impl SkillBook {
    pub fn gain(&mut self, skill_key: i32, amount: u32) -> u32 {
        let t = self.tracks.entry(skill_key).or_default();
        t.add_xp(amount);
        t.level()
    }

    pub fn level_of(&self, skill_key: i32) -> u32 {
        self.tracks.get(&skill_key).map(|t| t.level()).unwrap_or(0)
    }

    pub fn xp_of(&self, skill_key: i32) -> u32 {
        self.tracks.get(&skill_key).map(|t| t.xp).unwrap_or(0)
    }

    /// `SKILLS n key:lvl:xp ...` (top tracks by xp).
    pub fn format_query(&self, max: usize) -> String {
        if self.tracks.is_empty() {
            return "SKILLS 0".into();
        }
        let mut rows: Vec<(i32, u32, u32)> = self
            .tracks
            .iter()
            .map(|(&k, t)| (k, t.level(), t.xp))
            .collect();
        rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        let n = rows.len().min(max.max(1));
        let parts: Vec<String> = rows
            .into_iter()
            .take(n)
            .map(|(k, lvl, xp)| format!("{k}:{lvl}:{xp}"))
            .collect();
        format!("SKILLS {} {}", n, parts.join(" "))
    }
}

/// Session: p_id → SkillBook.
#[derive(Debug, Default, Clone)]
pub struct SkillState {
    pub by_player: HashMap<i32, SkillBook>,
}

impl SkillState {
    pub fn book_mut(&mut self, p_id: i32) -> &mut SkillBook {
        self.by_player.entry(p_id).or_default()
    }

    pub fn on_craft(&mut self, p_id: i32, recipe_key: i32) -> u32 {
        self.book_mut(p_id).gain(recipe_key, XP_PER_CRAFT)
    }

    pub fn format_query(&self, p_id: i32) -> String {
        self.by_player
            .get(&p_id)
            .map(|b| b.format_query(8))
            .unwrap_or_else(|| "SKILLS 0".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_from_xp() {
        let mut t = SkillTrack::default();
        assert_eq!(t.level(), 0);
        t.add_xp(3);
        assert_eq!(t.level(), 1);
        t.add_xp(5);
        assert_eq!(t.level(), 2);
    }

    #[test]
    fn craft_gains() {
        let mut s = SkillState::default();
        let lvl = s.on_craft(1, 242);
        assert_eq!(lvl, 0);
        for _ in 0..5 {
            s.on_craft(1, 242);
        }
        assert!(s.book_mut(1).level_of(242) >= 1);
        let q = s.format_query(1);
        assert!(q.starts_with("SKILLS "));
        assert!(q.contains("242:"));
    }
}
