# 2026-09-07 YOU-ARE-PROF

**status:** **DONE**

Hearer `YOU ARE SMITH` (and aliases) assigns profession; not speaker DoNaming first-name. Live fan-out sets job runtimes (clears prior assigned).

- Follower/relative gate (`I AM NOT YOUR FOLLOWER!`)
- `YOU ARE ALICE` still swallows (not a profession)
- `SMITH!` still assigns; runtimes now sync too

Tests: `you_are_profession_assigns` / `fan_out_ai_say_you_are_profession`

`cargo check -p ol-server --offline` ok.

Residual: npc starving cands. Next **DO-WATERING-LOW**.
