# 2026-09-09 WELLS-OIL-DECAY

**status:** **DONE**

Product TODO: wells / oil never decay. Haxe `PatchObjectData` L675-684 sets `decayFactor = -1` when description contains Well / Pump (not Pumpkin) / Vein / Mine / Iron Pit / Drilling / Rig / Cave / Ancient. Oil pumpjacks match Pump; oil rigs match Drilling/Rig.

- `apply_haxe_object_description_loops` already set `-1` (helper `haxe_never_decay_description`)
- `object_decay_chance_ex2` already skips `decay_factor <= 0` (Haxe `DecayObject`)

Tests: `wells_oil_description_never_decay` / `shallow_well_662_id_table_overrides_never_decay` / `object_decay_blocked_when_decay_factor_nonpositive`

`cargo check -p ol-server --offline` Finished.

Residual: Shallow Well 662 still decays to Natural Spring 3030 (Haxe id-table override 0.1). Oil Palm is not a never-decay keyword. Next **EVE-DEADLY-ANIMALS**.
