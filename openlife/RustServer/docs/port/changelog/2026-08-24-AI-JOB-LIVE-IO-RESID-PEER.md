# AI-JOB-LIVE-IO-RESID / farm hasOrBecomeProfession live peer-cap

## Chunk
- **matrix_id:** `AI-JOB-LIVE-IO-RESID`
- **status:** **PARTIAL → farm peer-cap DONE**; remaining cleanUpBowls + hungry-cost pair
- **Haxe:** `AiBase.hasOrBecomeProfession` ~4466; `countProfession` ~1284; `doBasicFarming` ~2351; `doWatering` ~3549
- **Rust:** `ol-sim` `profession_scan` + `ol-ai-professions` `has_or_become_profession`

## Implemented
1. `ProfessionScanInput.farm_peer_lasts` — eligible same-home last farm jobs (`None` → aggregate `peer_count`)
2. `farm_scan_peer_count(inp, job)` — Haxe `countProfession(jobKey)`
3. `farm_profession_scan_tick` calls `has_or_become_profession` instead of the ladder `farm_has_profession` bool
4. Mid `doWatering(3)` uses WaterBringer-filtered count (not any-farm aggregate)
5. Live fill: `farm_peer_lasts_from_state` on ladder + apply scan; NPC `farm_peer_lasts_from_npc_rows` + `NpcProfessionPeerRow.last_farm`
6. Baker `DeferFarm` / makeStuff basic farm same gate

## Tests
- `farm_scan_peer_count_job_specific_vs_aggregate`
- `farm_profession_scan_tick_peer_cap_skips_when_not_sticky`
- `farm_profession_scan_tick_room_becomes_and_assigns_last`
- `npc_peer_count_for_kind_multi_prof_and_wounded` (lasts assert)

## Residual
- Full Haxe `cleanUpBowls` live (gooseberry / dry-bean bowls after pileUp)
- Hungry-cost pair / live AI notReachable map
- ~~NPC views without `profession_state` still lack job-specific last~~ **NPC-IGNORED-FLOOR** `PlayerSnapshot.last_farm` DONE

## Verify
```powershell
cargo test -p ol-sim --lib -- farm_profession_scan_tick_peer farm_scan_peer_count npc_peer_count_for_kind -- --test-threads=1
```
