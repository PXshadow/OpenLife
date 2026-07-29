//! AI takes over a human body on disconnect; reclaim on reconnect.
//!
//! Haxe: `Connection.close` attaches `ServerAi` when the player is still alive;
//! `Connection.isAi()` is `sock == null`; `rloginHelper` deactivates AI and
//! rebinds the living body; `ServerAi.doRebirth` drops non-`account.isAi`
//! replacement AIs instead of respawning them.
//!
//! // Haxe: Connection.close / rloginHelper / ServerAi (AI-TAKEOVER / disconnect_ai)

use crate::player_soul::email_looks_ai;

/// Result of applying disconnect AI-takeover policy to one body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeoverAttachResult {
    /// Body still alive → AI now controls it (`ai_controlled=true`, `connected=false`).
    Attached,
    /// Body already dead/deleted → no AI attach (`ai_controlled=false`, `connected=false`).
    SkippedDeleted,
}

/// Whether disconnect should attach an AI controller (Haxe: `player.deleted == false`).
// Haxe: Connection.close `if (this.player.deleted == false) this.serverAi = new ServerAi(...)`
pub fn should_attach_ai_on_disconnect(deleted: bool) -> bool {
    !deleted
}

/// Apply disconnect → AI takeover flags.
///
/// Sets `connected = false`. When not deleted, sets `ai_controlled = true`
/// (Haxe `serverAi = new ServerAi(player)`). Deleted bodies clear takeover.
// Haxe: Connection.close
pub fn attach_ai_takeover(
    connected: &mut bool,
    ai_controlled: &mut bool,
    deleted: bool,
) -> TakeoverAttachResult {
    *connected = false;
    if should_attach_ai_on_disconnect(deleted) {
        *ai_controlled = true;
        TakeoverAttachResult::Attached
    } else {
        *ai_controlled = false;
        TakeoverAttachResult::SkippedDeleted
    }
}

/// Human reclaim: drop AI controller and mark body online again.
// Haxe: rloginHelper ais.remove + serverAi = null; sock becomes non-null
pub fn release_ai_takeover(connected: &mut bool, ai_controlled: &mut bool) {
    *ai_controlled = false;
    *connected = true;
}

/// Clear takeover when the body dies (no human-replacement rebirth).
// Haxe: ServerAi.doRebirth when `account.isAi == false` → removeAi
pub fn clear_ai_on_death(ai_controlled: &mut bool) {
    *ai_controlled = false;
}

/// Haxe `ServerAi.doRebirth`: permanent AI accounts may respawn; human-replacement AIs must not.
// Haxe: ServerAi.doRebirth `if (this.player.account.isAi == false) { removeAi; return; }`
pub fn should_respawn_ai_after_death(account_is_permanent_ai: bool) -> bool {
    account_is_permanent_ai
}

/// Permanent AI account heuristic (spawned NPCs / selfplay / `ai@` emails).
///
/// Human disconnect takeover keeps the original human email — those are **not**
/// permanent AI accounts, so death removes the controller without AI rebirth.
// Haxe: PlayerAccount.isAi
pub fn account_is_permanent_ai(email: &str) -> bool {
    email_looks_ai(email)
}

/// Haxe `Connection.isAi()` / `GlobalPlayerInstance.isAi()` for live bodies.
///
/// True when:
/// - AI currently controls the body (`ai_controlled`), or
/// - no human client (`!connected` — Haxe `sock == null`), or
/// - permanent AI account email.
// Haxe: Connection.isAi → sock == null; account.isAi for pure NPCs
pub fn player_is_ai(connected: bool, ai_controlled: bool, email: &str) -> bool {
    ai_controlled || !connected || account_is_permanent_ai(email)
}

/// Inverse of [`player_is_ai`] (Haxe `isHuman`).
pub fn player_is_human(connected: bool, ai_controlled: bool, email: &str) -> bool {
    !player_is_ai(connected, ai_controlled, email)
}

/// Normalize email for reconnect match (trim + lowercase).
pub fn normalize_reconnect_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

/// True if this living body can be reclaimed by a reconnecting client with `email`.
///
/// Requires non-deleted body, matching email, and either AI takeover or disconnected
/// (same life still in the world). Already-connected humans with a live client are
/// not stolen (another session still owns the socket).
// Haxe: PlayerAccount.getLastLivingPlayer + rlogin reclaim
pub fn body_eligible_for_reconnect(
    deleted: bool,
    connected: bool,
    ai_controlled: bool,
    body_email: &str,
    login_email: &str,
) -> bool {
    if deleted {
        return false;
    }
    if normalize_reconnect_email(body_email) != normalize_reconnect_email(login_email) {
        return false;
    }
    // Prefer AI-takeover / offline bodies; do not yank a still-connected human.
    if connected && !ai_controlled {
        return false;
    }
    true
}

/// Scan `(conn_id, deleted, connected, ai_controlled, email)` for the best reclaim target.
///
/// Prefers `ai_controlled` bodies, then any offline living match. Returns conn_id.
pub fn find_reconnect_body_conn_id<'a, I>(login_email: &str, bodies: I) -> Option<u64>
where
    I: IntoIterator<Item = (u64, bool, bool, bool, &'a str)>,
{
    let mut best_ai: Option<u64> = None;
    let mut best_offline: Option<u64> = None;
    for (conn_id, deleted, connected, ai_controlled, email) in bodies {
        if !body_eligible_for_reconnect(deleted, connected, ai_controlled, email, login_email) {
            continue;
        }
        if ai_controlled {
            best_ai = Some(conn_id);
        } else if best_offline.is_none() {
            best_offline = Some(conn_id);
        }
    }
    best_ai.or(best_offline)
}

/// Haxe rlogin position snap: birth-relative origin reset so client coords restart at body tile.
///
/// Returns new `(gx/birth_x, gy/birth_y, x, y)` after folding world position into client origin.
// Haxe: rloginHelper gx=0 gy=0; x=tx; y=ty; exactTx/Ty = tx/ty
pub fn reconnect_position_snap(world_x: i32, world_y: i32) -> (i32, i32, i32, i32) {
    // Keep absolute world tile; zero birth so client relative coords equal world.
    (0, 0, world_x, world_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_on_alive_disconnect() {
        let mut connected = true;
        let mut ai = false;
        assert_eq!(
            attach_ai_takeover(&mut connected, &mut ai, false),
            TakeoverAttachResult::Attached
        );
        assert!(!connected);
        assert!(ai);
    }

    #[test]
    fn skip_attach_when_deleted() {
        let mut connected = true;
        let mut ai = false;
        assert_eq!(
            attach_ai_takeover(&mut connected, &mut ai, true),
            TakeoverAttachResult::SkippedDeleted
        );
        assert!(!connected);
        assert!(!ai);
    }

    #[test]
    fn release_restores_human_client() {
        let mut connected = false;
        let mut ai = true;
        release_ai_takeover(&mut connected, &mut ai);
        assert!(connected);
        assert!(!ai);
        assert!(player_is_human(connected, ai, "alice@example.com"));
    }

    #[test]
    fn human_takeover_is_ai_while_offline() {
        assert!(player_is_ai(false, true, "alice@example.com"));
        assert!(!player_is_human(false, true, "alice@example.com"));
        // Permanent NPC email always AI even if "connected".
        assert!(player_is_ai(true, false, "npc-forager@local"));
        assert!(account_is_permanent_ai("selfplay@local"));
        assert!(!account_is_permanent_ai("alice@example.com"));
    }

    #[test]
    fn no_respawn_for_human_replacement_ai() {
        assert!(!should_respawn_ai_after_death(false));
        assert!(should_respawn_ai_after_death(true));
        let mut ai = true;
        clear_ai_on_death(&mut ai);
        assert!(!ai);
    }

    #[test]
    fn reconnect_prefers_ai_controlled_body() {
        let bodies = [
            (10u64, false, false, false, "a@x"),
            (11u64, false, false, true, "a@x"),
            (12u64, true, false, true, "a@x"), // dead
            (13u64, false, true, false, "a@x"), // still online human — skip
        ];
        assert_eq!(find_reconnect_body_conn_id("A@X", bodies), Some(11));
    }

    #[test]
    fn reconnect_offline_without_flag() {
        let bodies = [(5u64, false, false, false, "bob@y")];
        assert_eq!(find_reconnect_body_conn_id("bob@y", bodies), Some(5));
    }

    #[test]
    fn reconnect_rejects_wrong_email_and_online() {
        let bodies = [
            (1u64, false, false, true, "other@x"),
            (2u64, false, true, false, "me@x"),
        ];
        assert_eq!(find_reconnect_body_conn_id("me@x", bodies), None);
    }

    #[test]
    fn position_snap_zeros_birth() {
        assert_eq!(reconnect_position_snap(100, -40), (0, 0, 100, -40));
    }
}
