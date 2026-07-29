//! Haxe `ObjectHelper.InitObjectHelpersAfterRead` pure core (**NESTED-OLW1-POLISH**).
//!
//! After OLW load, rewire grave → account.graves and owned objects → player.owning;
//! prune living_owners that are missing or deleted.
//!
//! Callers supply living-player / account predicates; this module does not touch
//! social books or account maps directly.
//!
//! ## Haxe parity notes
//! - `isOwned()` requires description `+owned` (not mere presence of living_owners).
//! - Grave path only links `ownersByPlayerAccount`; it does **not** prune `livingOwners`.
//! - Deleted living owner → `removeOwner`: drop p_id from livingOwners **and** account id
//!   from `ownersByPlayerAccount` (caller supplies account tokens for deleted p_ids).
//! - Missing living owner → only drop from `livingOwners` (account list kept).

use crate::ComplexObject;

/// One grave ↔ account link produced after load (Haxe `account.graves.push(obj)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraveAccountLink {
    pub account_id: i32,
    pub tx: i32,
    pub ty: i32,
    pub base_id: i32,
}

/// One player ↔ owned-object link (Haxe `player.owning.push(obj)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerOwningLink {
    pub player_id: i32,
    pub tx: i32,
    pub ty: i32,
    pub base_id: i32,
}

/// Creator lineage id that should get `ownsObject = true` (Haxe `getLinage` / first living owner).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineageOwnsLink {
    pub creator_player_id: i32,
    pub tx: i32,
    pub ty: i32,
    pub base_id: i32,
}

/// Result of scanning one complex helper for post-load owner rewire.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PostloadHelperResult {
    /// Account ids whose graves list should include this tile (grave only).
    pub grave_accounts: Vec<i32>,
    /// Living player ids that still own this object after prune.
    pub living_owners_kept: Vec<i32>,
    /// Owner ids removed (missing or deleted).
    pub living_owners_removed: Vec<i32>,
    /// Deleted (not merely missing) p_ids — Haxe `removeOwner` path.
    pub living_owners_deleted: Vec<i32>,
    /// True when `living_owners` or `owners_by_account` was mutated.
    pub owners_changed: bool,
}

/// Presence of a living-owner id for Haxe prune vs removeOwner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivingOwnerStatus {
    /// Player exists and is not deleted — keep.
    Alive,
    /// Player exists and is deleted — Haxe `removeOwner` (strip living + account).
    Deleted,
    /// No player loaded / unknown id — Haxe only drops from livingOwners.
    Missing,
    /// Cold-boot keep: treat as alive so disk owners survive empty player map.
    Keep,
}

/// Haxe grave name/description heuristic (content may also flag graves).
pub fn name_looks_like_grave(name: &str, description: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let d = description.to_ascii_lowercase();
    n.contains("grave")
        || n.contains("bone pile")
        || n.contains("skull")
        || d.contains("grave")
        || d.contains("bone pile")
}

/// Haxe `ObjectHelper.isOwned` — description contains `+owned`.
#[inline]
pub fn description_is_owned(description: &str) -> bool {
    description.to_ascii_lowercase().contains("+owned")
}

/// True when helper is owned for post-load rewire (Haxe `isOwned()` on description).
///
/// Does **not** use living_owners alone — content without `+owned` is a no-op path
/// even if disk lists still have owners.
#[inline]
pub fn helper_is_owned(description: &str) -> bool {
    description_is_owned(description)
}

/// True when helper has any owner lists (disk signal; not Haxe isOwned gate).
#[inline]
pub fn helper_has_owner_lists(co: &ComplexObject) -> bool {
    co.owner_id != 0 || !co.living_owners.is_empty()
}

/// True when helper is a grave for post-load account wiring.
///
/// Prefer content `is_grave` when available; else name heuristic + account owners.
pub fn helper_is_grave(co: &ComplexObject, name: &str, description: &str, content_is_grave: bool) -> bool {
    if content_is_grave {
        return true;
    }
    if !co.owners_by_account.is_empty() && name_looks_like_grave(name, description) {
        return true;
    }
    name_looks_like_grave(name, description)
}

/// Haxe living-owner prune for one helper (owned path).
///
/// `status(p_id)` — Alive/Keep keep; Missing drop living only; Deleted drop + flag removeOwner.
pub fn rewire_living_owners_status(
    living_owners: &[i32],
    mut status: impl FnMut(i32) -> LivingOwnerStatus,
) -> PostloadHelperResult {
    let mut out = PostloadHelperResult::default();
    for &id in living_owners {
        if id == 0 {
            out.living_owners_removed.push(id);
            out.owners_changed = true;
            continue;
        }
        match status(id) {
            LivingOwnerStatus::Alive | LivingOwnerStatus::Keep => {
                out.living_owners_kept.push(id);
            }
            LivingOwnerStatus::Deleted => {
                out.living_owners_removed.push(id);
                out.living_owners_deleted.push(id);
                out.owners_changed = true;
            }
            LivingOwnerStatus::Missing => {
                out.living_owners_removed.push(id);
                out.owners_changed = true;
            }
        }
    }
    if out.living_owners_kept != living_owners {
        out.owners_changed = true;
    }
    out
}

/// Backward-compatible prune: `player_alive` true → keep, false → Missing (not Deleted).
pub fn rewire_living_owners(
    living_owners: &[i32],
    mut player_alive: impl FnMut(i32) -> bool,
) -> PostloadHelperResult {
    rewire_living_owners_status(living_owners, |id| {
        if player_alive(id) {
            LivingOwnerStatus::Alive
        } else {
            LivingOwnerStatus::Missing
        }
    })
}

/// Strip account tokens associated with deleted living owners (Haxe `removeOwner`).
///
/// `account_token_of(p_id)` returns the soul/account id stored in `owners_by_account`.
pub fn strip_account_owners_for_deleted(
    owners_by_account: &mut Vec<i32>,
    deleted_player_ids: &[i32],
    mut account_token_of: impl FnMut(i32) -> Option<i32>,
) -> bool {
    if deleted_player_ids.is_empty() || owners_by_account.is_empty() {
        return false;
    }
    let mut tokens = Vec::new();
    for &pid in deleted_player_ids {
        if let Some(tok) = account_token_of(pid) {
            if tok != 0 {
                tokens.push(tok);
            }
        }
    }
    if tokens.is_empty() {
        return false;
    }
    let before = owners_by_account.len();
    owners_by_account.retain(|a| !tokens.contains(a));
    owners_by_account.len() != before
}

/// Creator p_id for lineage ownsObject (Haxe `getLinage` ← `livingOwners[0]`).
#[inline]
pub fn helper_creator_player_id(co: &ComplexObject) -> Option<i32> {
    if let Some(&id) = co.living_owners.first() {
        if id != 0 {
            return Some(id);
        }
    }
    if co.owner_id != 0 {
        return Some(co.owner_id);
    }
    None
}

/// Apply rewire to a mutable [`ComplexObject`]: prune dead living owners on **owned** path.
///
/// Returns grave account ids (if grave), player owning links for living owners kept,
/// and optional creator lineage link.
///
/// `account_token_of` used only when a living owner is **Deleted** (removeOwner).
pub fn apply_helper_postload(
    co: &mut ComplexObject,
    tx: i32,
    ty: i32,
    name: &str,
    description: &str,
    content_is_grave: bool,
    mut player_status: impl FnMut(i32) -> LivingOwnerStatus,
    mut account_token_of: impl FnMut(i32) -> Option<i32>,
) -> (
    Vec<GraveAccountLink>,
    Vec<PlayerOwningLink>,
    Option<LineageOwnsLink>,
    PostloadHelperResult,
) {
    let is_grave = helper_is_grave(co, name, description, content_is_grave);
    let mut graves = Vec::new();
    let mut owning = Vec::new();
    let lineage = helper_creator_player_id(co).map(|creator_player_id| LineageOwnsLink {
        creator_player_id,
        tx,
        ty,
        base_id: co.base_id,
    });

    if is_grave {
        // Haxe: grave path only account.graves + ownsObject; no livingOwners prune.
        for &aid in &co.owners_by_account {
            if aid != 0 {
                graves.push(GraveAccountLink {
                    account_id: aid,
                    tx,
                    ty,
                    base_id: co.base_id,
                });
            }
        }
        return (graves, owning, lineage, PostloadHelperResult::default());
    }

    // Haxe: else if (obj.isOwned()) — description +owned
    if !helper_is_owned(description) {
        // Still report creator lineage for non-owned map objects (Haxe first block).
        return (graves, owning, lineage, PostloadHelperResult::default());
    }

    // Ensure primary owner is in living list for prune scan
    let mut scan = co.living_owners.clone();
    if co.owner_id != 0 && !scan.contains(&co.owner_id) {
        scan.insert(0, co.owner_id);
    }
    let mut result = rewire_living_owners_status(&scan, &mut player_status);
    if result.owners_changed {
        co.living_owners = result.living_owners_kept.clone();
        if co.owner_id != 0 && !co.living_owners.contains(&co.owner_id) {
            co.owner_id = co.living_owners.first().copied().unwrap_or(0);
        } else if co.owner_id == 0 {
            co.owner_id = co.living_owners.first().copied().unwrap_or(0);
        }
        // Haxe removeOwner for deleted: also strip ownersByPlayerAccount
        if strip_account_owners_for_deleted(
            &mut co.owners_by_account,
            &result.living_owners_deleted,
            &mut account_token_of,
        ) {
            result.owners_changed = true;
        }
    }
    for &pid in &result.living_owners_kept {
        owning.push(PlayerOwningLink {
            player_id: pid,
            tx,
            ty,
            base_id: co.base_id,
        });
    }
    // Creator lineage after prune (first remaining living owner / owner_id)
    let lineage = helper_creator_player_id(co).map(|creator_player_id| LineageOwnsLink {
        creator_player_id,
        tx,
        ty,
        base_id: co.base_id,
    });
    (graves, owning, lineage, result)
}

/// Convenience: bool alive callback (Missing when false) + no account strip tokens.
pub fn apply_helper_postload_simple(
    co: &mut ComplexObject,
    tx: i32,
    ty: i32,
    name: &str,
    description: &str,
    content_is_grave: bool,
    mut player_alive: impl FnMut(i32) -> bool,
) -> (
    Vec<GraveAccountLink>,
    Vec<PlayerOwningLink>,
    Option<LineageOwnsLink>,
    PostloadHelperResult,
) {
    apply_helper_postload(
        co,
        tx,
        ty,
        name,
        description,
        content_is_grave,
        |id| {
            if player_alive(id) {
                LivingOwnerStatus::Alive
            } else {
                LivingOwnerStatus::Missing
            }
        },
        |_| None,
    )
}

/// Batch scan: all complex helpers on a world slice.
///
/// `helpers` are `(tx, ty, ComplexObject)` snapshots; mutates in place when pruning.
///
/// Returns (graves, owning, lineage_creator_ids to mark ownsObject).
pub fn init_object_helpers_after_read(
    helpers: &mut [(i32, i32, ComplexObject)],
    name_of: impl Fn(i32) -> (String, String, bool),
    mut player_status: impl FnMut(i32) -> LivingOwnerStatus,
    mut account_token_of: impl FnMut(i32) -> Option<i32>,
) -> (
    Vec<GraveAccountLink>,
    Vec<PlayerOwningLink>,
    Vec<LineageOwnsLink>,
) {
    let mut all_graves = Vec::new();
    let mut all_owning = Vec::new();
    let mut all_lineage = Vec::new();
    for (tx, ty, co) in helpers.iter_mut() {
        if co.base_id == 0 {
            continue;
        }
        let (name, desc, is_grave_flag) = name_of(co.base_id);
        let (g, o, lin, _) = apply_helper_postload(
            co,
            *tx,
            *ty,
            &name,
            &desc,
            is_grave_flag,
            &mut player_status,
            &mut account_token_of,
        );
        all_graves.extend(g);
        all_owning.extend(o);
        if let Some(l) = lin {
            all_lineage.push(l);
        }
    }
    (all_graves, all_owning, all_lineage)
}

/// Batch with bool alive only (Missing when false; no account strip).
pub fn init_object_helpers_after_read_simple(
    helpers: &mut [(i32, i32, ComplexObject)],
    name_of: impl Fn(i32) -> (String, String, bool),
    mut player_alive: impl FnMut(i32) -> bool,
) -> (
    Vec<GraveAccountLink>,
    Vec<PlayerOwningLink>,
    Vec<LineageOwnsLink>,
) {
    init_object_helpers_after_read(
        helpers,
        name_of,
        |id| {
            if player_alive(id) {
                LivingOwnerStatus::Alive
            } else {
                LivingOwnerStatus::Missing
            }
        },
        |_| None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(id: i32, owners: &[i32]) -> ComplexObject {
        let mut co = ComplexObject::new_simple(id);
        co.living_owners = owners.to_vec();
        if let Some(&first) = owners.first() {
            co.owner_id = first;
        }
        co
    }

    #[test]
    fn name_looks_like_grave_detects() {
        assert!(name_looks_like_grave("Bone Pile", ""));
        assert!(name_looks_like_grave("x", "Marked Grave"));
        assert!(!name_looks_like_grave("Basket", "woven"));
    }

    #[test]
    fn description_owned_gate() {
        assert!(description_is_owned("Chest +owned"));
        assert!(description_is_owned("x +OWNED y"));
        assert!(!description_is_owned("Chest"));
        assert!(!helper_is_owned("Basket"));
    }

    #[test]
    fn rewire_prunes_missing_and_deleted() {
        let r = rewire_living_owners_status(&[1, 2, 3], |id| match id {
            1 | 3 => LivingOwnerStatus::Alive,
            2 => LivingOwnerStatus::Deleted,
            _ => LivingOwnerStatus::Missing,
        });
        assert_eq!(r.living_owners_kept, vec![1, 3]);
        assert_eq!(r.living_owners_removed, vec![2]);
        assert_eq!(r.living_owners_deleted, vec![2]);
        assert!(r.owners_changed);
    }

    #[test]
    fn rewire_keeps_all_when_alive() {
        let r = rewire_living_owners(&[5, 6], |_| true);
        assert_eq!(r.living_owners_kept, vec![5, 6]);
        assert!(!r.owners_changed || r.living_owners_removed.is_empty());
    }

    #[test]
    fn apply_grave_links_accounts_no_prune() {
        // Haxe: grave path does not touch livingOwners.
        let mut co = ComplexObject::new_simple(87);
        co.owners_by_account = vec![10, 20];
        co.living_owners = vec![99];
        co.owner_id = 99;
        co.text = "grave".into();
        let (g, o, lin, r) = apply_helper_postload(
            &mut co,
            3,
            4,
            "Grave",
            "+origGrave",
            true,
            |_| LivingOwnerStatus::Missing,
            |_| None,
        );
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].account_id, 10);
        assert_eq!(g[0].tx, 3);
        assert!(o.is_empty());
        assert_eq!(co.living_owners, vec![99], "graves must not prune living_owners");
        assert_eq!(lin.map(|l| l.creator_player_id), Some(99));
        assert!(!r.owners_changed);
    }

    #[test]
    fn apply_owned_prunes_and_links_players() {
        let mut co = owned(100, &[1, 2, 3]);
        co.owners_by_account = vec![1001, 1002];
        let (g, o, _, r) = apply_helper_postload(
            &mut co,
            1,
            2,
            "Chest",
            "Chest +owned",
            false,
            |id| {
                if id == 2 {
                    LivingOwnerStatus::Deleted
                } else if id == 1 || id == 3 {
                    LivingOwnerStatus::Alive
                } else {
                    LivingOwnerStatus::Missing
                }
            },
            |id| if id == 2 { Some(1002) } else { None },
        );
        assert!(g.is_empty());
        assert_eq!(co.living_owners, vec![1, 3]);
        assert_eq!(o.len(), 2);
        assert!(r.owners_changed);
        assert_eq!(co.owner_id, 1);
        // Deleted player 2 → removeOwner strips account token 1002
        assert_eq!(co.owners_by_account, vec![1001]);
        assert_eq!(r.living_owners_deleted, vec![2]);
    }

    #[test]
    fn without_owned_tag_no_rewire() {
        let mut co = owned(100, &[1, 2]);
        let (g, o, lin, r) = apply_helper_postload(
            &mut co,
            1,
            1,
            "Chest",
            "Chest", // no +owned
            false,
            |_| LivingOwnerStatus::Missing,
            |_| None,
        );
        assert!(g.is_empty());
        assert!(o.is_empty());
        assert_eq!(co.living_owners, vec![1, 2], "non-+owned must not prune");
        assert!(!r.owners_changed);
        assert_eq!(lin.map(|l| l.creator_player_id), Some(1));
    }

    #[test]
    fn batch_init_after_read() {
        let mut helpers = vec![
            (0, 0, owned(50, &[1, 9])),
            (1, 1, {
                let mut g = ComplexObject::new_simple(87);
                g.owners_by_account = vec![42];
                g.living_owners = vec![7];
                g
            }),
        ];
        let (graves, owning, lineage) = init_object_helpers_after_read(
            &mut helpers,
            |id| {
                if id == 87 {
                    ("Grave".into(), "+origGrave".into(), true)
                } else {
                    ("Box +owned".into(), "Box +owned".into(), false)
                }
            },
            |pid| {
                if pid == 1 {
                    LivingOwnerStatus::Alive
                } else {
                    LivingOwnerStatus::Missing
                }
            },
            |_| None,
        );
        assert_eq!(graves.len(), 1);
        assert_eq!(graves[0].account_id, 42);
        assert_eq!(owning.len(), 1);
        assert_eq!(owning[0].player_id, 1);
        assert_eq!(helpers[0].2.living_owners, vec![1]);
        // Grave keeps living owner 7
        assert_eq!(helpers[1].2.living_owners, vec![7]);
        assert!(lineage.iter().any(|l| l.creator_player_id == 1));
        assert!(lineage.iter().any(|l| l.creator_player_id == 7));
    }
}
