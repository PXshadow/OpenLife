# 2026-09-09 BABY-BONES-ARMS

**status:** **DONE**

Haxe `GlobalPlayerInstance.doDeathHelper` L4014 TODO: place baby bones in arms.

- When a **held** player dies, detach from the carrier and `set_held(1920)` (Baby Bones) on the carrier
- Dying carrier still drops the held player onto the death tile (Haxe `dropPlayer`)
- Grave placement unchanged (baby pile 3053 when content has it)
- Unheld death does not give bones to a bystander

Tests: `held_baby_hunger_death_puts_bones_in_carrier_arms` / `unheld_hunger_death_does_not_give_bystander_bones` / `held_baby_death_places_bones_in_carrier_arms` / `unheld_death_does_not_give_bystander_bones`

`cargo check -p ol-server --offline` Finished.

Residual: Basket of Baby Bones 3052 unused; superMeh eat-death may skip `apply_death_polish`. Next **RECENT-EXILE-ALLY**.
