# 2026-09-04 SHORTCRAFT-MAX-NEW-ACTOR

**status:** **DONE**

Haxe `shortCraftOnTarget` maxNewActor: `GetTransition(actor, target).newActorID`, `CountCloseObjects(..., 30)` + held, refuse when `count >= maxNewActor`. No trans → count 0 (never refuse).

- dropHeld PreferShortCraft: skip when at cap (`drop_held_object_ex` + ContentDb)
- profession farm/smith/baker/shepherd/fire-food: count `trans.newActorID` not actor parent
- npc/selfplay pass content into `smart_drop_held_from_sensors_ex`

Tests: `mutton_oven_max_new_actor_counts_transition_new_actor` / `baker_mutton_max_new_actor_counts_transition_new_actor` (569+250→570, cap 4)

`cargo check -p ol-server --offline` ok.
