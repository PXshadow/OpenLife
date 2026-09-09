# 2026-09-09 LOCATION-SAYS-MARKERS

**status:** **DONE**

SAY MARK stores `MarkerState` then fans LOCATION_SAYS (`LS`) so clients see the pin.

- After a labeled MARK, `format_location_says` + FRAME to the speaker
- Close humans (Haxe `SendLocationToAllClose` r=20) get the same text at their birth-relative coords
- Wire text: `x y ! label` (MarkerState `wire_lines_for` / `custom_mark_ls_text`)
- Empty MARK stays PS FAIL and does not emit LS
- Social MOTHER/LEADER pins stay `map_location_pins` PS

Tests: `say_mark_adds_custom_marker_for_self` / `say_mark_fans_ls_to_close_not_far` / `say_without_mark_does_not_emit_ls`

`cargo check -p ol-server --offline` Finished.

Residual: 1s debug LS is still coords not markers. Next **AGE-10-FATHER-LIVE**.
