# Chunk: PRESTIGE-ALLY-COST / ally_prestige_cost

**Status:** DONE — 2026-07-28 (gap-close: HIT gate/strength/anger exile-aware)  
**Mode:** implement  
**Haxe:** `openlife/server/GlobalPlayerInstance.hx` `kill` L4454 gate + L4540–4545 (`PrestigeCostPerDamageForAlly` + GM); `isAlly` L6123–6126; mid-hit `exile` L4481 before cost  
**Settings:** `ServerSettings.PrestigeCostPerDamageForAlly` default **1**  
**Rust:** `ol-sim` `relations::is_ally` + `reputation::PrestigeCostFactors` / `compute_hit_reputation_with_factors` + live HIT via `apply_connecting_hit_reputation`

## Haxe behavior

After a connecting illegal unarmed hit (not legit, target unarmed):

1. Category chain: child → elder → **ally** → close relative → unarmed woman  
2. Ally branch: `targetPlayer.isAlly(this) && !isCursed`  
3. `prestigeCost = ceil(damage * PrestigeCostPerDamageForAlly)`  
4. `lostCombatPrestige += prestigeCost`  
5. Without Devil Mask 3213: `addHealthAndPrestige(-prestigeCost)` +  
   `sendGlobalMessage('Lost $prestigeCost prestige for attacking ally ${name}!')`  
6. **isAlly** uses full `getTopLeader` (exile edges). Mid-hit `exile(target)` can drop multi-hop followers out of ally status before the cost runs; **peer** allies under the same top leader remain allies after peer exile.
7. Unarmed ally **gate** also uses `isAlly` — already-exiled multi-hop targets do not re-warn / re-exile.

## Rust

| Piece | Location |
|-------|----------|
| Exile-aware ally | `relations::is_ally` (Haxe `isAlly` / `get_top_leader`) |
| Follow-only helper | `is_leadership_ally` (no exile; misc non-HIT) |
| Pure cost | `compute_hit_reputation` / `*_with_factors` + `PrestigeCostFactors.ally` |
| GM text | `PrestigeCostCategory::Ally` → `"ally"`; `format_prestige_cost_global_message` |
| Live factor | `GameplayKnobs.prestige_cost_per_damage_for_ally` ← LiveSettings / `server.toml` |
| HIT gate + strength + anger | exile-aware `is_ally` (not `is_leadership_ally`) |
| HIT recompute | post-exile `is_ally` for allyFactor + reputation |
| Apply path | `apply_connecting_hit_reputation` recomputes `is_ally` + live factors |
| SAY KILL pre-flag | exile-aware `is_ally` (recomputed in apply_connecting) |
| Tests | `is_ally_breaks_on_multi_hop_exile`; `prestige_cost_category_ally_and_gm_text`; `say_hit_peer_ally_prestige_cost_and_gm`; `say_hit_multi_hop_already_exiled_skips_ally_gate`; `say_hit_multi_hop_second_hit_no_ally_prestige_category` |

## Residuals

- Haxe L4525 “count as ally if exile happened not long ago” (open both sides)  
- Other PrestigeCost* categories (child/elder/relative/woman) still module-const factors (ally is LiveSettings)  
- `CombatReputationRestorePerYear` still module-const  
- Full `addHealthAndPrestige` yum_multiplier / family share / darkNosaj early skip (score prestige proxy only)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- is_ally_breaks_on_multi_hop
cargo test -p ol-sim --lib -- prestige_cost_category_ally
cargo test -p ol-sim --lib -- say_hit_peer_ally_prestige
cargo test -p ol-sim --lib -- say_hit_multi_hop
cargo test -p ol-config --lib -- gameplay_defaults_match_haxe
```
