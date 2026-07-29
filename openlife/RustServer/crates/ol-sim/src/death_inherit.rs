//! Death inheritance polish (Haxe `GlobalPlayerInstance` death helper family).
//!
//! | Haxe | Rust |
//! |------|------|
//! | `InheritCoins` | [`apply_inherit_coins`] — past-actions + kids + residual |
//! | residual TODO "store coins in grave" | residual → [`InheritTransfer`] kind `"grave"` when tile set |
//! | `InheritOwnership` | [`apply_inherit_ownership_on_helpers`] |
//! | `ChooseNewLeader` | [`choose_new_leader`] |
//! | `countLeadershipPower` | [`count_leadership_power`] |
//!
//! Account "soul" credit remains `coinsInherited` via
//! [`crate::accounts::AccountBook::credit_coins_inherited`].

use crate::accounts::AccountBook;
use crate::ally::AllyState;
use crate::economy::{Economy, INHERIT_COINS_FACTOR};
use crate::prestige::PrestigeClass;
use crate::relations::{
    is_close_relative, is_leadership_ally, is_same_family, living_children_of,
};
use crate::score::Scoreboard;
use crate::social::SocialState;
use ol_world::ComplexObject;
use std::collections::{HashMap, HashSet};

/// One inheritance transfer for events / tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritTransfer {
    pub to_p_id: i32,
    pub amount: i32,
    /// `"past_actions"` | `"child"` | `"treasury"` | `"grave"`
    pub kind: &'static str,
}

/// Inputs for one death inheritance pass (no SimState coupling).
pub struct InheritContext<'a> {
    pub deceased_p_id: i32,
    pub deceased_email: &'a str,
    /// Living player ids (not deleted) — p_id set.
    pub living: &'a HashMap<i32, bool>,
    pub social: &'a SocialState,
    pub allies: &'a AllyState,
    pub accounts: &'a mut AccountBook,
    pub economy: &'a mut Economy,
    pub scoreboard: &'a mut Scoreboard,
    /// When residual coins have no kids, deposit into this grave helper (Haxe TODO).
    /// `None` → treasury fallback (intentional when no grave placed).
    pub grave: Option<&'a mut ComplexObject>,
    /// Haxe `ServerSettings.InheritCoinsFactor` (live via GameplayKnobs).
    // C-SS-MORE-BATCH4
    pub inherit_coins_factor: f32,
}

/// Run Haxe-shaped InheritCoins. Returns transfers (for events/tests).
pub fn apply_inherit_coins(ctx: &mut InheritContext<'_>) -> Vec<InheritTransfer> {
    let deceased = ctx.deceased_p_id;
    let coins = ctx.economy.take_wallet(deceased);
    ctx.scoreboard.set_coins(deceased, 0);
    if coins <= 0 {
        return vec![];
    }

    // Haxe: account.coinsInherited += coins * InheritCoinsFactor
    // C-SS-MORE-BATCH4: live InheritCoinsFactor
    let factor = if ctx.inherit_coins_factor.is_finite() && ctx.inherit_coins_factor >= 0.0 {
        ctx.inherit_coins_factor
    } else {
        INHERIT_COINS_FACTOR
    };
    ctx.accounts
        .credit_coins_inherited(ctx.deceased_email, coins, factor);

    let mut remaining = coins;
    let mut out = Vec::new();

    // Phase 1: past-actions distribution to ally / same-family living players.
    while remaining >= 1 {
        let mut best: Option<(i32, f32, bool)> = None; // p_id, score, close
        for (&pid, &alive) in ctx.living.iter() {
            if !alive || pid == deceased {
                continue;
            }
            let ally = ctx.allies.is_mutual_or_either(deceased, pid)
                || is_leadership_ally(&ctx.social.following, deceased, pid);
            let family = is_same_family(ctx.social, deceased, pid);
            if !ally && !family {
                continue;
            }
            let email = account_email_for(ctx, pid);
            let mut score = ctx
                .accounts
                .get(&email)
                .map(|r| r.coins_inherited)
                .unwrap_or(0.0);
            let close = is_close_relative(ctx.social, deceased, pid);
            if close {
                score *= 2.0;
            }
            if score < 1.0 {
                continue;
            }
            match best {
                Some((_, best_s, _)) if score <= best_s => {}
                _ => best = Some((pid, score, close)),
            }
        }
        let Some((best_pid, score, close)) = best else {
            break;
        };
        let give = remaining.min(score.floor() as i32).max(0);
        if give < 1 {
            break;
        }
        remaining -= give;
        ctx.economy.add_coins(best_pid, give);
        let w = ctx.economy.coins_of(best_pid);
        ctx.scoreboard.set_coins(best_pid, w);
        // Debit coinsInherited (half rate if close relative — Haxe).
        let email = account_email_for(ctx, best_pid);
        let debit = if close {
            give as f32 / 2.0
        } else {
            give as f32
        };
        if let Some(r) = ctx
            .accounts
            .by_email
            .get_mut(&crate::accounts::normalize_email(&email))
        {
            r.coins_inherited = (r.coins_inherited - debit).max(0.0);
        }
        out.push(InheritTransfer {
            to_p_id: best_pid,
            amount: give,
            kind: "past_actions",
        });
    }

    // Phase 2: remaining → living children equally.
    let kids = if remaining >= 1 {
        living_children_of(ctx.social, deceased, ctx.living, true)
    } else {
        Vec::new()
    };
    if remaining >= 1 && !kids.is_empty() {
        let each = remaining / kids.len() as i32;
        let mut given = 0i32;
        if each >= 1 {
            for &kid in &kids {
                ctx.economy.add_coins(kid, each);
                let w = ctx.economy.coins_of(kid);
                ctx.scoreboard.set_coins(kid, w);
                given += each;
                out.push(InheritTransfer {
                    to_p_id: kid,
                    amount: each,
                    kind: "child",
                });
            }
        }
        // Remainder of integer split stays for treasury/grave.
        remaining -= given;
    }

    // Phase 3: residual.
    // Haxe: no kids → TODO store coins in grave (was silent drop).
    // Rust: prefer grave.coins when helper provided; else treasury.
    if remaining > 0 {
        if kids.is_empty() {
            if let Some(grave) = ctx.grave.as_mut() {
                grave.coins += remaining as f32;
                out.push(InheritTransfer {
                    to_p_id: 0,
                    amount: remaining,
                    kind: "grave",
                });
                remaining = 0;
            }
        }
        if remaining > 0 {
            ctx.economy.deposit_treasury(remaining);
            out.push(InheritTransfer {
                to_p_id: 0,
                amount: remaining,
                kind: "treasury",
            });
        }
    }
    out
}

fn account_email_for(ctx: &InheritContext<'_>, p_id: i32) -> String {
    // Prefer last known email from account book by last_p_id; else synthetic.
    for r in ctx.accounts.by_email.values() {
        if r.last_p_id == p_id {
            return r.email.clone();
        }
    }
    format!("pid{p_id}@inherit.local")
}

/// Format event log lines for transfers.
pub fn format_inherit_events(deceased: i32, transfers: &[InheritTransfer]) -> Vec<String> {
    transfers
        .iter()
        .map(|t| match t.kind {
            "treasury" => format!("INHERIT {deceased} treasury={} {}", t.amount, t.amount),
            "grave" => format!("INHERIT {deceased} grave={} {}", t.amount, t.amount),
            k => format!("INHERIT {deceased} {k}={} {}", t.to_p_id, t.amount),
        })
        .collect()
}

// ─── InheritOwnership ───────────────────────────────────────────────────────

/// Result of transferring ownership from a dead player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipTransfer {
    pub x: i32,
    pub y: i32,
    /// New sole owner when property was empty after removal (`0` if left unowned).
    pub new_owner: i32,
}

/// Haxe `ObjectHelper.removeOwner` / `addOwner` on living_owners + owner_id.
///
/// Mutates `h` in place. Returns `true` if the deceased was an owner.
pub fn remove_owner_from_helper(h: &mut ComplexObject, deceased: i32) -> bool {
    if deceased == 0 {
        return false;
    }
    let was = h.is_owner(deceased);
    if !was {
        return false;
    }
    h.living_owners.retain(|&id| id != deceased);
    if h.owner_id == deceased {
        h.owner_id = h.living_owners.first().copied().unwrap_or(0);
    }
    was
}

/// Haxe `ObjectHelper.addOwner` for a living player id (no account list update).
pub fn add_owner_to_helper(h: &mut ComplexObject, p_id: i32) {
    if p_id == 0 {
        return;
    }
    if h.living_owners.contains(&p_id) {
        if h.owner_id == 0 {
            h.owner_id = p_id;
        }
        return;
    }
    h.living_owners.push(p_id);
    if h.owner_id == 0 {
        h.owner_id = p_id;
    }
}

/// Haxe `InheritOwnership`: remove dead player; if no owners left and they had a
/// follow-leader, transfer sole ownership to that leader.
///
/// `helpers` is a list of `(x, y, helper)` mutated in place.
pub fn apply_inherit_ownership_on_helpers(
    helpers: &mut [((i32, i32), &mut ComplexObject)],
    deceased: i32,
    follow_leader: Option<i32>,
) -> Vec<OwnershipTransfer> {
    let mut out = Vec::new();
    let leader = follow_leader.filter(|&id| id != 0 && id != deceased);
    for &mut ((x, y), ref mut h) in helpers.iter_mut() {
        if !remove_owner_from_helper(h, deceased) {
            continue;
        }
        let mut new_owner = 0i32;
        if h.living_owners.is_empty() {
            if let Some(lid) = leader {
                add_owner_to_helper(h, lid);
                new_owner = lid;
            }
        }
        out.push(OwnershipTransfer { x, y, new_owner });
    }
    out
}

/// Event lines for ownership transfers.
pub fn format_ownership_events(deceased: i32, transfers: &[OwnershipTransfer]) -> Vec<String> {
    transfers
        .iter()
        .map(|t| {
            if t.new_owner != 0 {
                format!(
                    "INHERIT_OWN {deceased} {} {} -> {}",
                    t.x, t.y, t.new_owner
                )
            } else {
                format!("INHERIT_OWN {deceased} {} {} unowned", t.x, t.y)
            }
        })
        .collect()
}

// ─── ChooseNewLeader ────────────────────────────────────────────────────────

/// Haxe `countLeadershipPower` (familyPrestige defaults 0 when account map missing).
///
/// ```text
/// power = (prestige + coins) + family_prestige
/// Noble *= 2; Serf /= 2
/// return power / 10
/// ```
pub fn count_leadership_power(
    prestige: f32,
    coins: i32,
    family_prestige: f32,
    class: PrestigeClass,
) -> f32 {
    let mut power = prestige.max(0.0) + (coins.max(0) as f32) + family_prestige.max(0.0);
    match class {
        PrestigeClass::Noble => power *= 2.0,
        PrestigeClass::Serf => power /= 2.0,
        _ => {}
    }
    power / 10.0
}

/// Outcome of Haxe `ChooseNewLeader`.
#[derive(Debug, Clone, PartialEq)]
pub struct LeaderSuccession {
    pub new_leader: Option<i32>,
    /// Followers reassigned to the new leader: `(follower_id, new_leader_id)`.
    pub reassigned: Vec<(i32, i32)>,
    /// Direct followers of the dead leader (including the elected one) before reassignment.
    pub direct_follower_count: usize,
    /// Followers under the new leader after reassignment, not counting the leader (Haxe `count-1`).
    pub subject_count: usize,
    pub is_king: bool,
    pub is_emperor: bool,
    /// `"Emperor"` | `"King"` | `"Leader"`
    pub title: &'static str,
}

/// Haxe `ChooseNewLeader` pure graph update.
///
/// 1. Among living players with `following[p] == dead_leader`, pick highest
///    [`count_leadership_power`] (caller supplies scores).
/// 2. New leader inherits `dead_leader`'s follow target.
/// 3. Other direct followers re-point to new leader unless mutual exile.
pub fn choose_new_leader(
    dead_leader: i32,
    dead_leader_follow: Option<i32>,
    following: &mut HashMap<i32, i32>,
    exiles: &HashMap<i32, HashSet<i32>>,
    // Living candidate power scores (p_id → power). Missing → skip.
    power: &HashMap<i32, f32>,
) -> LeaderSuccession {
    let empty = LeaderSuccession {
        new_leader: None,
        reassigned: Vec::new(),
        direct_follower_count: 0,
        subject_count: 0,
        is_king: false,
        is_emperor: false,
        title: "Leader",
    };
    if dead_leader == 0 {
        return empty;
    }

    // Collect direct followers (living power map is the living filter).
    let mut directs: Vec<i32> = following
        .iter()
        .filter(|(&follower, &leader)| leader == dead_leader && follower != dead_leader)
        .filter(|(follower, _)| power.contains_key(follower))
        .map(|(&f, _)| f)
        .collect();
    directs.sort_unstable();
    let count = directs.len();
    if count == 0 {
        following.retain(|_, &mut l| l != dead_leader);
        return empty;
    }

    // Highest power; tie-break lower id.
    let mut best_leader: Option<i32> = None;
    let mut best_score = f32::NEG_INFINITY;
    for &p in &directs {
        let score = power.get(&p).copied().unwrap_or(f32::NEG_INFINITY);
        match best_leader {
            None => {
                best_score = score;
                best_leader = Some(p);
            }
            Some(cur) => {
                if score > best_score || ((score - best_score).abs() < 1e-6 && p < cur) {
                    best_score = score;
                    best_leader = Some(p);
                }
            }
        }
    }

    let Some(new_leader) = best_leader else {
        return empty;
    };

    // New leader follows who the dead leader followed (if any, and not self).
    following.remove(&dead_leader);
    match dead_leader_follow {
        Some(f) if f != 0 && f != new_leader && f != dead_leader => {
            following.insert(new_leader, f);
        }
        _ => {
            following.remove(&new_leader);
        }
    }

    let mut reassigned = Vec::new();
    for &p in &directs {
        if p == new_leader {
            continue;
        }
        // Skip if mutual exile.
        let exiled_by_new = exiles
            .get(&new_leader)
            .map(|s| s.contains(&p))
            .unwrap_or(false);
        let exiled_new = exiles
            .get(&p)
            .map(|s| s.contains(&new_leader))
            .unwrap_or(false);
        if exiled_by_new || exiled_new {
            // Leave follow as-is only if still pointing at dead — clear dead follow.
            if following.get(&p) == Some(&dead_leader) {
                following.remove(&p);
            }
            continue;
        }
        following.insert(p, new_leader);
        reassigned.push((p, new_leader));
    }
    // Haxe `count` is direct followers of dead, then count -= 1 (not counting new leader).
    let subjects_for_title = count.saturating_sub(1);
    let new_follows_none = !following.contains_key(&new_leader);
    let is_king = subjects_for_title > 4 && new_follows_none;
    let is_emperor = is_king && subjects_for_title > 14;
    let title = if is_emperor {
        "Emperor"
    } else if is_king {
        "King"
    } else {
        "Leader"
    };

    LeaderSuccession {
        new_leader: Some(new_leader),
        reassigned,
        direct_follower_count: count,
        subject_count: subjects_for_title,
        is_king,
        is_emperor,
        title,
    }
}

/// `LEADER_DIE <dead> <new> <title> n=<subjects>` event line.
pub fn format_leader_succession_event(dead: i32, succ: &LeaderSuccession) -> Option<String> {
    let new = succ.new_leader?;
    Some(format!(
        "LEADER_DIE {dead} {new} {} n={}",
        succ.title, succ.subject_count
    ))
}

/// Tag grave with deceased living owner + account soul key (Haxe account.graves push subset).
///
/// Uses `living_owners` for player id and `owners_by_account` with a stable
/// non-zero token from email (FNV-ish hash) so next-life rewire can find it.
pub fn stamp_grave_soul(grave: &mut ComplexObject, deceased_p_id: i32, email: &str) {
    if deceased_p_id != 0 && !grave.living_owners.contains(&deceased_p_id) {
        // Haxe still lists the dead as owner until InitObjectHelpersAfterRead.
        grave.living_owners.push(deceased_p_id);
    }
    if grave.owner_id == 0 {
        grave.owner_id = deceased_p_id;
    }
    let token = account_soul_token(email);
    if token != 0 && !grave.owners_by_account.contains(&token) {
        grave.owners_by_account.push(token);
    }
}

/// Stable non-zero i32 token from email (soul / account id proxy; not persisted OLA1 id).
pub fn account_soul_token(email: &str) -> i32 {
    let e = crate::accounts::normalize_email(email);
    if e.is_empty() {
        return 0;
    }
    let mut h: u32 = 2166136261;
    for b in e.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(16777619);
    }
    let v = (h & 0x7FFF_FFFF) as i32;
    if v == 0 {
        1
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::social::LineageNode;

    fn living_map(ids: &[i32]) -> HashMap<i32, bool> {
        ids.iter().map(|&i| (i, true)).collect()
    }

    #[test]
    fn residual_to_treasury_when_no_beneficiaries() {
        let mut accounts = AccountBook::default();
        accounts.on_spawn("eve@x", 1, "Eve");
        let mut economy = Economy::default();
        economy.add_coins(1, 10);
        let mut scoreboard = Scoreboard::default();
        scoreboard.ensure_player(1, "Eve");
        let social = SocialState::default();
        let allies = AllyState::default();
        let living = living_map(&[1]);
        let mut ctx = InheritContext {
            deceased_p_id: 1,
            deceased_email: "eve@x",
            living: &living,
            social: &social,
            allies: &allies,
            accounts: &mut accounts,
            economy: &mut economy,
            scoreboard: &mut scoreboard,
            grave: None,
            inherit_coins_factor: INHERIT_COINS_FACTOR,
        };
        let t = apply_inherit_coins(&mut ctx);
        assert_eq!(economy.coins_of(1), 0);
        assert_eq!(economy.treasury, 10);
        assert!(t.iter().any(|x| x.kind == "treasury" && x.amount == 10));
        assert!((accounts.get("eve@x").unwrap().coins_inherited - 8.0).abs() < 1e-4);
    }

    /// C-SS-MORE-BATCH4: live InheritCoinsFactor 0.5 vs default 0.8.
    // Haxe: ServerSettings.InheritCoinsFactor + InheritCoins L4030
    #[test]
    fn inherit_coins_factor_live_override() {
        let mut accounts = AccountBook::default();
        accounts.on_spawn("eve@x", 1, "Eve");
        let mut economy = Economy::default();
        economy.add_coins(1, 10);
        let mut scoreboard = Scoreboard::default();
        scoreboard.ensure_player(1, "Eve");
        let social = SocialState::default();
        let allies = AllyState::default();
        let living = living_map(&[1]);
        let mut ctx = InheritContext {
            deceased_p_id: 1,
            deceased_email: "eve@x",
            living: &living,
            social: &social,
            allies: &allies,
            accounts: &mut accounts,
            economy: &mut economy,
            scoreboard: &mut scoreboard,
            grave: None,
            inherit_coins_factor: 0.5,
        };
        let _ = apply_inherit_coins(&mut ctx);
        assert!((accounts.get("eve@x").unwrap().coins_inherited - 5.0).abs() < 1e-4);
    }

    #[test]
    fn residual_to_grave_when_no_kids() {
        let mut accounts = AccountBook::default();
        accounts.on_spawn("eve@x", 1, "Eve");
        let mut economy = Economy::default();
        economy.add_coins(1, 10);
        let mut scoreboard = Scoreboard::default();
        scoreboard.ensure_player(1, "Eve");
        let social = SocialState::default();
        let allies = AllyState::default();
        let living = living_map(&[]);
        let mut grave = ComplexObject::new_simple(87);
        let mut ctx = InheritContext {
            deceased_p_id: 1,
            deceased_email: "eve@x",
            living: &living,
            social: &social,
            allies: &allies,
            accounts: &mut accounts,
            economy: &mut economy,
            scoreboard: &mut scoreboard,
            grave: Some(&mut grave),
            inherit_coins_factor: INHERIT_COINS_FACTOR,
        };
        let t = apply_inherit_coins(&mut ctx);
        assert_eq!(economy.coins_of(1), 0);
        assert_eq!(economy.treasury, 0);
        assert!((grave.coins - 10.0).abs() < 1e-4);
        assert!(t.iter().any(|x| x.kind == "grave" && x.amount == 10));
    }

    #[test]
    fn children_split_remaining() {
        let mut accounts = AccountBook::default();
        accounts.on_spawn("mom@x", 1, "Mom");
        accounts.on_spawn("a@x", 2, "A");
        accounts.on_spawn("b@x", 3, "B");
        let mut economy = Economy::default();
        economy.add_coins(1, 10);
        let mut scoreboard = Scoreboard::default();
        scoreboard.ensure_player(1, "Mom");
        scoreboard.ensure_player(2, "A");
        scoreboard.ensure_player(3, "B");
        let mut social = SocialState::default();
        social.lineages.insert(1, LineageNode::eve(1, "Mom"));
        let mom = social.lineages.get(&1).unwrap().clone();
        social
            .lineages
            .insert(2, LineageNode::with_mother(2, "A", &mom));
        social
            .lineages
            .insert(3, LineageNode::with_mother(3, "B", &mom));
        let allies = AllyState::default();
        let living = living_map(&[2, 3]);
        let mut ctx = InheritContext {
            deceased_p_id: 1,
            deceased_email: "mom@x",
            living: &living,
            social: &social,
            allies: &allies,
            accounts: &mut accounts,
            economy: &mut economy,
            scoreboard: &mut scoreboard,
            grave: None,
            inherit_coins_factor: INHERIT_COINS_FACTOR,
        };
        let t = apply_inherit_coins(&mut ctx);
        assert_eq!(economy.coins_of(1), 0);
        assert_eq!(economy.coins_of(2), 5);
        assert_eq!(economy.coins_of(3), 5);
        assert_eq!(economy.treasury, 0);
        assert_eq!(t.iter().filter(|x| x.kind == "child").count(), 2);
    }

    #[test]
    fn past_actions_priority_before_children() {
        let mut accounts = AccountBook::default();
        accounts.on_spawn("dead@x", 1, "Dead");
        accounts.on_spawn("ally@x", 2, "Ally");
        accounts.on_spawn("kid@x", 3, "Kid");
        accounts.ensure("ally@x").coins_inherited = 7.0;
        let mut economy = Economy::default();
        economy.add_coins(1, 10);
        let mut scoreboard = Scoreboard::default();
        for (id, n) in [(1, "Dead"), (2, "Ally"), (3, "Kid")] {
            scoreboard.ensure_player(id, n);
        }
        let mut social = SocialState::default();
        social.lineages.insert(1, LineageNode::eve(1, "Dead"));
        let dead = social.lineages.get(&1).unwrap().clone();
        social
            .lineages
            .insert(3, LineageNode::with_mother(3, "Kid", &dead));
        let mut allies = AllyState::default();
        allies.add(1, 2).unwrap();
        let living = living_map(&[2, 3]);
        let mut ctx = InheritContext {
            deceased_p_id: 1,
            deceased_email: "dead@x",
            living: &living,
            social: &social,
            allies: &allies,
            accounts: &mut accounts,
            economy: &mut economy,
            scoreboard: &mut scoreboard,
            grave: None,
            inherit_coins_factor: INHERIT_COINS_FACTOR,
        };
        let t = apply_inherit_coins(&mut ctx);
        assert_eq!(economy.coins_of(2), 7);
        assert_eq!(economy.coins_of(3), 3);
        assert!(t.iter().any(|x| x.kind == "past_actions" && x.amount == 7));
        assert!(t.iter().any(|x| x.kind == "child" && x.amount == 3));
        assert!(accounts.get("ally@x").unwrap().coins_inherited < 1.0);
    }

    #[test]
    fn close_relative_double_score() {
        let mut accounts = AccountBook::default();
        accounts.on_spawn("dead@x", 1, "Dead");
        accounts.on_spawn("mom@x", 2, "Mom");
        accounts.ensure("mom@x").coins_inherited = 4.0;
        let mut economy = Economy::default();
        economy.add_coins(1, 5);
        let mut scoreboard = Scoreboard::default();
        scoreboard.ensure_player(1, "Dead");
        scoreboard.ensure_player(2, "Mom");
        let mut social = SocialState::default();
        social.lineages.insert(2, LineageNode::eve(2, "Mom"));
        let mom = social.lineages.get(&2).unwrap().clone();
        social
            .lineages
            .insert(1, LineageNode::with_mother(1, "Dead", &mom));
        let allies = AllyState::default();
        let living = living_map(&[2]);
        let mut ctx = InheritContext {
            deceased_p_id: 1,
            deceased_email: "dead@x",
            living: &living,
            social: &social,
            allies: &allies,
            accounts: &mut accounts,
            economy: &mut economy,
            scoreboard: &mut scoreboard,
            grave: None,
            inherit_coins_factor: INHERIT_COINS_FACTOR,
        };
        let t = apply_inherit_coins(&mut ctx);
        assert_eq!(economy.coins_of(2), 5);
        assert!(t.iter().any(|x| x.kind == "past_actions" && x.to_p_id == 2));
        assert!((accounts.get("mom@x").unwrap().coins_inherited - 1.5).abs() < 1e-4);
    }

    #[test]
    fn ownership_transfers_to_follow_leader() {
        let mut a = ComplexObject::with_owner(100, 1);
        let mut b = ComplexObject::with_owner(101, 1);
        add_owner_to_helper(&mut b, 9); // co-owned — stay with 9
        let mut helpers: Vec<((i32, i32), &mut ComplexObject)> =
            vec![((2, 3), &mut a), ((4, 5), &mut b)];
        let t = apply_inherit_ownership_on_helpers(&mut helpers, 1, Some(7));
        assert_eq!(a.owner_id, 7);
        assert!(a.living_owners.contains(&7));
        assert!(!a.living_owners.contains(&1));
        assert_eq!(b.owner_id, 9);
        assert!(b.living_owners.contains(&9));
        assert!(!b.living_owners.contains(&1));
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].new_owner, 7);
        assert_eq!(t[1].new_owner, 0); // still has owners
    }

    #[test]
    fn leadership_power_noble_serf() {
        let common = count_leadership_power(10.0, 10, 0.0, PrestigeClass::Commoner);
        let noble = count_leadership_power(10.0, 10, 0.0, PrestigeClass::Noble);
        let serf = count_leadership_power(10.0, 10, 0.0, PrestigeClass::Serf);
        assert!((common - 2.0).abs() < 1e-4);
        assert!((noble - 4.0).abs() < 1e-4);
        assert!((serf - 1.0).abs() < 1e-4);
    }

    #[test]
    fn leadership_power_family_prestige_adds() {
        // Haxe: power = (prestige + coins + familyPrestige) / 10
        let base = count_leadership_power(0.0, 0, 0.0, PrestigeClass::Commoner);
        let with_fam = count_leadership_power(0.0, 0, 50.0, PrestigeClass::Commoner);
        assert!((base - 0.0).abs() < 1e-4);
        assert!((with_fam - 5.0).abs() < 1e-4);
    }

    #[test]
    fn choose_new_leader_reassigns_followers() {
        let mut following = HashMap::new();
        following.insert(2, 1);
        following.insert(3, 1);
        following.insert(4, 1);
        following.insert(1, 99); // dead followed 99
        let mut power = HashMap::new();
        power.insert(2, 1.0);
        power.insert(3, 5.0); // best
        power.insert(4, 2.0);
        let exiles = HashMap::new();
        let succ = choose_new_leader(1, Some(99), &mut following, &exiles, &power);
        assert_eq!(succ.new_leader, Some(3));
        assert_eq!(following.get(&3), Some(&99));
        assert_eq!(following.get(&2), Some(&3));
        assert_eq!(following.get(&4), Some(&3));
        assert!(!following.contains_key(&1));
        assert_eq!(succ.subject_count, 2);
        assert_eq!(succ.title, "Leader");
        assert!(format_leader_succession_event(1, &succ)
            .unwrap()
            .contains("LEADER_DIE 1 3"));
    }

    #[test]
    fn choose_new_leader_skips_exile() {
        let mut following = HashMap::new();
        following.insert(2, 1);
        following.insert(3, 1);
        let mut power = HashMap::new();
        power.insert(2, 10.0);
        power.insert(3, 1.0);
        let mut exiles: HashMap<i32, HashSet<i32>> = HashMap::new();
        exiles.entry(2).or_default().insert(3); // new leader 2 exiled 3
        let succ = choose_new_leader(1, None, &mut following, &exiles, &power);
        assert_eq!(succ.new_leader, Some(2));
        assert!(!following.contains_key(&3)); // cleared, not reassigned
        assert_eq!(succ.reassigned.len(), 0);
    }

    #[test]
    fn stamp_grave_soul_sets_account_token() {
        let mut g = ComplexObject::new_simple(87);
        stamp_grave_soul(&mut g, 42, "Hero@X.com");
        assert!(g.living_owners.contains(&42));
        assert_eq!(g.owner_id, 42);
        assert_eq!(g.owners_by_account.len(), 1);
        assert_eq!(g.owners_by_account[0], account_soul_token("hero@x.com"));
    }
}
