# 2026-09-04 CLOTHING-HAS-TAILOR

**status:** **DONE**

Haxe `hasOrBecomeProfession('TAILOR', maxProf)` gates medium/low clothing. Live paths hardcoded `has_tailor: true`.

- `has_or_become_tailor` — sticky last, `max<0` high-prio, else `count >= max + wasIdle`
- `clothing_has_tailor_for_player` — live roster `countProfession` + assigned max=100
- `apply_clothing_craft_tick` / sensor extras / npc 2a3 use the gate
- Assign `lastProfession=TAILOR` only after high band / `fillUpQuiver` skip

Tests: `has_or_become_tailor_max_and_sticky` / `clothing_has_tailor_for_player_respects_peer_cap` / `apply_clothing_craft_tick_assigns_last_tailor_when_gate_opens`

`cargo check -p ol-server --offline` ok.
