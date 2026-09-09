# 2026-09-02 TH-BLOCKS-REMOVE + TH-FORTIFY-APPLY

**status:** **DONE**

## Closed

- Haxe `ObjectData.blocksRemove` (ServerSettings 987/988) on live DROP-into-container, REMV, PUTNEST, and clothing nest put/SREMV. Closed/locked chests no longer store or yield contents until opened.
- Haxe `doCommandHelper` fortify (L189–211): USE held matching `fortificationObjId` costs `floor(value * FortificationCosePerHit)` (compiled 1.0), `hits -= value`, `countObj += 1`, consume held, say cost/need-coins. No transform.

## Tests

- `drop_and_remv_blocked_on_closed_chest`
- `fortify_apply_consumes_held_and_coins` / `fortify_apply_refuses_without_coins`
