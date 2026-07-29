# Jason vs Rust player draw — step audit

Sources: `jasonrohrer/OneLife` `animationBank.cpp`, `LivingLifePage.cpp`,
`ageControl.cpp`, `spriteBank.cpp`; `jasonrohrer/minorGems` `SpriteGL.cpp`,
`doublePair.cpp`. Line numbers as of sparse fetch for this audit.

## Pipeline order (person)

| # | Jason | Rust | Match? |
|---|--------|------|--------|
| 1 | `LivingLifePage::drawLiveObject` clocks, age, arm-hide, clothing set | `SceneRenderer` player branch: age, `arm_holding_parameters`, `person_anim_pack`, clothing list | yes |
| 2 | Optional rideable behind layers | `SpriteLayerFilter::BehindPlayerOnly` before person | yes |
| 3 | `drawObjectAnim(displayID, … clothing, clothingContained)` | `draw_object_with_pack_ex(…, worn_clothing)` | yes |
| 4 | Per-layer: frame time + pauses | `SpriteAnimParam::frame_time` / pack sample | yes |
| 5 | Base `spritePos = obj->spritePos[i]` | `spr.x/y` | yes |
| 6 | Age head/body offset on those indices only (`ageControl.cpp`) | `age_head_offset` / `age_body_offset` | yes |
| 7 | Anim osc/offset × `inAnimFade` (+ target blend) | `sample_sprite_pack` | yes |
| 8 | Fade hardness formula → `workingSpriteFade` | `sample_fade` | **see §fade** |
| 9 | `rot` + rock + frozen-rot; rotCenterOffset pivot | same in `sample_sprite_pack` + pose | yes |
| 10 | `workingDelta = posed − rest` | same before parent walk | yes |
| 11 | Parent walk-up with local deltas + rot compound | `apply_jason_parent_chain` | **fixed this audit** |
| 12 | FlipH: `pos.x *= -1`, `rot *= -1` | screen scale flip + rot negate | yes |
| 13 | `pos = spritePos + inPos` | `screen + posed * scale` (Y flip) | yes |
| 14 | `drawSprite` applies **centerAnchor** | geometric center from anchor | **see §anchor** |
| 15 | Clothing: bottom/tunic/backpack under topBackArm; shoes on feet; hat after | interleave in `draw_object_with_pack_ex` | yes |
| 16 | Map y high→low, layers behind/player/front | `sort_y` DESC + `DrawLayer` | yes |

## § Parent chain (critical)

Jason (`animationBank.cpp` ~2505–2625):

```text
workingDeltaPos[i] = workingSpritePos[i] - obj->spritePos[i]
workingDeltaRot[i] = workingRot[i] - obj->spriteRot[i]

for each sprite i:
  pos = workingSpritePos[i]
  rot = workingRot[i]
  p = parent[i]
  while p != -1:
    if workingDeltaRot[p] != 0:
      angle = -2π * workingDeltaRot[p]
      rot += workingDeltaRot[p]
      pos += rotate(workingDeltaPos[p], -angle)
      childOff = pos - obj->spritePos[p]   // parent REST
      pos += rotate(childOff, angle) - childOff
    else:
      pos += workingDeltaPos[p]
    p = parent[p]
```

`rotate` is minorGems standard 2D: `(c*x - s*y, s*x + c*y)`.

**Was:** Haxe-style re-parent from roots using already-compounded parent world pose.  
**Now:** exact Jason walk-up on frozen local deltas (`apply_jason_parent_chain`).

Zero-rotation case (rest / pure translate): both formulas agree  
`child_final = child_local + Σ parent_deltas`.

## § Center anchor

`SpriteGL.cpp` (~522–539), OpenGL Y-up:

```text
centerOffset = mCenterOffset * scale
if flipH: centerOffset.x = -centerOffset.x
posX = inPosition.x - centerOffset.x
posY = inPosition.y + centerOffset.y   // + Y offset
```

Rust (object Y-up, then screen Y-down):

```text
geo = (attach.x - ax, attach.y + ay)
screen = (sx + geo.x * s * flip, sy - geo.y * s)
```

Matches SpriteGL after Y flip. (Wrong `-ay` floated hair / neck gap.)

## § Fade unused channel

Content often stores `fadeMin=fadeMax=0` with `fadeOsc=0`. Strict C++ formula yields **0** alpha.  
Defaults when loading unset fields are `fadeMax=1`. Gameplay shows opaque body.

Rust: if osc≈0 and min≈0 and max≈0 → treat as **1.0** (opaque). Documented intentional; matches playable Jason, not a naïve zero product.

## § Clothing attach

Jason: after parent chain, `animBodyPos` / feet / head;  
`clothingOffset` flip X, rotate by part rot, `cPos = part + offset + inPos`;  
draw order bottom → tunic → backpack under `topBackArmIndex`.

Rust: `PersonAnchors` post-chain; `clothing_screen_pos` flip+rotate; interleave same slots.

## § Holding / rideable / PE

Jason: `getArmHoldingParameters`, HoldingPos from back hand or body;  
rideable person-under-vehicle with behind/front split; PE body under arm / face after hat.

Rust: same hooks; PE still simplified vs mid-arm bodyEmot interleave (residual).

## Residual (not body-assembly)

- Full PE mid-arm bodyEmot interleave
- Frozen-arm full-layer anim override detail
- Pixel-identical OpenGL filtering
