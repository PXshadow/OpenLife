# Haxe Open Life client architecture

**Rust hub:** [README.md](README.md) (this file = reference for fast load / atlas ideas)

**Root:** `C:\OhOl\OpenLife\openlife\client\`  
**Supporting:** `resources/`, `data/`, `graphics/TgaData.hx`, `engine/`

Not a full Jason port — a **smaller, modern** client with valuable **performance ideas** we must absorb.

---

## 1. Module map

| File | ~LOC | Role |
|------|------|------|
| `Main.hx` | 20 | entry |
| `Game.hx` | 46 | game shell |
| `Client.hx` | 345 | TCP, `#` frames, CM decompress, LOGIN, keepalive |
| `ClientTag.hx` | 111 | server→client tags (richer enum than bare strings) |
| `World.hx` | 62 | map state stub/layer |
| `Object.hx` | 105 | drawable object instance |
| `Render.hx` | 215 | ground + object draw, atlas build |
| `SpriteBatch.hx` | 246 | custom sprite batch (Heaps/h2d) |
| `BinPack.hx` | 115 | texture atlas packing |
| `Sound.hx` | 63 | AIFF mono load |
| `Fps.hx` | 200 | timing / FPS |
| `ClientSettings.hx` | 3 | settings hook |

---

## 2. Fast asset pipeline (steal this)

```
OneLifeData7/
  objects/*.txt
  sprites/*.tga + *.txt
  animations/
  groundTileCache/     precomputed biome tiles
  sounds/
       │
       ▼
Resource.hx            path resolution (objects/N, sprites/N.tga, …)
ObjectBake.hx          multi-use dummy expansion + bake.res stamp
TgaData.hx             TGA decode
BinPack + SpriteBatch  pack many sprites into 4096² atlases
Render.addObject       instance sprites with age/flip/color
```

### ObjectBake (multi-use dummies)

Haxe expands `numUses > 1` into dummy object IDs so rendering can show intermediate use states without re-parsing every session. Stamp file `bake.res` vs `nextObjectNumber.txt` detects stale bake.

**Rust:** generate dummies once into **OLC1** binary (or side table); never re-walk thousands of text files at interactive startup.

### Resource layout

`Resource.objectData`, `spriteImage`, `animation`, `ground`, `sound`, `music` — single place for path rules. Mirror in Rust `content::paths`.

### Ground

Uses **groundTileCache** biome slices (`biome_{id}_x{i}_y{j}{a}.tga`) when present — huge win vs stitching raw ground every frame.

---

## 3. Network (Haxe)

`Client.hx`:

- Read until `#`, or CM compressed payload  
- HMAC login similar to official  
- Tag dispatch via `ClientTag`  
- Optional relay socket  

Cross-check tags with Rust `parse.rs` / server `ol-protocol`.

---

## 4. What Haxe does **not** fully replace

- Full LivingLife HUD / all messages  
- Official pathFind nuances  
- Complete animation bank fidelity  
- Account/reflector UX  

Use Haxe for **assets + batching + tags list**; use C++ for **behavior completeness**.

---

## 5. Port policy

| Topic | Prefer |
|-------|--------|
| Wire tags list | Haxe `ClientTag` + C++ handlers + protocol.txt (union) |
| Sprite atlas | Haxe BinPack approach (or modern GPU atlas) |
| Object dummy uses | Haxe ObjectBake semantics |
| Live simulation feel | C++ LivingLifePage |
| Headless tests | Rust RustClient (extend) |
