# AiBase doTimeStuffHelper think order

Heartbeat: 2026-09-21T19:30:00Z

NPC think in `npc_ai.rs` follows Haxe `doTimeStuffHelper` L416–L873:

Before L599: L428 mid-path skip → countSeeds → cleanUp → heldBy → escape → **drop / close-use (L497–510)** → waitingTime → **hungry infant L523 (walk 5 then 3, always return)** → **child-with-mother L533 (walk, nested handleTemperature, nice-baby; non-nice close falls through)** → wounded → death → isEating (no moving skip) → feed child → switchCloths → consider-food (deadly skip, nested attack/kill/graves/clothes/fire with live best-keeper) → far use → container → pickup food (leftover foodTarget kept) → temp → attack (GetClosePlayerTarget) → stay-close (doStuff) → killAnimal → feed player in need → follow → L599 moving skip.

After L599: foundFamily / allyUp → CriticalCraft L609–629 → fire → knife → second carrot → clothes pickup → temperature → lambs → hunting → sharpie → graves → bucket → skewer 139+2832 → craft queue → high clothing, medium if age>10 → assigned → doCriticalStuff → low work → home → drop held → idle.

Removed extra tails: bottom-up `evaluate_nearby_crafts`, peel `force_drop_at_feet` before home, hunter HIT, 6-dir explore. Early clothing no longer runs assigned-TAILOR medium under age 10 (that is L749 assigned job).
