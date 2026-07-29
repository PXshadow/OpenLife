//! Per-tile session locks (SAY LOCK/UNLOCK) **and** pure Haxe key/lock flow.
//!
//! Session [`LockState`] is orthogonal to object `externId` pairing (TH-LOCK / LOCKPICK).
//!
//! Haxe: `TransitionHelper.doCommandHelper` key gates + `TransitionHelper.LockPick`
//! + ownership pairing for Lock and Key 912/1000 + owner empty-hand open.
//! Live knobs: Haxe `ServerSettings.Lockpick*` via `LockpickSettings` / LiveSettings
//! (LOCKPICK-SETTINGS / lockpick_live_knobs).

use std::collections::HashSet;
use std::sync::Mutex;

/// Pending private say from key/lock / alt-outcome USE (conn_id, text).
/// Taken by USE intent handler.
// Haxe: player.say(..., true) — String so dynamic "Try again! Hits N" works (TH-ALT-OUTCOME)
static LAST_LOCK_SAY: Mutex<Option<(u64, String)>> = Mutex::new(None);

/// Record Haxe `player.say(..., true)` from lock/key / alt-outcome gates for this USE.
pub fn note_lock_say(conn_id: u64, text: impl Into<String>) {
    if let Ok(mut g) = LAST_LOCK_SAY.lock() {
        *g = Some((conn_id, text.into()));
    }
}

/// Take and clear pending lock say (if any).
pub fn take_lock_say() -> Option<(u64, String)> {
    LAST_LOCK_SAY.lock().ok().and_then(|mut g| g.take())
}

// ---------------------------------------------------------------------------
// Object ids (OHOL / Open Life)
// ---------------------------------------------------------------------------

/// Key (pairs with Locked* via externId).
pub const KEY_OBJ: i32 = 917;
/// Lock Removal Key (mismatch → LockPick).
pub const LOCK_REMOVAL_KEY_OBJ: i32 = 1003;
/// Lock Blank — receives key externId from held Key 917.
pub const LOCK_BLANK_OBJ: i32 = 904;
/// Lock — same copy path as blank.
pub const LOCK_OBJ: i32 = 4058;
/// Lock and Key — sets owner + pairs externId on USE.
pub const LOCK_AND_KEY_OBJ: i32 = 912;
/// Lock and Key -removed.
pub const LOCK_AND_KEY_REMOVED_OBJ: i32 = 1000;
/// Default broken-key target (content also patches 917/1003 → 862).
pub const BROKEN_KEY_OBJ: i32 = 862;

// ---------------------------------------------------------------------------
// ServerSettings lockpick defaults
// Haxe: ServerSettings.LockpickSucessChance / FailChance / ExhaustionCost / CoinCost
// ---------------------------------------------------------------------------

/// Haxe `LockpickSucessChance` (%).
pub const LOCKPICK_SUCCESS_CHANCE: f32 = 5.0;
/// Haxe `LockpickFailChance` (%).
pub const LOCKPICK_FAIL_CHANCE: f32 = 10.0;
/// Haxe `LockpickExhaustionCost`.
pub const LOCKPICK_EXHAUSTION_COST: f32 = 3.0;
/// Haxe `LockpickCoinCost`.
pub const LOCKPICK_COIN_COST: f32 = 1.0;

/// Female: exhaustion × 0.5, failChance × 0.8.
// Haxe: TransitionHelper.LockPick isFemale branch
pub const LOCKPICK_FEMALE_EXHAUSTION_MULT: f32 = 0.5;
pub const LOCKPICK_FEMALE_FAIL_MULT: f32 = 0.8;

/// Runtime lockpick knobs (Haxe ServerSettings subset).
///
/// Live-reloaded from `server.toml` via `LiveSettings` → `SimState::lockpick_settings`
/// (LOCKPICK-SETTINGS / lockpick_live_knobs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LockpickSettings {
    pub success_chance: f32,
    pub fail_chance: f32,
    pub exhaustion_cost: f32,
    pub coin_cost: f32,
}

impl Default for LockpickSettings {
    fn default() -> Self {
        Self {
            success_chance: LOCKPICK_SUCCESS_CHANCE,
            fail_chance: LOCKPICK_FAIL_CHANCE,
            exhaustion_cost: LOCKPICK_EXHAUSTION_COST,
            coin_cost: LOCKPICK_COIN_COST,
        }
    }
}

impl LockpickSettings {
    /// Build from raw knobs (non-finite or negative → Haxe defaults).
    // Haxe: ServerSettings.Lockpick*
    pub fn from_parts(
        success_chance: f32,
        fail_chance: f32,
        exhaustion_cost: f32,
        coin_cost: f32,
    ) -> Self {
        Self {
            success_chance: sanitize_nonneg(success_chance, LOCKPICK_SUCCESS_CHANCE),
            fail_chance: sanitize_nonneg(fail_chance, LOCKPICK_FAIL_CHANCE),
            exhaustion_cost: sanitize_nonneg(exhaustion_cost, LOCKPICK_EXHAUSTION_COST),
            coin_cost: sanitize_nonneg(coin_cost, LOCKPICK_COIN_COST),
        }
    }

    /// Map hot-reload snapshot fields (already sanitized by `ol-config`).
    // Haxe: ServerSettings.Lockpick* after ReadServerSettings
    pub fn from_live(
        success_chance: f32,
        fail_chance: f32,
        exhaustion_cost: f32,
        coin_cost: f32,
    ) -> Self {
        Self::from_parts(success_chance, fail_chance, exhaustion_cost, coin_cost)
    }
}

#[inline]
fn sanitize_nonneg(v: f32, default: f32) -> f32 {
    if v.is_finite() && v >= 0.0 {
        v
    } else {
        default
    }
}

/// Apply female multipliers to exhaustion cost and fail chance.
// Haxe: TransitionHelper.LockPick
pub fn lockpick_settings_for_player(base: &LockpickSettings, is_female: bool) -> LockpickSettings {
    if !is_female {
        return *base;
    }
    LockpickSettings {
        success_chance: base.success_chance,
        fail_chance: base.fail_chance * LOCKPICK_FEMALE_FAIL_MULT,
        exhaustion_cost: base.exhaustion_cost * LOCKPICK_FEMALE_EXHAUSTION_MULT,
        coin_cost: base.coin_cost,
    }
}

/// Map pure lockpick `coins_after` (Haxe `player.coins: Float`) onto integer wallet.
///
/// Haxe keeps fractional coins; Rust `Wallet.coins` is `i32`. Floor non-negative
/// remainders so fractional live `lockpick_coin_cost` (e.g. 1.5) never *adds*
/// phantom integer coins. Negative / non-finite → 0.
// Haxe: TransitionHelper.LockPick player.coins -= coinCost (Float)
#[inline]
pub fn lockpick_coins_to_wallet_i32(coins_after: f32) -> i32 {
    if !coins_after.is_finite() || coins_after <= 0.0 {
        return 0;
    }
    // floor keeps remainder ≤ pure Float; matches chest Math.floor coin store spirit
    coins_after.floor() as i32
}

// ---------------------------------------------------------------------------
// Session tile locks (SAY LOCK / UNLOCK / CLAIM path — not Haxe object keys)
// ---------------------------------------------------------------------------

/// Set of locked tile coordinates (session only; not OLW).
#[derive(Debug, Default, Clone)]
pub struct LockState {
    pub locked: HashSet<(i32, i32)>,
}

impl LockState {
    pub fn lock(&mut self, x: i32, y: i32) {
        self.locked.insert((x, y));
    }

    pub fn unlock(&mut self, x: i32, y: i32) -> bool {
        self.locked.remove(&(x, y))
    }

    pub fn is_locked(&self, x: i32, y: i32) -> bool {
        self.locked.contains(&(x, y))
    }

    pub fn count(&self) -> usize {
        self.locked.len()
    }

    pub fn format_query(&self, x: i32, y: i32) -> String {
        format!(
            "LOCKTILE {x} {y} locked={}",
            if self.is_locked(x, y) { 1 } else { 0 }
        )
    }
}

// ---------------------------------------------------------------------------
// Pure key / lock flow
// ---------------------------------------------------------------------------

/// Haxe `objectData.description.contains('Locked')`.
#[inline]
pub fn description_is_locked(description: &str) -> bool {
    description.contains("Locked")
}

/// Map unit random \[0,1\] → key id in **1..=10000**.
///
/// Haxe `WorldMap.randomInt(10000)` is 0..=10000; 0 is the unset sentinel for
/// `externId`, so the port uses 1..=10000 for fresh pairing.
// Haxe: WorldMap.world.randomInt(10000)
pub fn random_key_id(unit_random: f32) -> i32 {
    let u = if unit_random.is_finite() {
        unit_random.clamp(0.0, 1.0)
    } else {
        0.0
    };
    // floor(u * 10000) + 1 → 1..=10000 for u in [0,1]
    let v = (u * 10000.0).floor() as i32 + 1;
    v.clamp(1, 10000)
}

/// Outcome of Key 917 vs Locked* externId gate (before transition apply).
// Haxe: TransitionHelper.doCommandHelper Key 917 + isLocked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMatchOutcome {
    /// Both zero → assign shared `new_key_id` to held + target.
    PairBoth {
        new_key_id: i32,
    },
    /// Held non-zero, target zero → copy held onto target.
    AssignTarget {
        key_id: i32,
    },
    /// Already matching (including both already set equal).
    Match,
    /// Mismatch — refuse USE (Haxe says KEY DOES NOT FIT; no lockpick on 917).
    Mismatch,
}

/// Pure Key 917 pairing against a Locked* target's extern ids.
///
/// `new_key_id` is only used when both externs are 0 (caller supplies
/// [`random_key_id`]).
// Haxe: TransitionHelper.doCommandHelper L219-239
pub fn key_match(held_extern: i32, target_extern: i32, new_key_id: i32) -> KeyMatchOutcome {
    if held_extern == 0 && target_extern == 0 {
        let id = if new_key_id > 0 {
            new_key_id
        } else {
            1
        };
        return KeyMatchOutcome::PairBoth { new_key_id: id };
    }
    if held_extern != 0 && target_extern == 0 {
        return KeyMatchOutcome::AssignTarget {
            key_id: held_extern,
        };
    }
    if held_extern != target_extern {
        return KeyMatchOutcome::Mismatch;
    }
    KeyMatchOutcome::Match
}

/// Whether Key 917 USE on `target_id` should copy held extern onto the lock blank.
// Haxe: TransitionHelper.doCommandHelper L253-258 (904 / 4058)
#[inline]
pub fn is_blank_lock_target(target_id: i32) -> bool {
    target_id == LOCK_BLANK_OBJ || target_id == LOCK_OBJ
}

/// Held is Lock and Key family used for ownership pairing.
// Haxe: heldId == 912 || heldId == 1000
#[inline]
pub fn is_lock_and_key_held(held_id: i32) -> bool {
    held_id == LOCK_AND_KEY_OBJ || held_id == LOCK_AND_KEY_REMOVED_OBJ
}

/// Pair held + target extern ids for Lock and Key 912/1000 on a non-empty target.
///
/// Returns `(held_extern_out, target_extern_out)`. When held was 0, uses `new_key_id`.
// Haxe: TransitionHelper.doTransitionIfPossibleHelper L997-1007
pub fn lock_and_key_pair_extern(
    held_extern: i32,
    new_key_id: i32,
) -> (i32, i32) {
    let id = if held_extern == 0 {
        if new_key_id > 0 {
            new_key_id
        } else {
            1
        }
    } else {
        held_extern
    };
    (id, id)
}

/// Lockpick attempt outcome (after cost gates).
// Haxe: TransitionHelper.LockPick
#[derive(Debug, Clone, PartialEq)]
pub enum LockpickOutcome {
    /// `player.coins < coinCost`
    NeedCoins,
    /// `exhaustion > food_store_max / 2`
    TooExhausted,
    /// `rand * 100 < successChance`
    Success {
        coins_after: f32,
        exhaustion_after: f32,
        say: &'static str,
    },
    /// `rand * 100 > 100 - failChance` → transform held to decays_to
    Broke {
        coins_after: f32,
        exhaustion_after: f32,
        new_held_id: i32,
        say: &'static str,
    },
    /// Failed, key intact
    Failed {
        coins_after: f32,
        exhaustion_after: f32,
        say: &'static str,
    },
}

impl LockpickOutcome {
    /// True when lockpick opened the lock (USE transition may proceed).
    pub fn allows_use(&self) -> bool {
        matches!(self, LockpickOutcome::Success { .. })
    }

    pub fn say(&self) -> Option<&'static str> {
        match self {
            LockpickOutcome::NeedCoins => Some("Need more coins"),
            LockpickOutcome::TooExhausted => Some("I am too exhausted!"),
            LockpickOutcome::Success { say, .. }
            | LockpickOutcome::Broke { say, .. }
            | LockpickOutcome::Failed { say, .. } => Some(*say),
        }
    }
}

/// Pure lockpick roll (Lock Removal Key 1003 mismatch path).
///
/// `rng01` is Haxe `calculateRandomFloat()` in \[0,1\]; compared as `rng01 * 100`.
/// `decays_to_obj` is held key's `objectData.decaysToObj` (0 → [`BROKEN_KEY_OBJ`]).
// Haxe: TransitionHelper.LockPick L418-455
pub fn try_lockpick(
    coins: f32,
    exhaustion: f32,
    food_store_max: f32,
    is_female: bool,
    settings: &LockpickSettings,
    decays_to_obj: i32,
    rng01: f32,
) -> LockpickOutcome {
    let s = lockpick_settings_for_player(settings, is_female);
    let coin_cost = s.coin_cost.max(0.0);
    let exh_cost = s.exhaustion_cost.max(0.0);

    if coins < coin_cost {
        return LockpickOutcome::NeedCoins;
    }
    // Haxe: player.exhaustion > player.food_store_max / 2
    if exhaustion > food_store_max / 2.0 {
        return LockpickOutcome::TooExhausted;
    }

    let coins_after = coins - coin_cost;
    let exhaustion_after = exhaustion + exh_cost;
    let r = if rng01.is_finite() {
        rng01.clamp(0.0, 1.0) * 100.0
    } else {
        50.0
    };

    if r < s.success_chance {
        return LockpickOutcome::Success {
            coins_after,
            exhaustion_after,
            say: "I got it!",
        };
    }
    if r > 100.0 - s.fail_chance {
        let broken = if decays_to_obj > 0 {
            decays_to_obj
        } else {
            BROKEN_KEY_OBJ
        };
        return LockpickOutcome::Broke {
            coins_after,
            exhaustion_after,
            new_held_id: broken,
            say: "Damn it broke!",
        };
    }
    LockpickOutcome::Failed {
        coins_after,
        exhaustion_after,
        say: "Failed!",
    }
}

/// Pure gate: owner may open locked object empty-handed when a Key 917 transition exists.
// Haxe: TransitionHelper.doTransitionIfPossibleHelper L1010-1025
pub fn owner_may_open_empty_hand(
    is_held_empty: bool,
    transition_was_null: bool,
    key_transition_exists: bool,
    owner_account: Option<i32>,
    player_account: i32,
) -> bool {
    if !is_held_empty || !transition_was_null || !key_transition_exists {
        return false;
    }
    match owner_account {
        Some(oid) if oid != 0 && oid == player_account => true,
        _ => false,
    }
}

/// Resolve target owner account id from helper (first account owner, else 0).
#[inline]
pub fn owner_account_of(owners_by_account: &[i32], owner_id: i32) -> Option<i32> {
    if let Some(&a) = owners_by_account.first() {
        if a != 0 {
            return Some(a);
        }
    }
    if owner_id != 0 {
        Some(owner_id)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// USE pre-gate result (wire-facing pure bundle)
// ---------------------------------------------------------------------------

/// Side effects + allow/refuse for key/lock USE pre-gates (no world mutation).
#[derive(Debug, Clone, PartialEq)]
pub struct LockUseGate {
    pub allow: bool,
    pub held_extern: i32,
    pub target_extern: i32,
    /// When set, transform held to this id (broken key).
    pub held_id_override: Option<i32>,
    pub coins_after: Option<f32>,
    pub exhaustion_after: Option<f32>,
    pub say: Option<&'static str>,
    /// True when Lock and Key should stamp ownership + hits=1 after place.
    pub claim_ownership: bool,
}

impl LockUseGate {
    pub fn allow_passthrough(held_extern: i32, target_extern: i32) -> Self {
        Self {
            allow: true,
            held_extern,
            target_extern,
            held_id_override: None,
            coins_after: None,
            exhaustion_after: None,
            say: None,
            claim_ownership: false,
        }
    }
}

/// Evaluate Haxe key/lock USE gates before applying a transition.
///
/// Covers:
/// - Key 917 + Locked* pairing / refuse
/// - Lock Removal Key 1003 + Locked* + lockpick on mismatch
/// - Key 917 on blank lock 904/4058 copy
/// - Lock and Key 912/1000 ownership pair flag (when transition will apply)
// Haxe: TransitionHelper.doCommandHelper L214-258 + doTransitionIfPossibleHelper L997-1007
pub fn evaluate_lock_use_gate(
    held_id: i32,
    target_id: i32,
    target_description: &str,
    held_extern: i32,
    target_extern: i32,
    transition_found: bool,
    coins: f32,
    exhaustion: f32,
    food_store_max: f32,
    is_female: bool,
    settings: &LockpickSettings,
    decays_to_obj: i32,
    unit_random_key: f32,
    unit_random_lockpick: f32,
) -> LockUseGate {
    let is_locked = description_is_locked(target_description);
    let mut gate = LockUseGate::allow_passthrough(held_extern, target_extern);

    // Key 917 + Locked*
    if held_id == KEY_OBJ && is_locked {
        let new_id = random_key_id(unit_random_key);
        match key_match(held_extern, target_extern, new_id) {
            KeyMatchOutcome::PairBoth { new_key_id } => {
                gate.held_extern = new_key_id;
                gate.target_extern = new_key_id;
                gate.say = Some("NEW KEY FOR TARGET!");
            }
            KeyMatchOutcome::AssignTarget { key_id } => {
                gate.target_extern = key_id;
                gate.say = Some("USE KEY FOR TARGET!!");
            }
            KeyMatchOutcome::Match => {}
            KeyMatchOutcome::Mismatch => {
                // Haxe L236 TODO lockpick — port-as-is: refuse only
                gate.allow = false;
                gate.say = Some("KEY DOES NOT FIT!");
                return gate;
            }
        }
    }

    // Lock Removal Key 1003 + Locked*
    if held_id == LOCK_REMOVAL_KEY_OBJ && is_locked {
        if held_extern != target_extern {
            match try_lockpick(
                coins,
                exhaustion,
                food_store_max,
                is_female,
                settings,
                decays_to_obj,
                unit_random_lockpick,
            ) {
                LockpickOutcome::NeedCoins => {
                    gate.allow = false;
                    gate.say = Some("Need more coins");
                    return gate;
                }
                LockpickOutcome::TooExhausted => {
                    gate.allow = false;
                    gate.say = Some("I am too exhausted!");
                    return gate;
                }
                LockpickOutcome::Success {
                    coins_after,
                    exhaustion_after,
                    say,
                } => {
                    gate.coins_after = Some(coins_after);
                    gate.exhaustion_after = Some(exhaustion_after);
                    gate.say = Some(say);
                    // allow continues
                }
                LockpickOutcome::Broke {
                    coins_after,
                    exhaustion_after,
                    new_held_id,
                    say,
                } => {
                    gate.allow = false;
                    gate.coins_after = Some(coins_after);
                    gate.exhaustion_after = Some(exhaustion_after);
                    gate.held_id_override = Some(new_held_id);
                    gate.say = Some(say);
                    return gate;
                }
                LockpickOutcome::Failed {
                    coins_after,
                    exhaustion_after,
                    say,
                } => {
                    gate.allow = false;
                    gate.coins_after = Some(coins_after);
                    gate.exhaustion_after = Some(exhaustion_after);
                    gate.say = Some(say);
                    return gate;
                }
            }
        }
    }

    // Key 917 on Lock Blank 904 / Lock 4058
    if held_id == KEY_OBJ && is_blank_lock_target(target_id) {
        gate.target_extern = held_extern;
        gate.say = Some("LOCK HAS NOW THE SAME KEY!");
    }

    // Lock and Key 912/1000 on non-empty target (when a transition will run)
    if transition_found && is_lock_and_key_held(held_id) && target_id != 0 {
        let new_id = random_key_id(unit_random_key);
        let (he, te) = lock_and_key_pair_extern(held_extern, new_id);
        gate.held_extern = he;
        gate.target_extern = te;
        gate.claim_ownership = true;
        gate.say = Some("Its mine now!");
    }

    gate
}

// ---------------------------------------------------------------------------
// Session + pure tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_unlock_session() {
        let mut l = LockState::default();
        l.lock(3, 4);
        assert!(l.is_locked(3, 4));
        assert!(l.unlock(3, 4));
        assert!(!l.is_locked(3, 4));
        assert!(l.format_query(3, 4).contains("locked=0"));
    }

    #[test]
    fn description_locked_substring() {
        assert!(description_is_locked("Locked Wooden Chest"));
        assert!(!description_is_locked("Wooden Chest"));
        assert!(description_is_locked("Something Locked Inside"));
    }

    #[test]
    fn random_key_id_range() {
        assert_eq!(random_key_id(0.0), 1);
        assert_eq!(random_key_id(0.9999), 10000);
        assert!((1..=10000).contains(&random_key_id(0.5)));
    }

    #[test]
    fn key_match_both_zero_pairs() {
        match key_match(0, 0, 4242) {
            KeyMatchOutcome::PairBoth { new_key_id } => assert_eq!(new_key_id, 4242),
            o => panic!("expected PairBoth, got {o:?}"),
        }
    }

    #[test]
    fn key_match_assign_target() {
        match key_match(77, 0, 1) {
            KeyMatchOutcome::AssignTarget { key_id } => assert_eq!(key_id, 77),
            o => panic!("expected AssignTarget, got {o:?}"),
        }
    }

    #[test]
    fn key_match_mismatch() {
        assert_eq!(key_match(1, 2, 9), KeyMatchOutcome::Mismatch);
    }

    #[test]
    fn key_match_equal() {
        assert_eq!(key_match(5, 5, 9), KeyMatchOutcome::Match);
        assert_eq!(key_match(0, 5, 9), KeyMatchOutcome::Mismatch); // held 0, target set
    }

    #[test]
    fn try_lockpick_need_coins() {
        let s = LockpickSettings::default();
        assert_eq!(
            try_lockpick(0.0, 0.0, 20.0, false, &s, 862, 0.01),
            LockpickOutcome::NeedCoins
        );
    }

    #[test]
    fn try_lockpick_too_exhausted() {
        let s = LockpickSettings::default();
        // food_max 20 → half 10; exhaustion 11 > 10
        assert_eq!(
            try_lockpick(5.0, 11.0, 20.0, false, &s, 862, 0.01),
            LockpickOutcome::TooExhausted
        );
    }

    #[test]
    fn try_lockpick_success() {
        let s = LockpickSettings::default();
        // success_chance 5 → r < 5 succeeds; rng01=0.04 → 4
        match try_lockpick(5.0, 0.0, 20.0, false, &s, 862, 0.04) {
            LockpickOutcome::Success {
                coins_after,
                exhaustion_after,
                ..
            } => {
                assert!((coins_after - 4.0).abs() < 1e-5);
                assert!((exhaustion_after - 3.0).abs() < 1e-5);
            }
            o => panic!("expected Success, got {o:?}"),
        }
    }

    #[test]
    fn try_lockpick_break() {
        let s = LockpickSettings::default();
        // failChance 10 → break when r > 90; rng01=0.95 → 95
        match try_lockpick(2.0, 1.0, 20.0, false, &s, 862, 0.95) {
            LockpickOutcome::Broke {
                new_held_id,
                coins_after,
                ..
            } => {
                assert_eq!(new_held_id, 862);
                assert!((coins_after - 1.0).abs() < 1e-5);
            }
            o => panic!("expected Broke, got {o:?}"),
        }
    }

    #[test]
    fn try_lockpick_failed_middle() {
        let s = LockpickSettings::default();
        // r=50 → neither success nor break
        match try_lockpick(3.0, 0.0, 20.0, false, &s, 862, 0.5) {
            LockpickOutcome::Failed { .. } => {}
            o => panic!("expected Failed, got {o:?}"),
        }
    }

    #[test]
    fn try_lockpick_female_halves_exhaustion_and_fail() {
        let s = LockpickSettings::default();
        // female failChance = 8 → break when r > 92
        // rng01=0.91 → 91: male would break (>90), female fails soft
        match try_lockpick(5.0, 0.0, 20.0, true, &s, 862, 0.91) {
            LockpickOutcome::Failed {
                exhaustion_after, ..
            } => {
                // exhaustion cost 3 * 0.5 = 1.5
                assert!((exhaustion_after - 1.5).abs() < 1e-5);
            }
            o => panic!("expected Failed for female mid-high roll, got {o:?}"),
        }
        // male at same roll breaks
        match try_lockpick(5.0, 0.0, 20.0, false, &s, 862, 0.91) {
            LockpickOutcome::Broke { .. } => {}
            o => panic!("expected Broke for male, got {o:?}"),
        }
    }

    #[test]
    fn blank_lock_and_lock_and_key_ids() {
        assert!(is_blank_lock_target(904));
        assert!(is_blank_lock_target(4058));
        assert!(!is_blank_lock_target(988));
        assert!(is_lock_and_key_held(912));
        assert!(is_lock_and_key_held(1000));
        assert_eq!(lock_and_key_pair_extern(0, 333), (333, 333));
        assert_eq!(lock_and_key_pair_extern(9, 1), (9, 9));
    }

    #[test]
    fn owner_open_pure() {
        assert!(owner_may_open_empty_hand(true, true, true, Some(7), 7));
        assert!(!owner_may_open_empty_hand(true, true, true, Some(7), 8));
        assert!(!owner_may_open_empty_hand(false, true, true, Some(7), 7));
        assert!(!owner_may_open_empty_hand(true, false, true, Some(7), 7));
        assert!(!owner_may_open_empty_hand(true, true, false, Some(7), 7));
    }

    #[test]
    fn evaluate_gate_key_mismatch_refuses() {
        let s = LockpickSettings::default();
        let g = evaluate_lock_use_gate(
            KEY_OBJ,
            988,
            "Locked Wooden Chest",
            1,
            2,
            true,
            10.0,
            0.0,
            20.0,
            false,
            &s,
            862,
            0.5,
            0.01,
        );
        assert!(!g.allow);
        assert_eq!(g.say, Some("KEY DOES NOT FIT!"));
    }

    #[test]
    fn evaluate_gate_key_pair_both_zero() {
        let s = LockpickSettings::default();
        let g = evaluate_lock_use_gate(
            KEY_OBJ,
            988,
            "Locked Wooden Chest",
            0,
            0,
            true,
            10.0,
            0.0,
            20.0,
            false,
            &s,
            862,
            0.1, // random_key_id → 1001
            0.0,
        );
        assert!(g.allow);
        assert_eq!(g.held_extern, g.target_extern);
        assert!(g.held_extern > 0);
    }

    #[test]
    fn evaluate_gate_1003_success_allows() {
        let s = LockpickSettings::default();
        let g = evaluate_lock_use_gate(
            LOCK_REMOVAL_KEY_OBJ,
            988,
            "Locked Wooden Chest",
            1,
            2,
            true,
            10.0,
            0.0,
            20.0,
            false,
            &s,
            862,
            0.5,
            0.01, // success
        );
        assert!(g.allow);
        assert_eq!(g.say, Some("I got it!"));
        assert!(g.coins_after.is_some());
    }

    #[test]
    fn evaluate_gate_blank_lock_copies() {
        let s = LockpickSettings::default();
        let g = evaluate_lock_use_gate(
            KEY_OBJ,
            LOCK_BLANK_OBJ,
            "Lock Blank",
            55,
            0,
            true,
            0.0,
            0.0,
            20.0,
            false,
            &s,
            862,
            0.0,
            0.0,
        );
        assert!(g.allow);
        assert_eq!(g.target_extern, 55);
        assert_eq!(g.say, Some("LOCK HAS NOW THE SAME KEY!"));
    }

    #[test]
    fn evaluate_gate_lock_and_key_claims() {
        let s = LockpickSettings::default();
        let g = evaluate_lock_use_gate(
            LOCK_AND_KEY_OBJ,
            100,
            "Door",
            0,
            0,
            true,
            0.0,
            0.0,
            20.0,
            false,
            &s,
            862,
            0.2,
            0.0,
        );
        assert!(g.allow);
        assert!(g.claim_ownership);
        assert_eq!(g.held_extern, g.target_extern);
        assert!(g.held_extern > 0);
    }

    #[test]
    fn female_settings_mult() {
        let s = LockpickSettings::default();
        let f = lockpick_settings_for_player(&s, true);
        assert!((f.exhaustion_cost - 1.5).abs() < 1e-5);
        assert!((f.fail_chance - 8.0).abs() < 1e-5);
        assert!((f.success_chance - 5.0).abs() < 1e-5);
        assert!((f.coin_cost - 1.0).abs() < 1e-5);
    }

    #[test]
    fn lockpick_settings_from_parts_and_live() {
        let s = LockpickSettings::from_parts(50.0, 20.0, 1.0, 2.0);
        assert!((s.success_chance - 50.0).abs() < 1e-5);
        assert!((s.fail_chance - 20.0).abs() < 1e-5);
        assert!((s.exhaustion_cost - 1.0).abs() < 1e-5);
        assert!((s.coin_cost - 2.0).abs() < 1e-5);
        // Guaranteed success with high chance; coin cost 2
        match try_lockpick(5.0, 0.0, 20.0, false, &s, 862, 0.4) {
            LockpickOutcome::Success {
                coins_after,
                exhaustion_after,
                ..
            } => {
                assert!((coins_after - 3.0).abs() < 1e-5);
                assert!((exhaustion_after - 1.0).abs() < 1e-5);
            }
            o => panic!("expected Success with high chance, got {o:?}"),
        }
        let bad = LockpickSettings::from_live(f32::NAN, -1.0, f32::INFINITY, -9.0);
        assert_eq!(bad, LockpickSettings::default());
        // zero coin cost allowed
        let free = LockpickSettings::from_parts(100.0, 0.0, 0.0, 0.0);
        match try_lockpick(0.0, 0.0, 20.0, false, &free, 862, 0.0) {
            LockpickOutcome::Success { coins_after, .. } => {
                assert!((coins_after - 0.0).abs() < 1e-5);
            }
            o => panic!("expected free Success, got {o:?}"),
        }
    }

    /// Fractional live coin_cost: pure keeps f32; wallet floor maps remainder.
    // Haxe: player.coins Float; Rust Wallet.coins i32
    #[test]
    fn fractional_coin_cost_pure_and_wallet_floor() {
        let s = LockpickSettings::from_parts(100.0, 0.0, 0.0, 1.5);
        match try_lockpick(5.0, 0.0, 20.0, false, &s, 862, 0.0) {
            LockpickOutcome::Success { coins_after, .. } => {
                assert!((coins_after - 3.5).abs() < 1e-5);
                assert_eq!(lockpick_coins_to_wallet_i32(coins_after), 3);
            }
            o => panic!("expected Success, got {o:?}"),
        }
        // Gate uses f32 compare: 1 coin < 1.5 cost → NeedCoins
        assert_eq!(
            try_lockpick(1.0, 0.0, 20.0, false, &s, 862, 0.0),
            LockpickOutcome::NeedCoins
        );
        assert_eq!(lockpick_coins_to_wallet_i32(0.9), 0);
        assert_eq!(lockpick_coins_to_wallet_i32(-1.0), 0);
        assert_eq!(lockpick_coins_to_wallet_i32(f32::NAN), 0);
        assert_eq!(lockpick_coins_to_wallet_i32(4.0), 4);
    }

    /// Live exhaustion_cost reloaded → female try_lockpick uses half of new cost.
    // Haxe: ServerSettings.LockpickExhaustionCost hot-reload + isFemale * 0.5
    #[test]
    fn live_exhaustion_cost_female_half() {
        let base = LockpickSettings::from_live(5.0, 10.0, 8.0, 1.0);
        assert!((base.exhaustion_cost - 8.0).abs() < 1e-5);
        let for_f = lockpick_settings_for_player(&base, true);
        assert!((for_f.exhaustion_cost - 4.0).abs() < 1e-5);
        // r=50 mid-band Failed; female exhaustion_after = 0 + 4
        match try_lockpick(5.0, 0.0, 20.0, true, &base, 862, 0.5) {
            LockpickOutcome::Failed {
                exhaustion_after, ..
            } => assert!((exhaustion_after - 4.0).abs() < 1e-5),
            o => panic!("expected Failed, got {o:?}"),
        }
        match try_lockpick(5.0, 0.0, 20.0, false, &base, 862, 0.5) {
            LockpickOutcome::Failed {
                exhaustion_after, ..
            } => assert!((exhaustion_after - 8.0).abs() < 1e-5),
            o => panic!("expected Failed male, got {o:?}"),
        }
    }

    /// success_chance=100 always opens; success=0 fail=100 always breaks (rng>0).
    #[test]
    fn evaluate_gate_live_forced_success_and_break() {
        let ok = LockpickSettings::from_parts(100.0, 0.0, 1.0, 2.0);
        let g = evaluate_lock_use_gate(
            LOCK_REMOVAL_KEY_OBJ,
            988,
            "Locked Wooden Chest",
            1,
            2,
            true,
            10.0,
            0.0,
            20.0,
            false,
            &ok,
            862,
            0.5,
            0.99, // would break under defaults; 100% success wins first
        );
        assert!(g.allow);
        assert_eq!(g.say, Some("I got it!"));
        assert_eq!(g.coins_after, Some(8.0));
        assert_eq!(g.exhaustion_after, Some(1.0));

        let brk = LockpickSettings::from_parts(0.0, 100.0, 2.0, 1.0);
        let g2 = evaluate_lock_use_gate(
            LOCK_REMOVAL_KEY_OBJ,
            988,
            "Locked Wooden Chest",
            1,
            2,
            true,
            5.0,
            0.0,
            20.0,
            false,
            &brk,
            862,
            0.5,
            0.5, // r=50 > 0 → break
        );
        assert!(!g2.allow);
        assert_eq!(g2.say, Some("Damn it broke!"));
        assert_eq!(g2.held_id_override, Some(862));
        assert_eq!(g2.coins_after, Some(4.0));
    }
}
