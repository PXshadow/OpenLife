//! Haxe `GlobalPlayerInstance.addHealthAndPrestige` pure helpers + fan math.
//!
//! Self always receives full `count` (prestige / yum). Positive counts also:
//! - add coins
//! - fan to parents / grandparents / children / one sibling / follow leaders
//!   scaled by clothing prestige factors.
//!
//! Clothing factor residual: ObjectData.prestigeFactor defaults to 0.5 per worn
//! non-zero clothing slot (content has no prestigeFactor field yet). Eve/Adam
//! (no mother) gets Haxe `/2` then `+0.5`.

/// Default Haxe `ObjectData.prestigeFactor` when content does not override.
// Haxe: ObjectData.prestigeFactor = 0.5
pub const DEFAULT_CLOTHING_PRESTIGE_FACTOR: f32 = 0.5;

/// Max follow-chain leaders that receive prestige (Haxe `for (ii in 0...4)`).
// Haxe: GlobalPlayerInstance.addHealthAndPrestige L6082
pub const PRESTIGE_LEADER_CHAIN_DEPTH: usize = 4;

/// Pure clothing prestige factor from worn parent ids.
///
/// Each non-zero slot contributes `slot_factor` (default 0.5). Eve/Adam halves
/// then adds 0.5.
// Haxe: GlobalPlayerInstance.calculateClothingPrestigeFactor L4289–4305
pub fn clothing_prestige_factor(clothing_ids: &[i32], is_eve_or_adam: bool) -> f32 {
    clothing_prestige_factor_ex(clothing_ids, is_eve_or_adam, DEFAULT_CLOTHING_PRESTIGE_FACTOR)
}

/// Like [`clothing_prestige_factor`] with explicit per-slot base factor.
pub fn clothing_prestige_factor_ex(
    clothing_ids: &[i32],
    is_eve_or_adam: bool,
    per_slot: f32,
) -> f32 {
    let base = if per_slot.is_finite() && per_slot >= 0.0 {
        per_slot
    } else {
        DEFAULT_CLOTHING_PRESTIGE_FACTOR
    };
    let mut factor = 0.0_f32;
    for &id in clothing_ids {
        if id > 0 {
            factor += base;
        }
    }
    if is_eve_or_adam {
        factor /= 2.0;
        factor += 0.5;
    }
    factor
}

/// Haxe `calculateTotalClothingPrestigeFactor` — average of giver + receiver.
// Haxe: GlobalPlayerInstance.calculateTotalClothingPrestigeFactor L6105–6108
#[inline]
pub fn total_clothing_prestige_factor(giver_factor: f32, receiver_factor: f32) -> f32 {
    let g = if giver_factor.is_finite() {
        giver_factor
    } else {
        0.0
    };
    let r = if receiver_factor.is_finite() {
        receiver_factor
    } else {
        0.0
    };
    (g + r) / 2.0
}

/// One prestige delta applied to a relative or leader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrestigeFanDelta {
    pub p_id: i32,
    pub prestige: f32,
    /// Leaders also receive coins (Haxe `leader.coins += tmpCount * leaderFactor`).
    pub coins: f32,
    /// Bucket for diagnostics / future PLB fields.
    pub kind: PrestigeFanKind,
}

/// Which relative/leader bucket received the fan share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrestigeFanKind {
    Parent,
    Grandparent,
    Child,
    Sibling,
    Leader,
}

/// Pure family + leader fan for a **positive** prestige count.
///
/// Self full amount is applied separately. Returns only **other** recipients.
///
/// Shares (Haxe L6018–6100):
/// - mother / father: `count * total_factor / 4` each
/// - each grandparent: same `/4`
/// - each living child: `count * total_factor / 2`
/// - one sibling (first other child of mother): `/2`
/// - up to 4 leaders on follow chain: each gets `count/4 * leader_factor`
// Haxe: GlobalPlayerInstance.addHealthAndPrestige L6007–6101
pub fn prestige_fan_deltas(
    count: f32,
    mother: Option<(i32, f32)>,
    father: Option<(i32, f32)>,
    grandparents: &[(i32, f32)],
    children: &[(i32, f32)],
    sibling: Option<(i32, f32)>,
    // (leader_id, clothing_factor, leader_extra, same_family, is_cursed, is_exiled)
    leaders: &[(i32, f32, f32, bool, bool, bool)],
    giver_clothing_factor: f32,
) -> Vec<PrestigeFanDelta> {
    let mut out = Vec::new();
    prestige_fan_deltas_ex(
        count,
        mother,
        father,
        grandparents,
        children,
        sibling,
        leaders,
        giver_clothing_factor,
        &mut out,
    );
    out
}

/// In-place variant of [`prestige_fan_deltas`].
// Haxe: GlobalPlayerInstance.addHealthAndPrestige L6018–6101
pub fn prestige_fan_deltas_ex(
    count: f32,
    mother: Option<(i32, f32)>,
    father: Option<(i32, f32)>,
    grandparents: &[(i32, f32)],
    children: &[(i32, f32)],
    sibling: Option<(i32, f32)>,
    leaders: &[(i32, f32, f32, bool, bool, bool)],
    giver_clothing_factor: f32,
    out: &mut Vec<PrestigeFanDelta>,
) {
    out.clear();
    if !count.is_finite() || count <= 0.0 {
        return;
    }
    let tmp = count;

    let push_rel = |out: &mut Vec<PrestigeFanDelta>, id: i32, recv_f: f32, div: f32, kind: PrestigeFanKind| {
        if id <= 0 {
            return;
        }
        let cf = total_clothing_prestige_factor(giver_clothing_factor, recv_f);
        let d = (tmp * cf) / div;
        if d.is_finite() && d != 0.0 {
            out.push(PrestigeFanDelta {
                p_id: id,
                prestige: d,
                coins: 0.0,
                kind,
            });
        }
    };

    if let Some((id, f)) = mother {
        push_rel(out, id, f, 4.0, PrestigeFanKind::Parent);
    }
    for &(id, f) in grandparents.iter().take(2) {
        // mother.mother / mother.father — caller orders; all grandparents /4
        push_rel(out, id, f, 4.0, PrestigeFanKind::Grandparent);
    }
    // If grandparents has more than 2, remaining are father-side (still /4)
    for &(id, f) in grandparents.iter().skip(2) {
        push_rel(out, id, f, 4.0, PrestigeFanKind::Grandparent);
    }
    if let Some((id, f)) = father {
        push_rel(out, id, f, 4.0, PrestigeFanKind::Parent);
    }

    for &(id, f) in children {
        push_rel(out, id, f, 2.0, PrestigeFanKind::Child);
    }
    if let Some((id, f)) = sibling {
        // Haxe only if mother != null && children.length > 0
        if mother.is_some() && !children.is_empty() {
            push_rel(out, id, f, 2.0, PrestigeFanKind::Sibling);
        }
    }

    // Leaders: tmpCount starts at count/4
    let mut leader_tmp = count / 4.0;
    for &(lid, cloth_f, extra, same_family, is_cursed, is_exiled) in leaders
        .iter()
        .take(PRESTIGE_LEADER_CHAIN_DEPTH)
    {
        if lid <= 0 || is_exiled {
            break;
        }
        // take half from leader half from follower
        let mut leader_factor = total_clothing_prestige_factor(giver_clothing_factor, cloth_f);
        leader_factor += if extra.is_finite() { extra } else { 0.0 };
        if !same_family {
            leader_factor *= 0.5;
        }
        if is_cursed {
            leader_tmp /= 2.0;
        }
        let d = leader_tmp * leader_factor;
        if d.is_finite() && d != 0.0 {
            out.push(PrestigeFanDelta {
                p_id: lid,
                prestige: d,
                coins: d,
                kind: PrestigeFanKind::Leader,
            });
        }
    }
}

/// Coins to add on positive food prestige (Haxe `this.coins += count` as Float).
/// Rust wallets are i32 — floor non-negative count.
// Haxe: GlobalPlayerInstance.addHealthAndPrestige L6008
#[inline]
pub fn coins_from_prestige_count(count: f32) -> i32 {
    if count.is_finite() && count > 0.0 {
        count.floor() as i32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clothing_factor_empty_and_worn() {
        assert!((clothing_prestige_factor(&[], false) - 0.0).abs() < 1e-6);
        assert!((clothing_prestige_factor(&[100, 0, 200], false) - 1.0).abs() < 1e-6);
        // Eve: (1.0)/2 + 0.5 = 1.0
        assert!((clothing_prestige_factor(&[100, 200], true) - 1.0).abs() < 1e-6);
        // Eve empty: 0/2 + 0.5 = 0.5
        assert!((clothing_prestige_factor(&[], true) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn total_factor_averages() {
        assert!((total_clothing_prestige_factor(1.0, 0.0) - 0.5).abs() < 1e-6);
        assert!((total_clothing_prestige_factor(0.0, 0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn parent_fan_shares_quarter() {
        // giver 0, mother 0 → total 0 → no fan
        let mut out = Vec::new();
        prestige_fan_deltas_ex(
            4.0,
            Some((10, 0.0)),
            None,
            &[],
            &[],
            None,
            &[],
            0.0,
            &mut out,
        );
        assert!(out.is_empty());

        // giver 1.0, mother 1.0 → total 1.0 → mother gets 4*1/4 = 1.0
        prestige_fan_deltas_ex(
            4.0,
            Some((10, 1.0)),
            None,
            &[],
            &[],
            None,
            &[],
            1.0,
            &mut out,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].p_id, 10);
        assert!((out[0].prestige - 1.0).abs() < 1e-5);
        assert_eq!(out[0].kind, PrestigeFanKind::Parent);
    }

    #[test]
    fn child_fan_half() {
        let mut out = Vec::new();
        prestige_fan_deltas_ex(
            4.0,
            None,
            None,
            &[],
            &[(20, 1.0)],
            None,
            &[],
            1.0,
            &mut out,
        );
        // count * 1 / 2 = 2
        assert_eq!(out.len(), 1);
        assert!((out[0].prestige - 2.0).abs() < 1e-5);
        assert_eq!(out[0].kind, PrestigeFanKind::Child);
    }

    #[test]
    fn leader_fan_with_coins() {
        let mut out = Vec::new();
        // count=4 → leader_tmp=1; factor = (1+1)/2 + 0 = 1 → d=1
        prestige_fan_deltas_ex(
            4.0,
            None,
            None,
            &[],
            &[],
            None,
            &[(99, 1.0, 0.0, true, false, false)],
            1.0,
            &mut out,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].p_id, 99);
        assert!((out[0].prestige - 1.0).abs() < 1e-5);
        assert!((out[0].coins - 1.0).abs() < 1e-5);
        assert_eq!(out[0].kind, PrestigeFanKind::Leader);
    }

    #[test]
    fn exile_stops_leader_chain() {
        let mut out = Vec::new();
        prestige_fan_deltas_ex(
            4.0,
            None,
            None,
            &[],
            &[],
            None,
            &[
                (1, 1.0, 0.0, true, false, true), // exiled → break
                (2, 1.0, 0.0, true, false, false),
            ],
            1.0,
            &mut out,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn coins_floor_positive() {
        assert_eq!(coins_from_prestige_count(3.7), 3);
        assert_eq!(coins_from_prestige_count(0.0), 0);
        assert_eq!(coins_from_prestige_count(-1.0), 0);
    }

    #[test]
    fn negative_count_no_fan() {
        let mut out = Vec::new();
        prestige_fan_deltas_ex(
            -2.0,
            Some((1, 1.0)),
            None,
            &[],
            &[(2, 1.0)],
            None,
            &[(3, 1.0, 0.0, true, false, false)],
            1.0,
            &mut out,
        );
        assert!(out.is_empty());
    }
}
