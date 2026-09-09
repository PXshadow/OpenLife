# SPAWN-AS-CHILD pairing + PLANT-OFFSPRING + leftovers

## SPAWN-AS-CHILD
- **status:** **DONE** pairing wire (Haxe `lastAiEveOrAdam` / `lastHumanEveOrAdam`)
- `SimState.last_ai_eve` / `last_human_eve`
- Eve births (no mother) resolve partner; second AI Eve co-spawns on first tile + follow
- Cross-pool still gated by `MaxPlayersBeforeStartingAsChild` compiled **0** (`allow_human_ai_eve_cross`)
- Residual: LiveSettings promotion of the knob; human LOGIN still adult Eve (not spawnAsChild on busy servers)

## PLANT-OFFSPRING
- **status:** **DONE** path (Haxe `GrowNewPlantsFromExistingFactor` 0.05)
- Spring long-term band: living non-multi-use plant with `spring_regrow_factor` may `try_spawn_object_near`
- Skip when `current >= original`; low-pop × `GrowBackPlantsIncreaseIfLowPopulation`
- Residual: LiveSettings field (compiled 0.05 via `DecayChanceKnobs`)

## CRAFT-SPECIALS
- **status:** **DONE** (already live from AI-CRAFT-MULTI-RESID + countDone after USE)
- Residual: DeferPottery helper return; fill-bucket tank polish

## WEB-LINEAGE
- **status:** **DONE enough** vs Open Life Haxe (`/stats/lineage`, `/character`, accounts)
- Residual: lastSaid/coins/Eve id on HTML; placeholder faces

## Needle/thread
- **status:** document-only — do not "fix" Haxe bugs in Rust
