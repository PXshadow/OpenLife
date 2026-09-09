# 2026-09-09 STARTING-GX-DEATH (+ NESTED-PERSIST-SER skip)

**status:** **DONE**

## NESTED-PERSIST-SER (skipped as already live)

FILE_MATRIX S-SER / Haxe `SerializeHelper.hx` is GPI RTTI codegen. Runtime nested persist is `ObjectHelper.WriteToFile` / OLW3 `NestedHelper`.

Already roundtrips recursive contained + uses + owners + times + custom vars + map `ground_id`. Tests: `olw3_slot_meta_and_owners_roundtrip` / `nested_helper_write_read_pure`.

Residual: NestedHelper has no `ground_id` (Haxe WriteToFile also omits `groundObject`; map cells persist it). Dummy ids stay parent+uses in helpers; `transform_to_dummy` is load-time with content.

## STARTING-GX-DEATH (this fire)

Haxe `GlobalPlayerInstance.doDeathHelper` L3995-3996: `ServerSettings.startingGx/Gy = this.tx/ty` after ChooseNewLeader (next Eve origin).

- `stamp_starting_gx_from_death` writes `SimState.spawn_x/y` from the death world tile (captured before held detach)
- Wired from `apply_death_polish` (ol-sim sole writer)

Tests: `death_stamps_starting_gx_gy_for_next_eve`

`cargo check -p ol-server` Finished.

Residual: session-only like Haxe statics (not written to server.toml). Next **SETTINGS-KNOB-TAIL**.
