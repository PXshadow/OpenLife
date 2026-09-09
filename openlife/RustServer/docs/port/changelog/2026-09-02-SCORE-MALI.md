# 2026-09-02 SCORE-MALI

**status:** **DONE**

Haxe `ScoreEntry.CreateScoreEntryForCursedGrave` subtracts `CursedGraveMali` (2) when a sharp stone pops from a decaying grave. Rust already had live `OldGraveDecayMali` / `AncestorPrestigeFactor`; `CursedGraveMali` was Deferred and the overflow path did not queue a score entry.

Live: `GameplayKnobs.cursed_grave_mali` + `create_score_entry_for_cursed_grave_ex`; `tick_auto_decays` overflow of id 34 queues mali.

Tests: `auto_decay_overflow_sharp_stone_queues_live_cursed_grave_mali` / `cursed_grave_stacks_or_creates` / `apply_live_settings_gameplay_knobs`.

`cargo check -p ol-server --offline` ok.
