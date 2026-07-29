## Rust: ally combat strength (ALLY-STRENGTH / ally_combat)

| Symbol | File | Role |
|--------|------|------|
| `GlobalPlayerInstance.calculateEnemyVsAllyStrengthFactor` | `server/GlobalPlayerInstance.hx` | close friendly vs enemy strength ratio |
| `GlobalPlayerInstance.makeAllCloseAllyAngryAt` | same | set lastPlayerAttackedMe on close allies |
| `GlobalPlayerInstance.DoDamage` allyFactor | same | 0.5 if ally else strength factor cap 1.2 |
| `GlobalPlayerInstance.kill` unarmed ally gate | same | first-hit warn; second-hit exile then damage |
| `ServerSettings.AllyConsideredClose` / `AllyStrenghTooLowForPickup` | `settings/ServerSettings.hx` | radius 5; pickup gate default 0 |
| `TransitionHelper` AllyStrenghTooLowForPickup | `server/TransitionHelper.hx` | refuse non-empty target if factor too low |
| `calculate_enemy_vs_ally_strength_factor` | `ol-sim/src/combat.rs` | pure Haxe factor (base 10, weapon×2 food_max) |
| `resolve_ally_damage_factor` | same | ally → 0.5; else min(factor, 1.2) |
| `close_ally_ids_for_anger` / `combat_strength` / `is_close_for_ally_strength` | same | makeAllCloseAllyAngryAt ids + strength helper |
| `ally_strength_blocks_pickup` | same | pure TransitionHelper pickup gate (default off) |
| `resolve_unarmed_ally_hit_gate` / `unarmed_ally_first_hit_messages` | same | kill first-hit warn / second exile pure |
| `AllyStrengthPlayer` / `UnarmedAllyHitGate` | same | scan snapshot + gate enum |
| HIT wire allyFactor + anger + unarmed gate | `ol-sim/src/lib.rs` SAY HIT | source-wired (not build-only) |
| USE pickup gate | `ol-sim/src/use_transition.rs` | threshold default 0 = off; say "Too many hostile people..." |
| Tests | `combat::ally_*` / `unarmed_ally_*` / `say_hit_ally_*` | pure + live HIT |

Residual: PrestigeCostPerDamageForAlly (illegal ally prestige speech after damage); `AllyStrenghTooLowForPickup` not yet LiveSettings (const 0; USE path ready when >0).
