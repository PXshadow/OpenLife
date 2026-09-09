# 2026-09-09 SPAWN-QUEUE-POLICY

**status:** **DONE**

Haxe `Connection.loginHelper` L135-138 TODOs + live MaxPlayers / IP new-account cap.

- Full = living humans+AIs + incoming > `max_players` (`server.toml`)
- New AI logins rejected when full; human logins cull synthetic AIs (keep `npc_min` unless high priority)
- Priority = account score + short last life + time away; reconnect of a living body always allowed
- IP: `NewAccountsPerIpPerDay` 3 / `TotalNewAccountsPerDay` 20; login spam 8/60s (empty IP skips)

Tests: `spawn_queue::tests::*` / `spawn_queue_full_human_culls_ai` / `spawn_queue_full_human_without_ai_rejected` / `spawn_queue_ip_spam_rejected`

`cargo check -p ol-server --offline` ok.

Residual: NPC scheduler local `active` may try to relog culled AIs; last-life stamped on revive not every death path. Next **VANILLA-ID-MAP**.
