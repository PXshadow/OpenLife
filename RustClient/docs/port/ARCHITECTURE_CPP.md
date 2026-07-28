# Jason C++ OHOL client architecture

**Rust hub:** [README.md](README.md) (this file = upstream reference only)

**Root:** `OneLife/gameSource/`  
**Protocol:** `OneLife/server/protocol.txt`  
**Core monster file:** `LivingLifePage.cpp` (~25k lines) + `LivingLifePage.h` (~900 lines)

---

## 1. Process / page model

```
game.cpp (entry, version, bank init)
  ├─ LoadingPage          progressive bank load / cache rebuild
  ├─ ExistingAccountPage  email / key / server reflector
  ├─ LivingLifePage       **in-game** (map, objects, net, input)
  ├─ RebirthChoicePage / FinalMessagePage / Settings / Review / …
  └─ Editors (object, anim, transition, scene) — tooling, not play
```

`currentGamePage` points at the active `GamePage`; each frame: step + draw.

**Rust map:** `app` state machine with `Phase::Loading | Account | Living | Dead | …`; headless skips draw.

---

## 2. Content “banks” (load once, query forever)

| Bank | Files | Role |
|------|-------|------|
| `spriteBank` | spriteBank.cpp ~1.8k | TGA sprites, IDs, hit maps |
| `objectBank` | objectBank.cpp ~6.4k | object defs, sprites layout, multi-use dummies |
| `animationBank` | animationBank.cpp ~3.8k | per-object anim records |
| `transitionBank` | transitionBank.cpp ~2.9k | craft/use graph client-side hints |
| `categoryBank` | categoryBank.cpp | categories |
| `soundBank` | soundBank.cpp ~1.7k | AIFF/OGG |
| `groundSprites` | groundSprites.cpp | biome tiles / overlays |
| `overlayBank` | overlayBank.cpp | overlays |
| `folderCache` / `binFolderCache` | *Cache.cpp | **fast folder packs** (text/bin concat) |

Boot order (simplified from `game.cpp`): rebuild caches if needed → load sprites → objects → anims → transitions → sounds → enter account/life.

**Haxe parallel:** `Resource.*` + `ObjectBake` + `ObjectData` + `TransitionImporter`.  
**Rust target:** shared **binary content** (CONTENT_BINARY.md) + optional text fallback.

---

## 3. LivingLifePage responsibilities

Single page owns almost all live gameplay:

| Concern | C++ area |
|---------|----------|
| TCP messages | parse `#` frames; handle tags (PU, PM, MC, MX, FX, …) |
| LiveObject table | `LiveObject` struct in header — age, food, holding, anim stacks, path, … |
| Map | chunk grid from MC; apply MX; floors/biomes/objects |
| Movement | pathFind, local prediction, FORCE, KA while moving |
| Actions | click → USE/DROP/REMV/SELF/…; queue if mid-move |
| Drawing | ground, objects, players, anim frames, HUD |
| Speech / emotes | PS, PE, LS |
| UI chrome | hunger boxes, heat arrows, hints, home arrows, yum slips |
| Mini-systems | photos, fitness, life tokens, curses, war/peace, posse |

**Rust rule:** split this god-page into modules; keep a **LiveWorld** + **NetSession** + **RenderView** (optional).

---

## 4. LiveObject (conceptual fields)

From `LivingLifePage.h` (subset — full list in header):

- Identity: `id`, `displayID`, `name`, lineage, eve  
- Vitals: `age`, `ageRate`, food store/capacity, dying, sick  
- Social: curse, war/peace, relationName  
- Holding: `holdingID`, held anim, riding offset, baby wiggle  
- Motion: path, speed, move animation  
- Anim: current/held stacks, fade  
- Screen: `onScreen`, `outOfRange`, sprite load flags  

Client **predicts** some motion; server **authoritative** via PU/PM/FORCE.

---

## 5. Network (client view)

```
connect → SN (players, challenge, version)
  → LOGIN/RLOGIN (hmac password + account key)
  → ACCEPTED | REJECTED
  → stream of server→client tags until disconnect
client→server: MOVE, KA, USE, DROP, REMV, SELF, SAY, EMOT, …
```

Frames are ASCII, `#`-terminated; optional `CM` compressed blocks (Haxe client handles CM).

---

## 6. Caching strategy (critical for “fast start”)

| Mechanism | Purpose |
|-----------|---------|
| `folderCache` | pack many small text files into one data block |
| `binFolderCache` | pack binary TGA/AIFF with pattern filter |
| `regenerateCaches` | rebuild when content version changes |
| groundTileCache | pre-sliced biome tiles on disk |

**Rust + Open Life:** keep the idea, use **versioned little-endian binaries** shared with the server (`OLC1`/`OLT1`/…), not necessarily the C++ cache file format.

---

## 7. Input & pathfinding

- `pathFind.cpp` — grid path for click-to-move  
- LivingLifePage maps screen → tile, issues MOVE with `@seq` and deltas (max 16)  
- Mid-move action queue (same idea as RustClient `nextAction` / actions queue)

---

## 8. Audio / music

- `soundBank` + `SoundUsage`  
- `musicPlayer` / `musicPlayer2` — age-based / journey music  

Headless: no-op backends.

---

## 9. What we do **not** need first

- Full editor suite (`Editor*Page`)  
- Photo upload servers  
- Diff-bundle auto-update (replace with our update story)  
- minorGems OpenGL layer 1:1  

---

## 10. Reading order for AI agents

1. `protocol.txt`  
2. `LivingLifePage.h` (`LiveObject`)  
3. Message handlers in `LivingLifePage.cpp` (grep `PU`, `MC`, `MX`)  
4. `objectBank` / `animationBank` load APIs  
5. `folderCache` / `binFolderCache`  
6. Cross-check Haxe `Client.hx` + `ClientTag.hx` + `Render.hx`
