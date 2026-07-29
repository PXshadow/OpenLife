# Chunk: REPUTATION-HIT / hit_reputation

**Status:** DONE — 2026-07-26 (gap close: no double-count, category prestige+GM, calm restore)  
**Related:** **PRESTIGE-ALLY-COST** DONE 2026-07-28 — ally cost LiveSettings + exile-aware isAlly on gate/factor/GM  
**Mode:** implement  
**Haxe:** `openlife/server/GlobalPlayerInstance.hx` `kill` L4504–4561 after `DoDamage`; `TimeHelper` L404–405  
**Rust:** `ol-sim/src/reputation.rs` + `combat.rs` + `lib.rs` (`apply_connecting_hit_reputation`, `tick_combat_reputation_restore`)

## Haxe behavior

After a connecting hit (`doDamage` returns damage > 0):

1. `attackWasLegit = damage < 2 * target.lostCombatPrestige`
2. **Legit:** both `lostCombatPrestige -= damage/2` (recover)
3. **Illegal + target unarmed:**
   - attacker guilt `+= damage` (or `×0.5` if attacker prestigeClass > target)
   - optional category cost `ceil(damage * PrestigeCostPerDamage*)` for child / elder / ally / close relative / unarmed woman
   - Devil Mask 3213 multiplies category damage ×5; skips `addHealthAndPrestige` + GM
4. **Illegal + target armed:** no float change (duel)

Not the exile "legal" flag — that is separate for death reason / scoreboard prestige.

Calm tick (`angryTime >= 0 && lost > 0 && darkNosaj < 1`):  
`lostCombatPrestige -= (CombatReputationRestorePerYear * dt) / 60`.

## Rust

| Piece | Location |
|-------|----------|
| Pure | `attack_was_legit`, `compute_hit_reputation`, `HitReputationInput/Delta`, `PrestigeCostCategory`, `PrestigeCostFactors`, `combat_reputation_restore_delta`, `format_prestige_cost_global_message`, `ReputationBook::apply_hit_delta` |
| Constants | MinAgeToEat=3, elderly>50, PrestigeCost* defaults, DEVIL_MASK=3213, `COMBAT_REPUTATION_RESTORE_PER_YEAR=2` |
| Live | `apply_connecting_hit_reputation` on HIT Wound + Kill + SAY KILL; category score prestige + GM; **ally factor LiveSettings** |
| Ally (PRESTIGE-ALLY-COST) | exile-aware `is_ally` after mid-hit exile; peer-ally GM integration test |
| Kill path | `resolve_kill` **does not** touch `lost_combat_prestige` (score prestige only) — float only via apply_connecting |
| Mirror | `combat.stats.lost_combat_prestige` for AI scans |
| Tick | `tick_combat_reputation_restore` in `tick_vitals_with_metrics` |
| Build wire | `build_reputation_hit.rs` + `src/_apply_prestige_ally_cost.py` |
| Tests | `reputation::*`; `say_hit_wound_*` / `say_hit_legit_*`; `say_kill_illegal_no_double_lost_combat`; `tick_restore_combat_reputation_when_calm`; `resolve_kill_leaves_lost_combat_prestige_unchanged`; **`prestige_cost_category_ally_*` / `say_hit_peer_ally_*` / `is_ally_breaks_*`** |

## Residuals

- Full Haxe `addHealthAndPrestige` (`yum_multiplier` + family share) — score prestige proxy only
- `darkNosaj` gate on restore (Player field not ported; treated as 0)
- Lineage disk `reputation = lost * -1` on death (`LineageNode` has no reputation field / OLN1)
- Other PrestigeCost* (child/elder/relative/woman) + restore rate not LiveSettings (ally **is** LiveSettings)
- Haxe L4525 recent-exile-ally TODO (open both sides)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
python crates\ol-sim\src\_apply_prestige_ally_cost.py
cargo test -p ol-sim --lib -- reputation
cargo test -p ol-sim --lib -- say_hit_wound_applies_reputation
cargo test -p ol-sim --lib -- say_hit_legit_recovers
cargo test -p ol-sim --lib -- say_kill_illegal_no_double
cargo test -p ol-sim --lib -- tick_restore_combat_reputation
cargo test -p ol-sim --lib -- resolve_kill_leaves_lost
cargo test -p ol-sim --lib -- say_hit_peer_ally_prestige
cargo test -p ol-sim --lib -- is_ally_breaks_on_multi_hop
```
