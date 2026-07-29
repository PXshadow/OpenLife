# Jason C++ player draw — reference checklist

**Sources** (jasonrohrer/OneLife `gameSource/`, tree used for this write-up):

| File | Role |
|------|------|
| `LivingLifePage.cpp` | Map row passes, `drawLiveObject`, rideable/held interleave |
| `LivingLifePage.h` | `LiveObject` anim clocks / clothing / held fields |
| `animationBank.cpp` | `drawObjectAnim` person loop, clothing attach, HoldingPos |
| `objectBank` | `clothingOffset`, body-part indices, `drawBehindPlayer`, `spriteBehindPlayer` |

Line numbers refer to upstream `master` as of sparse-clone for this port task.

---

## 1. Map / player interleave (same world row)

Loop rows **high Y → low Y** (north first). Per row (`LivingLifePage.cpp` ~8213–8864):

| Step | What | Notes |
|------|------|--------|
| 0 (screen-wide before row loop) | **Sprites marked behind player** only | `anySpritesBehindPlayer` → `prepareToSkipSprites(o, true)` then `drawMapCell` (~8242–8250). Whole object may still draw later. |
| 1 | **Whole `drawBehindPlayer` objects** | Roads, etc. If `anySpritesBehindPlayer`, skip already-drawn behind layers (~8288–8301). |
| 2 | **Players + moving map cells** (depth queue) | Sorted by true Y; adults then recently-dropped babies (~8371–8702). Person via `drawLiveObject` → `drawObjectAnim`. Held rideable front may defer. |
| 3 | **Permanent non-wall front** | `!drawBehindPlayer && !wallLayer && permanent` (~8711–8744). |
| 4 | **Non-permanent non-wall front** | Pickups, tools on ground (~8747–8780). |
| 5 | Deferred held-on-top (sliding) | (~8783–8788). |
| 6 | **Wall layer, not frontWall** | (~8793–8827). |
| 7 | **frontWall** (e.g. walls with signs) | (~8829–8863). |

**Rust map:** `DrawLayer` enum Floor → BehindPlayer → Player → FrontPermanent → FrontNonPermanent → FrontWall → FrontFrontWall; `push_map_object_draw_items` splits `BehindPlayerOnly` / `NotBehindPlayer`.

---

## 2. Person anim clocks (`drawLiveObject`)

- `animationFrameCount` / `lastAnimationFrameCount` → `timeVal = frf * count / 60` (~5354–5368).
- Dual-fade: if `lastAnimFade > 0`, draw **last** type with `animFade = lastAnimFade`, target = **cur** (~5362–5368).
- Fade step elsewhere: `lastAnimFade -= 0.05 * frameRateFactor` (map ground ~4555; person same constant in step).
- Age: `computeCurrentAge(inObj)` for age-gated sprites (~5377).
- Pack select: moving / eating / doing / ground / extra (PE) — see `addNewAnim*` (~2671+).

---

## 3. Limb hide while holding

`getArmHoldingParameters(heldObject, &hideClosestArm, &hideAllLimbs)` (~5401):

| Value | Meaning |
|-------|---------|
| `hideClosestArm == 0` | Normal hands; HoldingPos = **back hand** |
| `±1` | Hide that arm chain; HoldingPos = **body** |
| `-2` | Freeze arms (bulky); HoldingPos = **body** |
| `hideAllLimbs` | Hide legs (rideable path also freezes arms via frozenArmType) |

Rideable: person may draw under vehicle; vehicle behind layers first (`prepareToSkipSprites(held, true)` ~5716).

---

## 4. Clothing attach (critical)

**Not** “person feet + offset” alone. Offsets are applied to the **animated body-part sprite position**, then to person `inPos`.

| Slot | C++ field | Anchor body part | Contained vector index |
|------|-----------|------------------|------------------------|
| Hat | `clothing.hat` | **Head** (`animHeadPos + clothingOffset`) ~3549–3601 | `[0]` |
| Tunic | `clothing.tunic` | **Body** sprite pos + offset ~2801–2831; drawn ~2996–3034 | `[1]` |
| Front shoe | `clothing.frontShoe` | **Front foot** ~3106–3134, draw ~3504–3540 | `[2]` |
| Back shoe | `clothing.backShoe` | **Back foot** ~2769–2798, draw ~3464–3502 | `[3]` |
| Bottom | `clothing.bottom` | **Body** ~2863–2891, draw ~2958–2995 | `[4]` |
| Backpack | `clothing.backpack` | **Body** ~2893–2920, draw ~3035–3071 | `[5]` |

**Formula** (hat simplified; shoes/tunic same with foot/body `spritePos` and rot):

```text
offset = clothing->clothingOffset
if flipH: offset.x *= -1
// optional rotate offset by body-part rot delta
cPos = animPartPos + offset + inPos
drawObjectAnim(clothing, clothingAnimType, cPos, worn=true, cont[])
```

**Clothing anim type:** if person anim ≠ `moving`, clothing uses **`held`** type (~2031–2039).

**Interleave (within person):**

1. Person sprites bottom→top (age gates, parent chain, skip worn/invis flags).
2. When **back foot** drawn: compute backShoePos; later draw shoe on foot.
3. When **body** drawn: compute tunic/bottom/backpack positions.
4. When **top back arm**: bodyEmot; draw **bottom → tunic → backpack** (under top of back arm).
5. When **front foot**: front shoe.
6. After all person sprites: **hat on head**.
7. **headEmot** after hat.
8. Eyes/mouth/other emotes at head/eyes anchors during head/eyes layers.

**Worn flags:** `spriteInvisibleWhenWorn == 1` skip if worn; `== 2` skip if not worn (~2740–2753).

---

## 5. Contained in clothing / map containers

- Clothing contained: per-slot vector, drawn as `numCont` + ids into `drawObjectAnim` of the clothing object (~2960+).
- Map containers: drawn inside `drawMapCell` / object stack with `slotPos` + slot anim (not re-specified here; same bank path).

---

## 6. Held objects

- Non-rideable: `computeHeldDrawPos(HoldingPos, personPos, held, flip)` (~5560).
- Rideable: vehicle at **person pos**; behind layers under rider, front over (~5443–5716, ~8598–8668).
- Baby held: separate pack; may stack wound object on top (~8581–8590).

---

## 7. PE / emotion order

| Phase | When | Anchor |
|-------|------|--------|
| bodyEmot | Under top back arm | `animBodyPos` |
| eyeEmot | On eyes layer | head + `mainEyesOffset` |
| mouthEmot | On head if mouth exists | `animHeadPos` |
| otherEmot | On head | `animHeadPos` (can hide head/body) |
| headEmot | After hat | head |

Mouth skip when any emot has `mouthEmot != 0`.

---

## 8. Age-gated person sprites

- Each sprite: `ageRange` start/end; `-1,-1` = always (~objectBank / object txt).
- Body-part indices (`bodyIndex`, `headIndex`, feet) pick the **top-most matching** layer visible at age (`getBodyIndex` etc.).

---

## 9. Rust implementation checklist

- [x] Map layer enum matches C++ front sub-order (P3#23).
- [x] Dual-fade clocks + unused fade channel → opacity 1.
- [x] Clothing attach to **head/body/foot animated positions** (not feet-only).
- [x] Clothing draw order: shoes/tunic/bottom/backpack/hat with contained.
- [x] Full age-visible person layers paint (pose all layers; age only skips draw).
- [x] Age body/head offsets (`ageControl.cpp`).
- [x] Behind-player sprites under player; front permanent/non-perm/wall after.
- [x] Max zoom still full figure (`ZOOM_MAX` integration test).

---

## 10. Reading order for implementers

1. This file  
2. `LivingLifePage.cpp` ~8213–8864 (map order)  
3. `LivingLifePage.cpp` ~5349–5700 (`drawLiveObject`)  
4. `animationBank.cpp` ~2030–3610 (person loop + clothing)  
5. Rust `render.rs` `SceneRenderer::draw` + `draw_object_with_pack`
