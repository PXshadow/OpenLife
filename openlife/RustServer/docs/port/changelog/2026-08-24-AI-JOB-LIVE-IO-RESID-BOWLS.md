# AI-JOB-LIVE-IO-RESID / cleanUpBowls live

## Chunk
- **matrix_id:** `AI-JOB-LIVE-IO-RESID`
- **status:** **DONE core** — `cleanUpBowls` pure + baker `DeferCleanup` live; hungry-cost remains elsewhere
- **Haxe:** `AiBase.cleanUpBowls` ~4191–4219; `cleanUp` ~1052–1053
- **Rust:** `ol-ai-professions` `cleanup_profession` + `ol-sim` `cleanup_counts_from_scan`

## Implemented
1. `CleanupBowlSnap` + `clean_up_bowls_action(253|1176)`
2. Gooseberry remap 253→31; dry beans filler 1160; extra clay bowls pickup pods; empty single-use only
3. `clean_up_action` tail: bowls after clay merge (Haxe order)
4. Live: `cleanup_bowl_snap_from_scan` home r=30 + closest from player; `DeferCleanup` already calls `clean_up_action`

## Tests
- `clean_up_bowls_gooseberry_empties_two_single_use`
- `clean_up_bowls_dry_beans_fill_then_empty`
- `clean_up_action_bowls_after_clay_merge`
- `baker_defer_cleanup_expands_clean_up_bowls_gooseberry`

## Residual
- Hungry-cost pair / live AI notReachable map (**CRAFT-LIVE-IO** / **NPC-SCAN-FULL**)

## Verify
```powershell
cargo test -p ol-ai-professions --lib -- clean_up_bowls clean_up_action -- --test-threads=1
cargo test -p ol-sim --lib -- baker_defer_cleanup -- --test-threads=1
```
