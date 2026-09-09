# 2026-09-03 CLEAR-WRITING

**status:** **DONE**

Haxe `TransitionHelper.doCommandHelper` Rubber Ball 2170 + Paper with Charcoal Writing 1615:

- Clears tile helper `text` and `hits` (dummy parent of 2170 counts).
- USE (before helper snapshot / trans) + DROP/SWAP.

Tests: `clear_writing_pair_matches_haxe_ids` / `use_rubber_ball_clears_paper_text_and_hits` / `use_rubber_ball_dummy_clears_paper` / `use_rubber_ball_writing_trans_stays_blank` / `drop_rubber_ball_clears_paper_text`.

`cargo check -p ol-server --offline` ok.
