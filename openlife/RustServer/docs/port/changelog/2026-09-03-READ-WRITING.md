# 2026-09-03 READ-WRITING

**status:** **DONE**

Haxe `doCommandHelper` after USE/DROP/SWAP/REMV: if `heldObject.text` is non-empty, send PLAYER_SAYS `p_id/isCursed text}` + FRAME (extra `}` port-as-is). Actor connection only. Early refuses (neverDrop/wound/killMode/ally/grave/holding-player) skip. SWAP copies tile helper text into held.

Tests: `held_writing_ps_line_ports_extra_brace` / `use_far_sends_held_writing_ps` / `use_far_held_writing_ps_cursed` / `use_never_drop_skips_held_writing_ps` / `swap_sends_held_writing_ps`.

`cargo check -p ol-server --offline` ok.
