# CALL_INDEX fragment: PRESTIGE-ALLY-COST / ally_prestige_cost

**Merged into** [CALL_INDEX.md](CALL_INDEX.md) under ally combat / combat reputation (2026-07-28).

## Rust: ally prestige cost (PRESTIGE-ALLY-COST)

| Symbol | File | Role |
|--------|------|------|
| `is_ally` | `ol-sim/src/relations.rs` | Haxe `isAlly` (exile-aware `get_top_leader`) |
| `is_leadership_ally` | same | follow-graph only (no exile); misc non-HIT |
| `PrestigeCostFactors` / `compute_hit_reputation_with_factors` | `ol-sim/src/reputation.rs` | live/test category multipliers |
| `PRESTIGE_COST_PER_DAMAGE_ALLY` / `PrestigeCostCategory::Ally` | same | default 1 + GM phrase `"ally"` |
| `format_prestige_cost_global_message` | same | `Lost N prestige for attacking ally Name!` |
| `GameplayKnobs.prestige_cost_per_damage_for_ally` | `settings_live.rs` | LiveSettings → sim |
| `apply_connecting_hit_reputation` | `lib.rs` | recompute `is_ally` + live ally factor + GM |
| HIT gate / strength / anger / SAY KILL | `lib.rs` | exile-aware `is_ally` |
| Tests | `is_ally_breaks_on_multi_hop_exile` / `prestige_cost_category_ally_and_gm_text` / `say_hit_peer_ally_prestige_cost_and_gm` / `say_hit_multi_hop_*` | pure + live |

Residual: L4525 recent-exile-ally TODO; other PrestigeCost* LiveSettings; full yum_multiplier health / darkNosaj.
