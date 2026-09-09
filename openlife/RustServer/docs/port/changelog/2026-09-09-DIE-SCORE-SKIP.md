# 2026-09-09 DIE-SCORE-SKIP

**status:** **DONE**

Haxe `Connection.die` L840 TODO: do **not** lower score if `/DIE` is used.

- Prestige gate kept (`score < PrestigeCostForDie` still refuses)
- SAY DIE and client DIE tag no longer `score -= PrestigeCostForDie`
- Tests: `say_die_skips_prestige_cost_debit` / `client_die_skips_prestige_cost_debit` / `say_die_refuses_when_too_little_prestige`

`cargo check -p ol-server --offline` ok.

Residual: Haxe then `food_store -= 100` (starve death); Rust still instant `reason_suicide`. Next **SPAWN-QUEUE-POLICY**.
