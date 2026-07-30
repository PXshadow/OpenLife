# C++ ground biome draw (Jason OneLife) — full step-by-step

**Source:** `jasonrohrer/OneLife`  
- `gameSource/LivingLifePage.cpp` ~7151–7444 (biome pass) + ~7629–7721 (full-view overlay)  
- `gameSource/groundSprites.cpp` (sheet load + soft/square cache generation)

**CELL_D** = 128 object units per map tile (constant).

This document is the **authoritative** description of how the original client paints ground. The Rust client must match every rule listed under **Parity checklist**.

---

## 1. Asset pipeline (`groundSprites.cpp`)

### 1.1 Source sheet
- File: `graphics/ground_%d.tga` for biome `b` (unknown biome uses a special end-of-array set / `ground_U`).
- Dimensions must be multiples of `CELL_D`.
- Sheet is sliced into a grid:
  - `numTilesWide = w / CELL_D`
  - `numTilesHigh = h / CELL_D`
  - Typical: **4 × 4** variations.

### 1.2 Two sprite kinds per variation `(tx, ty)` in the sheet

| Kind | Cache file | Pixel size | How built |
|------|------------|------------|-----------|
| **Soft** `tiles[ty][tx]` | `groundTileCache/biome_{b}_x{tx}_y{ty}.tga` | **2×CELL_D = 256²** | Sample 2×CELL_D region of sheet **centered** on the 1× cell (wrap sheet edges). Alpha = soft circular-ish falloff from cell center + blur; solid core so corners aren’t undercut. |
| **Square** `squareTiles[ty][tx]` | `…_square.tga` | **CELL_D = 128²** | Exact `getSubImage(tx*CELL_D, ty*CELL_D, CELL_D, CELL_D)` — **hard rectangle**, full opacity. |
| **Whole sheet** `wholeSheet` | (in memory) | full sheet | Used when a full 4×4 (or tW×tH) same-biome region can be drawn in one sprite. |

Soft tiles are **not** simply scaled squares: they are larger than one cell with **soft alpha edges** so adjacent biomes blend when soft tiles overlap.

---

## 2. View / loop setup (`LivingLifePage.cpp` draw)

1. Compute visible map index range around the camera (`xStartFloor…xEndFloor`, `yStartFloor…yEndFloor`). Bounds intentionally loose so **off-map** cells can still show the **unknown** biome sheet.
2. Clear `mMapCellDrawnFlags[mapI]` for the whole map.
3. **Two passes** over cells (`pass = 0..1`), nested as:
   ```text
   for pass in 0..1:
     for y = yEndFloor down to yStartFloor:   // high → low
       for x = xStartFloor to xEndFloor:
         … biome draw (pass 0) or culvert stones (pass 1) …
   ```
4. Pass **0** = biome sprites. Pass **1** = culvert fault-line stones only (not biome fill).

---

## 3. Per-cell position (pass 0) — **same for soft and square**

```cpp
// Map index (x,y) → screen (object units, y-up)
int screenY = CELL_D * ( y + mMapOffsetY - mMapD / 2 );
int tileY   = -lrint( screenY / CELL_D );   // BEFORE offset
// slight offset to compensate for tile overlaps / centering
screenY -= 32;   // CELL_D/4

int screenX = CELL_D * ( x + mMapOffsetX - mMapD / 2 );
int tileX   =  lrint( screenX / CELL_D );   // BEFORE offset
screenX += 32;   // CELL_D/4

doublePair pos = { (double)screenX, (double)screenY };
```

**Critical details:**
- Variation indices `setX` / `setY` use `tileX` / `tileY` **before** the ±32 offset.
- **Draw position `pos` is used for BOTH `squareTiles` and `tiles` (soft).** There is **no** separate “flush cell rect” path for squares.
- Offset is **+32 X, −32 Y** (object units) from the grid-aligned corner/base of the cell.

In **tile units** (divide by CELL_D), if grid base for map cell `(mx, my)` is `(mx, my)` in tile space matching C++ index:

| | |
|--|--|
| Base (pre-offset) | ≈ `(mx, my)` in tile space (after map offset) |
| Draw center | ≈ `(mx + 0.25, my − 0.25)` in C++ y-up object space |

Soft-FB with world-Y up / screen-Y down must convert this consistently via `world_to_screen`.

---

## 4. Biome / sheet selection

```cpp
int b = inBounds ? mMapBiomes[mapI] : -1;
setDrawColor(1,1,1,1);
GroundSpriteSet *s = groundSprites[b];           // if valid
if (b == -1 || s == NULL) {
    s = groundSprites[last];                     // unknown sheet
    if (s was missing for real biome id) {
        setDrawColor( getXYRandom(b,b),
                      getXYRandom(b,b+100),
                      getXYRandom(b,b+300), 1 ); // multiply tint
    }
}
```

- Valid biome → white draw color, that biome’s sheet.
- Unknown / missing set → unknown sheet + optional **random RGB multiply** via `getXYRandom`.

**There is no solid-color rectangle underfill per cell in C++.**

---

## 5. Variation index into the sheet

```cpp
int setY = tileY % s->numTilesHigh;
int setX = tileX % s->numTilesWide;
// fix negative mods
```

Typically 4×4 → same family as  
`biome * 16 + abs(tileX % 4) + abs(tileY % 4) * 4` for cache keys.

---

## 6. Whole-sheet fast path (pass 0 only)

If `setX == 0 && setY == 0` **and** the entire `numTilesWide × numTilesHigh` block of map biomes (plus a 1-cell border) equals `b`:

1. Compute center of the big rectangle covering that block.
2. `drawSprite( s->wholeSheet, sheetPos )`.
3. Mark every cell under the sheet in `mMapCellDrawnFlags` so they are **skipped** later.

This paints a seamless sheet over large same-biome regions (no per-cell seams).

---

## 7. Per-cell soft vs square (pass 0, if not already drawn)

Neighbors (OOB ⇒ biome `-1` ≠ `b`):

| Name | Map sample |
|------|------------|
| `leftB` | `(x - 1, y)` |
| `aboveB` | `(x, y + 1)` |
| `diagB` | **`(x + 1, y + 1)`** — **above-right / NE**, not NW |

```cpp
if (leftB == b && aboveB == b && diagB == b) {
    // Interior: square tile (saves fill / hard edges OK)
    if (!(floorAt && floorR && floorB && floorBR))
        drawSprite( s->squareTiles[setY][setX], pos );
} else {
    // Border / transition: soft 2× tile
    if (!(floors cover 3×3 completely))
        drawSprite( s->tiles[setY][setX], pos );
}
```

`drawSprite` **centers** the sprite on `pos`.

| Sprite | Size | Coverage around `pos` |
|--------|------|------------------------|
| Square | 128×128 | ±64 object units (½ cell) → **edge-abuts** neighbors’ squares when `pos` is spaced 128 apart |
| Soft | 256×256 | ±128 object units (1 cell) → **overlaps** neighbors by ~½ cell; soft alpha blends biomes |

Floor skip: if floors fully cover the relevant cells, ground draw is skipped (perf).

---

## 8. After floors: full-view ground overlay (~7629+)

**Not** per-cell `ground_t0…t3` at map coords.

- Uses large `mGroundOverlaySprite[0..]` tiles (`graphics/ground_t0..t3.tga`, typically 1024²).
- Snaps a grid to `lastScreenViewCenter`.
- **Multiplicative pass** with **additive texture coloring**:
  - `toggleMultiplicativeBlend(true)` + `toggleAdditiveTextureColoring(true)`
  - `setDrawColor(multAmount, multAmount, multAmount, 1)` with `multAmount = 0.15`
  - Fragment = `clamp(texture + multAmount)`, then `dst *= fragment`
  - **Not** `dst *= texture * multAmount` (that crushes the frame to ~10% brightness)
- **Additive pass**: `toggleAdditiveBlend(true)`, `setDrawColor(1,1,1, addAmount)` with `addAmount = 0.25`
- Together this is a light texture wash / film grain, not a global darken.

---

## 9. Order relative to rest of frame

1. Clear / sky  
2. **Ground biomes** (this doc)  
3. Floors  
4. **Ground overlay** (screen-locked)  
5. Objects / players / HUD  

---

## 10. Parity checklist (Rust vs C++)

| # | C++ rule | Must match in Rust |
|---|----------|--------------------|
| A | No per-cell solid underfill | Do not paint opaque biome rectangles under every cell (or only as last-resort void) |
| B | Soft + square share **same** `pos` (+32 X, −32 Y) | Same world/screen center for both kinds |
| C | Square when L + up + **NE diag** same | `diag = (tx+1, ty+1)` |
| D | Soft when not (C) | Soft 2×CELL_D with soft alpha edges |
| E | Soft size 2×, square size 1× | `cells=2.0` vs `cells=1.0` (or exact pixel equiv.) |
| F | `drawSprite` centered on `pos` | Centered blits, not “flush bottom-left” for soft |
| G | Whole sheet when 4×4 (tW×tH) uniform + corner | Optional perf; improves seamlessness |
| H | Unknown sheet + `getXYRandom` tint | Same as missing `ground_N` |
| I | Variation from pre-offset tileX/Y | Stable 4×4 / sheet wrap |
| J | Screen-space ground overlay after floors | Not sparse per-cell Haxe overlays |
| K | Y loop high→low | Draw order for overdraw |

---

## 11. Why a “grid of squares” appears if wrong

| Mistake | Visual |
|---------|--------|
| Solid plate under every cell + misaligned squares | Hard grid of underfill color |
| Soft offset on square only / square flush but soft wrong | Broken borders or interior grid |
| Wrong diag (NW vs NE) | Too many soft or too many square tiles |
| Soft blitted as 1× or square as 2× | Stretch / hard edges |
| Per-cell Haxe `ground_tN` | Strange repeating tiles every 8 cells |
| Missing whole-sheet on large biomes | More visible 4×4 variation seams |

---

## 12. References (line anchors in downloaded copies)

Local working copies for port work (not product runtime):

- `docs/port/_LivingLifePage.cpp` (from jasonrohrer/OneLife)  
- `docs/port/_groundSprites.cpp`

Key anchors:
- Biome loop start: `LivingLifePage.cpp` ~7151  
- Position ±32: ~7166–7185  
- wholeSheet: ~7241–7307  
- square vs soft: ~7345–7389  
- Overlay: ~7629+  
- Soft generation: `groundSprites.cpp` ~208–415  

---

## 13. Rust client parity audit

Implementation: `RustClient/src/render.rs` + `ground_sprites.rs`.

| # | C++ rule | Rust status |
|---|----------|-------------|
| A | No solid per-cell underfill | **Match** — plates only if biome has **no** sheet |
| B | Soft + square share same `pos` (+32/−32) | **Match** — `(tx+0.25, ty−0.25)` |
| C | Square when L + up + **NE** diag same | **Match** |
| D | Soft when not (C) | **Match** — 2×CELL_D |
| E | Soft 2× / square 1× centered | **Match** |
| F | Soft/square TGA cache | **Match** |
| G | **wholeSheet** when setX=setY=0 + region uniform | **Match** — `ensure_whole_sheet` + border test + drawn flags |
| H | Unknown + `getXYRandom` | **Match** |
| I | Variation tileX/Y | **Match** |
| J | **Full-view ground overlay** after floors | **Match at Brightness 100%** — mult = additive texture coloring `dst *= clamp(tex+0.15)`; add `α=0.25`. Settings slider blends to legacy dark (`tex*0.15`) at 0% (default). |
| K | Y loop high→low | **Match** |
| L | **Floor cover skip** (square 2×2 / soft 3×3) | **Match** — `is_covered_by_floor` |
| M | Floors before overlay, objects after | **Match** |

### Historical “grid” cause

Opaque underfill plates + missing wholeSheet + missing screen overlay. All three are addressed.