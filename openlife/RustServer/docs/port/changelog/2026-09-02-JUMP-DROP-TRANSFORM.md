# 2026-09-02 JUMP dropPlayer — ignore xy, drop at carrier tile

**status:** **DONE**

Haxe `GPI.jump` ignores payload xy. Held baby `dropPlayer` at carrier tile (walkable); blocked keeps hold. Dual PU + FRAME.

Tests: `jump_emits_pu_note` (no teleport), `jump_releases_held_baby_from_mother` at mother xy.
