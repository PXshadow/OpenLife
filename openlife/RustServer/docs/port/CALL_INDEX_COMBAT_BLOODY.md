# Call index slice — COMBAT-BLOODY / bloody_weapon

Appended from `haxe-port-chunk` (matrix_id COMBAT-BLOODY). Merge into [CALL_INDEX.md](CALL_INDEX.md) when convenient.

## Haxe

| Symbol | File | Role |
|--------|------|------|
| `GlobalPlayerInstance.makeWeaponBloodyIfNeeded` | `server/GlobalPlayerInstance.hx` | Knife 560/750→750, War Sword 3047/3048→3048 on deadly animal; ttc=3 |
| `GlobalPlayerInstance.DoDamage` bloody held | same | `trans.newActorID` + `WeaponCoolDownFactor` / `IfWounding` on attacker held |
| `ObjectData.isBloody` / `neverDrop` / `speedMult` | `ServerSettings.PatchObjectData` | 750/3048/749 patches |
| `PatchTransitions` autoDecaySeconds | `ServerSettings.hx` | `-1+750`→3, `-1+3048`→2, `-1+749`→6 |
| `TimeHelper.DoTimeOnPlayerObjects` held | `server/TimeHelper.hx` | bloody auto-clean via `-1` transition |
| `TimeHelper.TryAnimaEscape` | same | Bow → Bloody Yew Bow 749, `timeToChange=2` |
| `TransitionHelper` isNeverDrop | `server/TransitionHelper.hx` | refuse DROP; isBloody re-arm ttc=3; countdown say |

## Rust

| Symbol | File | Role |
|--------|------|------|
| `make_weapon_bloody_if_needed` | `ol-sim/src/weapons.rs` | Haxe makeWeaponBloodyIfNeeded pure |
| `bloody_weapon_after_strike` / `weapon_bloody_time_to_change` | same | DoDamage cool-down × patched autoDecay base 3/2/6 |
| `bloody_weapon_auto_decay_base_ttc` / `bloody_weapon_clean_id_for` | same | 750→3/560, 3048→2/3047, 749→6/151 |
| `try_bloody_weapon_auto_clean` | same | pure held timer → clean id |
| `never_drop_remaining_secs` / `never_drop_should_rearm` / `never_drop_countdown_say` | same | DROP unstick helpers |
| `bloody_weapon_id_for` / `is_bloody_weapon` / `is_never_drop_weapon` / `bloody_weapon_speed_mult` | same | id map + flags + 0.75/0.85/0.6 |
| `weapon_damage` bow patches | same | 152→9, 1624→12 |
| `held_object_speed_mult` bloody override | `ol-sim/src/move_speed.rs` | PatchObjectData over content 0.25 |
| `BowEscapeEffects.time_to_change` | `ol-sim/src/animal_damage.rs` | TryAnimaEscape ttc=2 |
| `apply_bloody_weapon_transform` / `BloodyApplyMode` | `ol-sim/src/lib.rs` | HIT/HUNT held transform + held_helper ttc + PU |
| `tick_held_bloody_auto_clean` | same | vitals tick auto-clean |
| DROP `is_never_drop_weapon` + re-arm | `lib.rs` `apply_drop` | refuse + re-arm + countdown say |
| HIT Wound/Kill | `lib.rs` SAY HIT | `BloodyApplyMode::Strike` + PU |
| HUNT Hit/Kill | `lib.rs` SAY HUNT | `BloodyApplyMode::Animal` deadly gate + PU |
| Tests | `weapons::*`, `animal_damage::bow_escape_*` | make bloody, cool-down, auto-clean, neverDrop, bow escape ttc |
