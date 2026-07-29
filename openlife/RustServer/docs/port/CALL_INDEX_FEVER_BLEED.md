# Call index slice — COMBAT-FEVER-BLEED / fever_bleed

Appended from `haxe-port-chunk` (matrix_id COMBAT-FEVER-BLEED). Merged rows also in [CALL_INDEX.md](CALL_INDEX.md).

## Haxe

| Symbol | File | Role |
|--------|------|------|
| `DoDamage` doesRealDamage / moskitoDamageFactor | `server/GlobalPlayerInstance.hx` | parent≠2156; jungle love + yfCount + health(2,0.5) |
| `DoDamage` yellowfeverCount += 0.02 / fever infect | same | non-real hit resistance; `0.2*factor²` roll → fever NestedHelper |
| `hasYellowFever` / fever.id==2155 | same | heal gate + vitals gate |
| `updateFoodAndDoHealing` bleedingDamage | `server/TimeHelper.hx` | wound.objectData.damage × WoundDamageFactor → hits + 2×food |
| yellow fever food/heat | same | ExhaustionYellowFeverPerSec×2 food; heat += 0.02×isHeldFaktor |
| DoTimeOnPlayerObjects fever/hiddenWound | same | clear on TTC; re-equip empty hands; survive GM |

## Rust

| Symbol | File | Role |
|--------|------|------|
| `moskito_damage_factor` / `roll_yellow_fever_infect` / `plan_mosquito_fever_infect` | `ol-sim/src/weapon_wound.rs` | pure infect plan |
| `does_real_damage` / `fever_time_to_change` | same | non-real gate + TTC scale |
| `wound_object_id_for_bleed` / `object_damage_bleed_rate` | same | held vs hiddenWound bleed source |
| `yellow_fever_food_drain` / `yellow_fever_heat_delta` | `ol-sim/src/food_store_max.rs` | fever vitals pure |
| `is_yellow_fever` / `clear_if_timer_elapsed` | `ol-sim/src/nested_body.rs` | fever id 2155 + timer clear |
| `apply_mosquito_fever_candidate` | `ol-sim/src/lib.rs` | live infect + GM/say/sad |
| `tick_body_fever_and_hidden_wound` | same | body TTC clear + re-equip + survive GM |
| tick ObjectDef.damage bleed | same | tick_vitals preferred over stack×0.05 |
| Tests | `weapon_wound::*` fever + `food_store_max` yf/bleed | pure infect + rates 0.05/0.06/0.1 |

## FEVER-EMOTE append

| Symbol | File | Role |
|--------|------|------|
| `resolve_fever_pe_emote` / `resolve_update_emotes` | `ol-sim/src/fever_pe.rs` | pure UpdateEmotes ladder |
| `tick_update_emotes` / `should_update_emotes_this_tick` | `ol-sim/src/lib.rs` + fever_pe | tick%30 PE fan-out |
| `emit_feed_too_ill_feedback` | `ol-sim/src/lib.rs` | doEating ill say + PE 7 |
| Tests | `fever_pe::*` + `tick_vitals_emits_pe_yellow_fever_*` | pure + live PE 7/21 |
