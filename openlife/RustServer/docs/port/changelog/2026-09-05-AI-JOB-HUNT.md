# 2026-09-05 AI-JOB-HUNT

**status:** **DONE**

Haxe `AiBase.isHunting` after handleTemperature: `hasOrBecomeProfession('HUNTER', max)` then home quad < 400 shortCraft Knife 560 + Rattle Snake 764, Firebrand 248 + Mosquito 2157/2156. Assigned/last `isHunting(100)`; mid `age > 14 && isHunting()`.

- `hunting.rs` pure SM + `hunting_live.inc.rs` scan
- Mid ladder Fire → Hunting → Graves; assigned HUNTER sticky
- Player `hunter_profession` / npc `hunter_rt`; last_profession writeback on newly last hunter
- HandleDeath `from_runtimes_ex` + `ladder_profession_scan_tick` pass hunter

Tests: `hunting::*` / `apply_profession_scan_tick_hunts_snake_near_home`

`cargo check -p ol-server --offline` ok.

Residual: LUMBERJACK `isCuttingWood`; COLLECTOR `isCollecting`; own-grave `getOwnerAccount`.
