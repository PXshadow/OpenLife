//! Haxe `DoDebugCommands` — `!S` secret + gated admin SAY.
//!
//! Secret is live `ServerSettings.Secret` (`server.toml` / `GameplayKnobs.secret`).
//! `AllowDebugCommmands` gates the whole table. Do not log the secret value.

use crate::admin_env::{parse_season, set_season};
use crate::send_ps_reply;
use crate::SimState;
use ol_net::OutboundHub;

/// Haxe `BiomeTag.SNOW` (ol_world biome id 4).
const BIOME_SNOW: u8 = 4;

/// True when SAY is consumed as an admin/secret command.
// Haxe: GlobalPlayerInstance.DoDebugCommands ~5155
pub fn try_admin_say(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    text: &str,
) -> bool {
    if !state.gameplay.allow_debug_commands {
        return false;
    }
    let trimmed = text.trim();
    let upper = trimmed.to_uppercase();
    if upper.starts_with("!S ") {
        // Haxe: strings = text.split(' '); secret = strings[1]
        let secret = trimmed.split_whitespace().nth(1).unwrap_or("");
        let expected = state.gameplay.secret.as_str();
        let ok = !secret.is_empty() && secret == expected;
        if let Some(p) = state.players.get(&conn_id) {
            let acc = state.accounts.ensure(&p.email);
            acc.can_use_server_commands = ok;
            acc.role = if ok { 10 } else { 0 };
        }
        send_ps_reply(outbound, conn_id, &format!("Secret: {ok}"));
        return true;
    }
    if upper.contains("!SEASON") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        let season_tok = if let Some(rest) = upper.strip_prefix("!SEASON") {
            rest.trim()
        } else if let Some(idx) = upper.find("!SEASON") {
            upper[idx + "!SEASON".len()..].trim()
        } else {
            ""
        };
        if let Some(s) = parse_season(season_tok) {
            set_season(&mut state.environment, s);
            send_ps_reply(outbound, conn_id, &format!("Season set to {season_tok}"));
        }
        return true;
    }
    if upper.contains("!COIN") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if let Some(p) = state.players.get(&conn_id) {
            let pid = p.p_id;
            state.economy.add_coins(pid, 20);
        }
        send_ps_reply(outbound, conn_id, "Got More coins");
        return true;
    }
    if upper.contains("!BIOME") && !upper.contains("!OBIOME") {
        let biome = state.players.get(&conn_id).map(|p| {
            let w = state.world.read().unwrap();
            w.get_biome(p.x, p.y)
        });
        if let Some(b) = biome {
            send_ps_reply(outbound, conn_id, &format!("BIOME {b}"));
        }
        return true;
    }
    if upper.contains("!SNOW") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if let Some(p) = state.players.get(&conn_id) {
            let (x, y) = (p.x, p.y);
            state.world.write().unwrap().set_biome(x, y, BIOME_SNOW);
        }
        send_ps_reply(outbound, conn_id, "SNOW");
        return true;
    }
    if upper.contains("!COLD") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.heat = 0.0;
        }
        return true;
    }
    if upper.contains("!HOT") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.heat = 1.0;
        }
        return true;
    }
    if upper.contains("!UAGE") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.age -= 5.0;
            p.true_age -= 5.0;
        }
        return true;
    }
    if upper.contains("!AGE") || upper == "!" {
        // Haxe: !AGE / `!` — checkIfNotAllowed(player, text != '!')
        if admin_not_allowed_ex(state, outbound, conn_id, upper != "!") {
            return true;
        }
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.age += 5.0;
            p.true_age += 5.0;
        }
        return true;
    }
    if upper.contains("!MEH") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.food -= 5.0;
        }
        return true;
    }
    // Haxe `!F` food +10 — keep after `!F` prefixes that are more specific.
    if upper == "!F" || upper.starts_with("!F ") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.food += 10.0;
        }
        return true;
    }
    if upper.contains("!THOME") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.x = p.home_x;
            p.y = p.home_y;
        }
        crate::force_send_map_chunk(state, outbound, conn_id);
        return true;
    }
    if upper.contains("!SPEED") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if state.gameplay.speed_factor < 2.0 {
            state.gameplay.speed_factor = 10.0;
        } else {
            state.gameplay.speed_factor = 1.0;
        }
        send_ps_reply(outbound, conn_id, "Changed Speed!");
        return true;
    }
    if upper.contains("!CREATE") || upper.starts_with("!C ") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        let id = parse_create_id(trimmed);
        if id < 0 {
            return true;
        }
        if state.content.get(id).is_none() && id != 0 {
            send_ps_reply(outbound, conn_id, &format!("{id} is not an object"));
            return true;
        }
        if let Some(p) = state.players.get(&conn_id) {
            let (x, y) = (p.x, p.y);
            state.world.write().unwrap().set_object(x, y, id);
        }
        return true;
    }
    if upper.contains("!HEAL") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if let Some(p) = state.players.get(&conn_id) {
            let pid = p.p_id;
            let s = state.combat.stats_mut(pid);
            s.hits = (s.hits - 10.0).max(0.0);
        }
        if let Some(p) = state.players.get_mut(&conn_id) {
            if p.exhaustion > 0.0 {
                p.exhaustion = 0.0;
            }
        }
        return true;
    }
    if upper.contains("!HIT") {
        if admin_not_allowed(state, outbound, conn_id) {
            return true;
        }
        if let Some(p) = state.players.get(&conn_id) {
            let pid = p.p_id;
            state.combat.stats_mut(pid).hits += 10.0;
        }
        return true;
    }
    false
}

fn parse_create_id(text: &str) -> i32 {
    text.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(-1)
}

fn admin_not_allowed(state: &mut SimState, outbound: &OutboundHub, conn_id: u64) -> bool {
    admin_not_allowed_ex(state, outbound, conn_id, true)
}

fn admin_not_allowed_ex(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    say_error: bool,
) -> bool {
    let allowed = state
        .players
        .get(&conn_id)
        .map(|p| {
            state
                .accounts
                .get(&p.email)
                .map(|a| a.can_use_server_commands)
                .unwrap_or(false)
        })
        .unwrap_or(false);
    if !allowed {
        if say_error {
            send_ps_reply(outbound, conn_id, "not allowed!");
        }
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Season;
    use crate::spawn_player;
    use ol_content::ContentDb;
    use ol_net::OutboundHub;
    use std::sync::Arc;

    #[test]
    fn secret_grants_admin_then_coin() {
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = crate::SimState::with_default_empty(Arc::new(ContentDb::default()));
        state.gameplay.secret = "JASON".into();
        state.gameplay.allow_debug_commands = true;
        spawn_player(&mut state, 1, "admin@test");
        while rx.try_recv().is_ok() {}
        assert!(try_admin_say(&mut state, &hub, 1, "!S JASON"));
        assert!(state
            .accounts
            .get("admin@test")
            .unwrap()
            .can_use_server_commands);
        assert_eq!(state.accounts.get("admin@test").unwrap().role, 10);
        while rx.try_recv().is_ok() {}
        let pid = state.players.get(&1).unwrap().p_id;
        let coins0 = state.economy.wallet_mut(pid).coins;
        assert!(try_admin_say(&mut state, &hub, 1, "!COIN"));
        assert_eq!(state.economy.wallet_mut(pid).coins, coins0 + 20);
    }

    #[test]
    fn wrong_secret_and_not_allowed() {
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = crate::SimState::with_default_empty(Arc::new(ContentDb::default()));
        state.gameplay.secret = "JASON".into();
        state.gameplay.allow_debug_commands = true;
        spawn_player(&mut state, 1, "nope@test");
        while rx.try_recv().is_ok() {}
        assert!(try_admin_say(&mut state, &hub, 1, "!S WRONG"));
        assert!(!state
            .accounts
            .get("nope@test")
            .unwrap()
            .can_use_server_commands);
        while rx.try_recv().is_ok() {}
        assert!(try_admin_say(&mut state, &hub, 1, "!SEASON WINTER"));
        let pkt = rx.try_recv().expect("not allowed");
        assert!(String::from_utf8_lossy(&pkt).contains("not allowed"));
        assert_ne!(state.environment.season, Season::Winter);
    }

    #[test]
    fn allow_debug_commands_false_skips_secret() {
        let hub = OutboundHub::new();
        let mut state = crate::SimState::with_default_empty(Arc::new(ContentDb::default()));
        state.gameplay.secret = "JASON".into();
        state.gameplay.allow_debug_commands = false;
        spawn_player(&mut state, 1, "admin@test");
        assert!(!try_admin_say(&mut state, &hub, 1, "!S JASON"));
        assert!(!state
            .accounts
            .get("admin@test")
            .map(|a| a.can_use_server_commands)
            .unwrap_or(true));
    }

    #[test]
    fn season_and_create_after_secret() {
        let hub = OutboundHub::new();
        let mut state = crate::SimState::with_default_empty(Arc::new(ContentDb::default()));
        state.gameplay.secret = "JASON".into();
        state.gameplay.allow_debug_commands = true;
        spawn_player(&mut state, 1, "admin@test");
        assert!(try_admin_say(&mut state, &hub, 1, "!S JASON"));
        assert!(try_admin_say(&mut state, &hub, 1, "!SEASON WINTER"));
        assert_eq!(state.environment.season, Season::Winter);
        assert!(try_admin_say(&mut state, &hub, 1, "!CREATE 0"));
        let (x, y) = {
            let p = state.players.get(&1).unwrap();
            (p.x, p.y)
        };
        assert_eq!(state.world.read().unwrap().get_object(x, y), 0);
    }
}
