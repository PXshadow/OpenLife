//! Minimal wild-animal stub (Haxe animal AI subset — pure data + wander).

use rand::Rng;
use serde::Serialize;
use std::sync::{Arc, RwLock};

/// Shared animal counts for web (`/api/animals`).
pub type AnimalView = Arc<RwLock<AnimalSnapshot>>;

/// Live animal world mirror for self-play AI (sim publishes; agents read only).
///
/// Same Arc-share pattern as reverse craft graph: built/owned by sim, cloned to
/// readers via short `RwLock` scopes (never mutates from AI / net).
pub type AnimalWorldShare = Arc<RwLock<AnimalWorld>>;

/// Count-only animal snapshot (cheap; no per-entity list).
#[derive(Debug, Clone, Serialize, Default)]
pub struct AnimalSnapshot {
    pub total: usize,
    pub rabbit: usize,
    pub wolf: usize,
    pub boar: usize,
}

/// Species of a wild animal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimalKind {
    Rabbit,
    Wolf,
    Boar,
}

impl AnimalKind {
    /// Default hit points for a freshly spawned animal of this kind.
    pub fn default_hp(self) -> i32 {
        match self {
            Self::Rabbit => 5,
            Self::Wolf => 20,
            Self::Boar => 30,
        }
    }

    /// Lowercase kind label for chat queries.
    pub fn label(self) -> &'static str {
        match self {
            Self::Rabbit => "rabbit",
            Self::Wolf => "wolf",
            Self::Boar => "boar",
        }
    }

    /// OneLife content object id placed on the map for this kind
    /// (Haxe animals are map objects that walk via MX + old_x/old_y/speed).
    ///
    /// - Rabbit → 3566 Fleeing Rabbit
    /// - Wolf → 418 Wolf
    /// - Boar → 1323 Wild Boar
    pub fn object_id(self) -> i32 {
        match self {
            Self::Rabbit => 3566,
            Self::Wolf => 418,
            Self::Boar => 1323,
        }
    }

    /// MX walk speed (Haxe `SendAnimalMoveUpdateToAllClosePlayers` uses ~1).
    pub fn move_speed(self) -> f32 {
        match self {
            Self::Rabbit => 1.5,
            Self::Wolf => 1.0,
            Self::Boar => 0.8,
        }
    }
}

/// One live animal entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Animal {
    pub id: i32,
    pub x: i32,
    pub y: i32,
    pub kind: AnimalKind,
    pub hp: i32,
    /// Sim-seconds until next wander step (Haxe timed animal move).
    pub move_timer: f32,
}

/// Collection of animals with id allocation.
#[derive(Debug, Default, Clone)]
pub struct AnimalWorld {
    pub animals: Vec<Animal>,
    pub next_id: i32,
}

impl AnimalWorld {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fallback wander interval when content has no auto-decay move transition.
    ///
    /// Content OneLife `autoDecaySeconds` for wolf/boar is **3**; rabbits **1**.
    pub fn wander_interval(kind: AnimalKind) -> f32 {
        match kind {
            AnimalKind::Rabbit => 1.0,
            AnimalKind::Wolf => 3.0,
            AnimalKind::Boar => 3.0,
        }
    }

    /// Haxe-ish step radius: content `desiredMoveDist` or `move` with
    /// `if (moveDist < 3) moveDist += 1` (TimeHelper.doAnimalMovement).
    pub fn move_radius(kind: AnimalKind) -> i32 {
        match kind {
            // -1_3566: move=3 desired=5 → use 3–4 after Haxe adjust
            AnimalKind::Rabbit => 4,
            // -1_418 / -1_1323: move=2 → +1 → 3
            AnimalKind::Wolf => 3,
            AnimalKind::Boar => 3,
        }
    }

    /// Spawn an animal at `(x, y)` with default HP. Returns assigned id.
    pub fn spawn(&mut self, kind: AnimalKind, x: i32, y: i32) -> i32 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        // Stagger initial timers so the pack does not hop in lockstep.
        let stagger = (id.rem_euclid(5) as f32) * 0.2;
        self.animals.push(Animal {
            id,
            x,
            y,
            kind,
            hp: kind.default_hp(),
            move_timer: Self::wander_interval(kind) * 0.25 + stagger,
        });
        id
    }

    /// Advance animal move timers by `dt`; step when due (timed movement).
    ///
    /// Haxe `doAnimalMovement` subset:
    /// - Up to **20 random targets** in a Chebyshev `move_radius` box
    /// - Prefer empty tiles (first half of attempts)
    /// - Must be walkable and not the current tile
    /// - After move, re-arm timer from `interval` with ±20% jitter
    ///
    /// Returns `(animal_id, kind, old_x, old_y, new_x, new_y)` for MX fan-out.
    /// Like timed wander with Haxe pathing via `pick_dest`.
    ///
    /// `pick_dest(from_x, from_y, kind) -> Option<(nx,ny)>` encapsulates
    /// biome/object blockage (see [`crate::animal_move`]).
    pub fn tick_wander_timed_ex<R: Rng>(
        &mut self,
        rng: &mut R,
        dt: f32,
        world_w: i32,
        world_h: i32,
        interval_for: Option<&dyn Fn(AnimalKind) -> f32>,
        mut pick_dest: impl FnMut(&mut R, i32, i32, AnimalKind) -> Option<(i32, i32)>,
    ) -> Vec<(i32, AnimalKind, i32, i32, i32, i32)> {
        let mut moved = Vec::new();
        if world_w <= 0 || world_h <= 0 || dt <= 0.0 {
            return moved;
        }
        let _ = (world_w, world_h);
        for a in &mut self.animals {
            a.move_timer -= dt;
            if a.move_timer > 0.0 {
                continue;
            }
            let base_iv = interval_for
                .map(|f| f(a.kind))
                .unwrap_or_else(|| Self::wander_interval(a.kind))
                .max(0.35);
            let jitter = 0.8 + rng.gen_range(0.0..0.4);
            a.move_timer = base_iv * jitter;

            let ox = a.x;
            let oy = a.y;
            if let Some((nx, ny)) = pick_dest(rng, ox, oy, a.kind) {
                if nx != ox || ny != oy {
                    a.x = nx;
                    a.y = ny;
                    moved.push((a.id, a.kind, ox, oy, nx, ny));
                }
            }
        }
        moved
    }

    /// Legacy walkable-callback API (tests).
    pub fn tick_wander_timed_walkable<R: Rng>(
        &mut self,
        rng: &mut R,
        dt: f32,
        world_w: i32,
        world_h: i32,
        walkable: &dyn Fn(i32, i32) -> bool,
    ) -> Vec<(i32, AnimalKind, i32, i32, i32, i32)> {
        let rad_for = |k: AnimalKind| Self::move_radius(k);
        self.tick_wander_timed_ex(rng, dt, world_w, world_h, None, |rng, ox, oy, kind| {
            let rad = rad_for(kind).max(1);
            for _ in 0..20 {
                let nx = ox - rad + rng.gen_range(0..=rad * 2);
                let ny = oy - rad + rng.gen_range(0..=rad * 2);
                if (nx != ox || ny != oy)
                    && nx >= 0
                    && ny >= 0
                    && nx < world_w
                    && ny < world_h
                    && walkable(nx, ny)
                {
                    return Some((nx, ny));
                }
            }
            None
        })
    }

    /// One random cardinal step per animal when the destination is in-bounds and walkable.
    ///
    /// Legacy batch hop (used by tests). Prefer [`Self::tick_wander_timed`] in sim.
    pub fn tick_wander<R: Rng>(
        &mut self,
        rng: &mut R,
        world_w: i32,
        world_h: i32,
        walkable: &dyn Fn(i32, i32) -> bool,
    ) -> Vec<(i32, AnimalKind, i32, i32, i32, i32)> {
        self.tick_wander_timed_walkable(rng, 100.0, world_w, world_h, walkable)
    }

    /// Look up live animal by id.
    pub fn get(&self, id: i32) -> Option<&Animal> {
        self.animals.iter().find(|a| a.id == id)
    }

    /// Count animals of each kind (missing kinds omitted).
    pub fn counts_by_kind(&self) -> [(AnimalKind, usize); 3] {
        let mut rabbit = 0usize;
        let mut wolf = 0usize;
        let mut boar = 0usize;
        for a in &self.animals {
            match a.kind {
                AnimalKind::Rabbit => rabbit += 1,
                AnimalKind::Wolf => wolf += 1,
                AnimalKind::Boar => boar += 1,
            }
        }
        [
            (AnimalKind::Rabbit, rabbit),
            (AnimalKind::Wolf, wolf),
            (AnimalKind::Boar, boar),
        ]
    }

    /// Chat reply body for `SAY ?ANIMALS` (without leading player id).
    ///
    /// Example: `ANIMALS total=5 rabbit=3 wolf=2 boar=0`
    pub fn format_query(&self) -> String {
        let counts = self.counts_by_kind();
        format!(
            "ANIMALS total={} rabbit={} wolf={} boar={}",
            self.animals.len(),
            counts[0].1,
            counts[1].1,
            counts[2].1
        )
    }

    /// Count-only snapshot for web `/api/animals`.
    pub fn snapshot(&self) -> AnimalSnapshot {
        let counts = self.counts_by_kind();
        AnimalSnapshot {
            total: self.animals.len(),
            rabbit: counts[0].1,
            wolf: counts[1].1,
            boar: counts[2].1,
        }
    }

    /// True if any wolf is within Chebyshev `range` of tile `(x, y)`.
    ///
    /// AI threat sensor: use [`ANIMAL_THREAT_RANGE`] (5) for flee decisions.
    pub fn nearby_threat(&self, x: i32, y: i32, range: i32) -> bool {
        self.animals.iter().any(|a| {
            a.kind == AnimalKind::Wolf
                && (a.x - x).abs().max((a.y - y).abs()) <= range
        })
    }

    /// True if any rabbit/boar is within Chebyshev `range` of `(x, y)`.
    ///
    /// AI prey sensor for Hunter [`crate::Goal::Hunt`] (wolves are threats, not prey).
    pub fn nearby_prey(&self, x: i32, y: i32, range: i32) -> bool {
        self.animals.iter().any(|a| {
            matches!(a.kind, AnimalKind::Rabbit | AnimalKind::Boar)
                && (a.x - x).abs().max((a.y - y).abs()) <= range
        })
    }

    /// Direction `(dx, dy)` from `(x, y)` toward the nearest wolf within `range`.
    ///
    /// Used by self-play flee steering (step opposite of this vector). Ties break
    /// by first match in stable scan order.
    pub fn nearest_threat_dir(&self, x: i32, y: i32, range: i32) -> Option<(i32, i32)> {
        let mut best: Option<(i32, i32, i32)> = None; // (dist, dx, dy)
        for a in &self.animals {
            if a.kind != AnimalKind::Wolf {
                continue;
            }
            let dx = a.x - x;
            let dy = a.y - y;
            let d = dx.abs().max(dy.abs());
            if d > range {
                continue;
            }
            match best {
                None => best = Some((d, dx, dy)),
                Some((bd, _, _)) if d < bd => best = Some((d, dx, dy)),
                _ => {}
            }
        }
        best.map(|(_, dx, dy)| (dx, dy))
    }

    /// Id of the nearest animal within Chebyshev `range` of `(x, y)`, if any.
    ///
    /// Ties break by lower index (stable scan order).
    pub fn nearest_id(&self, x: i32, y: i32, range: i32) -> Option<i32> {
        let mut best: Option<(i32, i32)> = None; // (id, dist)
        for a in &self.animals {
            let d = (a.x - x).abs().max((a.y - y).abs());
            if d <= range {
                match best {
                    None => best = Some((a.id, d)),
                    Some((_, bd)) if d < bd => best = Some((a.id, d)),
                    _ => {}
                }
            }
        }
        best.map(|(id, _)| id)
    }

    /// Apply `amount` hit-point damage to animal `id`.
    ///
    /// Returns `None` if the id is unknown. On success: `(hp_left, kind, killed)`.
    /// When HP reaches ≤ 0 the animal is removed (`hp_left = 0`, `killed = true`).
    pub fn damage(&mut self, id: i32, amount: i32) -> Option<(i32, AnimalKind, bool)> {
        let idx = self.animals.iter().position(|a| a.id == id)?;
        let kind = self.animals[idx].kind;
        let hp = self.animals[idx].hp - amount;
        if hp <= 0 {
            self.animals.remove(idx);
            Some((0, kind, true))
        } else {
            self.animals[idx].hp = hp;
            Some((hp, kind, false))
        }
    }
}

/// Chebyshev range used by AI for wolf threat (Flee).
pub const ANIMAL_THREAT_RANGE: i32 = 5;

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn snapshot_counts_by_kind() {
        let mut w = AnimalWorld::new();
        w.spawn(AnimalKind::Rabbit, 0, 0);
        w.spawn(AnimalKind::Rabbit, 1, 0);
        w.spawn(AnimalKind::Wolf, 2, 0);
        let s = w.snapshot();
        assert_eq!(s.total, 3);
        assert_eq!(s.rabbit, 2);
        assert_eq!(s.wolf, 1);
        assert_eq!(s.boar, 0);
    }

    #[test]
    fn spawn_assigns_ids_and_default_hp() {
        let mut w = AnimalWorld::new();
        let r = w.spawn(AnimalKind::Rabbit, 1, 2);
        let wolf = w.spawn(AnimalKind::Wolf, 3, 4);
        let boar = w.spawn(AnimalKind::Boar, 5, 6);
        assert_eq!(r, 0);
        assert_eq!(wolf, 1);
        assert_eq!(boar, 2);
        assert_eq!(w.animals.len(), 3);
        assert_eq!(w.animals[0].hp, 5);
        assert_eq!(w.animals[1].hp, 20);
        assert_eq!(w.animals[2].hp, 30);
        assert_eq!(w.animals[0].kind, AnimalKind::Rabbit);
        assert_eq!((w.animals[0].x, w.animals[0].y), (1, 2));
    }

    #[test]
    fn tick_wander_moves_or_stays_in_bounds() {
        let mut w = AnimalWorld::new();
        w.spawn(AnimalKind::Rabbit, 5, 5);
        let mut rng = StdRng::seed_from_u64(42);
        let walkable = |_x: i32, _y: i32| true;
        for _ in 0..20 {
            w.tick_wander(&mut rng, 10, 10, &walkable);
            let a = &w.animals[0];
            assert!(a.x >= 0 && a.x < 10);
            assert!(a.y >= 0 && a.y < 10);
        }
        // With open walkable map and many ticks, should have moved at least once.
        // (seeded; if still at origin, check manhattan vs start over more steps)
        let mut moved = false;
        w.animals[0].x = 5;
        w.animals[0].y = 5;
        for _ in 0..50 {
            w.tick_wander(&mut rng, 10, 10, &walkable);
            if w.animals[0].x != 5 || w.animals[0].y != 5 {
                moved = true;
                break;
            }
        }
        assert!(moved, "expected at least one wander step on open map");
    }

    #[test]
    fn tick_wander_respects_walkable_callback() {
        let mut w = AnimalWorld::new();
        w.spawn(AnimalKind::Wolf, 2, 2);
        let mut rng = StdRng::seed_from_u64(7);
        // Block every destination → animal stays put.
        let blocked = |_x: i32, _y: i32| false;
        for _ in 0..10 {
            w.tick_wander(&mut rng, 8, 8, &blocked);
        }
        assert_eq!((w.animals[0].x, w.animals[0].y), (2, 2));
    }

    #[test]
    fn format_query_counts_kinds() {
        let mut w = AnimalWorld::new();
        w.spawn(AnimalKind::Rabbit, 0, 0);
        w.spawn(AnimalKind::Rabbit, 1, 0);
        w.spawn(AnimalKind::Wolf, 2, 0);
        let s = w.format_query();
        assert!(s.contains("total=3"), "{s}");
        assert!(s.contains("rabbit=2"), "{s}");
        assert!(s.contains("wolf=1"), "{s}");
        assert!(s.contains("boar=0"), "{s}");
    }

    #[test]
    fn nearby_threat_wolf_within_range() {
        let mut w = AnimalWorld::new();
        w.spawn(AnimalKind::Rabbit, 0, 0);
        // Wolf at Chebyshev 5 from (10,10) → (15,10)
        w.spawn(AnimalKind::Wolf, 15, 10);
        assert!(w.nearby_threat(10, 10, ANIMAL_THREAT_RANGE));
        assert!(w.nearby_threat(10, 10, 5));
        // Just outside range
        assert!(!w.nearby_threat(10, 10, 4));
        // Rabbit alone is not a threat
        let mut only_rabbit = AnimalWorld::new();
        only_rabbit.spawn(AnimalKind::Rabbit, 10, 10);
        assert!(!only_rabbit.nearby_threat(10, 10, 5));
    }

    #[test]
    fn nearby_prey_rabbit_boar_not_wolf() {
        let mut w = AnimalWorld::new();
        w.spawn(AnimalKind::Wolf, 10, 10);
        assert!(!w.nearby_prey(10, 10, ANIMAL_THREAT_RANGE));
        w.spawn(AnimalKind::Rabbit, 12, 10);
        assert!(w.nearby_prey(10, 10, ANIMAL_THREAT_RANGE));
        let mut boar_only = AnimalWorld::new();
        boar_only.spawn(AnimalKind::Boar, 10, 12);
        assert!(boar_only.nearby_prey(10, 10, ANIMAL_THREAT_RANGE));
    }

    #[test]
    fn nearest_threat_dir_points_at_closest_wolf() {
        let mut w = AnimalWorld::new();
        w.spawn(AnimalKind::Wolf, 14, 10); // dx=4
        w.spawn(AnimalKind::Wolf, 12, 10); // dx=2 closer
        assert_eq!(w.nearest_threat_dir(10, 10, ANIMAL_THREAT_RANGE), Some((2, 0)));
        assert_eq!(w.nearest_threat_dir(10, 10, 1), None);
        // No wolves
        let empty = AnimalWorld::new();
        assert_eq!(empty.nearest_threat_dir(0, 0, 5), None);
    }

    /// Arc-share contract: readers see sim-published animals (like craft graph Arc).
    #[test]
    fn animal_world_share_publish_readable() {
        let share: AnimalWorldShare = Arc::new(RwLock::new(AnimalWorld::new()));
        {
            let mut live = AnimalWorld::new();
            live.spawn(AnimalKind::Wolf, 5, 5);
            *share.write().unwrap() = live;
        }
        let r = share.read().unwrap();
        assert!(r.nearby_threat(5, 5, ANIMAL_THREAT_RANGE));
        assert_eq!(r.nearest_threat_dir(3, 5, ANIMAL_THREAT_RANGE), Some((2, 0)));
    }

    #[test]
    fn damage_hits_and_kills_removes() {
        let mut w = AnimalWorld::new();
        let id = w.spawn(AnimalKind::Rabbit, 1, 1);
        // Partial damage
        let r = w.damage(id, 2);
        assert_eq!(r, Some((3, AnimalKind::Rabbit, false)));
        assert_eq!(w.animals.len(), 1);
        assert_eq!(w.animals[0].hp, 3);
        // Kill
        let r = w.damage(id, 10);
        assert_eq!(r, Some((0, AnimalKind::Rabbit, true)));
        assert!(w.animals.is_empty());
        // Unknown id
        assert_eq!(w.damage(99, 1), None);
    }

    #[test]
    fn nearest_id_within_range() {
        let mut w = AnimalWorld::new();
        w.spawn(AnimalKind::Rabbit, 10, 10);
        let wolf = w.spawn(AnimalKind::Wolf, 2, 0);
        assert_eq!(w.nearest_id(0, 0, 1), None);
        assert_eq!(w.nearest_id(0, 0, 2), Some(wolf));
        assert_eq!(w.nearest_id(10, 10, 0), Some(0));
    }
}
