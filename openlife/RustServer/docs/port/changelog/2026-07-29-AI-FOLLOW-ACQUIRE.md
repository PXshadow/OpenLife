# AI-FOLLOW-ACQUIRE / auto_follow

**Date:** 2026-07-29  
**Mode:** implement  
**Status:** DONE (core empty-sticky acquire)

## Scope

Close residual from **AI-FOLLOW-WALK**: when `playerToFollow` / `ai_follow_p_id` is empty,
Haxe `isMovingToPlayer` acquires a target before walking.

## Haxe

- `AiBase.isMovingToPlayer` empty branch (~8287–8296)
- `isChildAndHasMother` → `getFollowPlayer()` (leadership mother)
- else `ServerSettings.AutoFollowPlayer` → `getClosestPlayer(20, followHuman)`
- `GlobalPlayerInstance.getClosestPlayer` (humans first, AIs second)

## Rust

- Pure: `plan_auto_follow_acquire`, `resolve_auto_follow_acquire`,
  `get_closest_player_for_auto_follow`, `AutoFollowCandidate`
  (`crates/ol-sim/src/ai_follow_walk.rs`)
- Live: `tick_ai_follow_acquire` before continuous walk in `tick_ai_follow_walk`
  (`ai_follow_walk_live.inc.rs`)
- Leadership mother via `direct_follow_leader` + `social.following`
- `AUTO_FOLLOW_PLAYER_DEFAULT = false` (matches Haxe `ServerSettings.AutoFollowPlayer`)

## Residuals

- Debug say target name while walking
- Specialized baby/child/wounded distance bands
- `server.toml` / LiveSettings `AutoFollowPlayer` knob (const default today)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- ai_follow -- --test-threads=1
cargo test -p ol-sim --lib -- tick_ai_follow_acquire -- --test-threads=1
```
