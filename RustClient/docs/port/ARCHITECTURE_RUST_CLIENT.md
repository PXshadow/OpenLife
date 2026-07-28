# Rust client architecture

**Crate:** `ohol-headless` at `C:\OhOl\OpenLife\RustClient`  
**Binaries:** `ohol-headless`, `ohol-client` (default features: `gpu` + `audio`)  
**Status of features:** [TODO_PORT.md](TODO_PORT.md) · hub: [README.md](README.md)

This doc explains **structure and data flow**. It does not repeat the P1–P5 checklist.

---

## 1. Design goals

| Goal | Meaning |
|------|---------|
| **Parity** | LivingLife gameplay on official-compatible servers |
| **Clean modules** | Domain split; no 25k-line god file |
| **Fast start** | Binary cache; lazy TGA/AIFF/OGG; optional pixel atlases |
| **Headless first** | Logic testable without a window |
| **Shared content** | Same OLC1/OLT1 as Rust server (`ol-binary`) |
| **Min deps** | See root `Cargo.toml` |

---

## 2. Layout (actual)

Single package today (multi-crate split is optional later).

```
RustClient/
  src/
    lib.rs              # re-exports
    main.rs             # ohol-headless CLI
    bin/ohol_client.rs  # soft-FB window
    …modules…
  docs/port/            # this kit
  .grok/workflows/      # automation scripts
```

### Modules by domain

| Domain | Modules | Role |
|--------|---------|------|
| **Wire** | `frame`, `tags`, `parse`, `login`, `wire_log` | `#` framing, CM inflate, SN/HMAC, tag parse, transcripts |
| **Session** | `session` | TCP, apply inbound, FM batch, food/heat, step anims/sounds, send actions |
| **World** | `live_object`, `client_map`, `emotion` | Players, clothing, speech/PE, map tiles |
| **Motion** | `move_state`, `multi_move_ext`, `map_global_offset`, `pathfind` | MOVE seq, multi-hop, wire offset policy, A* |
| **Intent** | `actions`, `click_tile`, `rmb_action`, `hover_pick` | Encode + path-to-adjacent + soft-FB pick |
| **Content text** | `content`, `category_bank` | Objects/transitions text, categories, find_ptrans |
| **Content binary** | `content_binary` + path `ol-binary` | Bake/load OLC1/OLT1; dummies; prefer_cache |
| **Banks** | `sprite_bank`, `anim_bank`, `ground_sprites`, `overlay_bank`, `sound_bank`, `music_bank` | OLS1/OLSA, OLA1, OLG1/OLGA, OLO1, OLSN, music |
| **Draw** | `render`, `hud`, `anim_draw`, `binpack`, `tga` | Soft FB, HUD, dual-fade packs, atlas pack, TGA |
| **Product UI** | `client_screen`, `load_progress`, `account_page`, `settings_page` | P5 screens Account/Loading/Playing/Death/Settings |
| **Tools** | `load_bench`, `probe_*` | Timing + probe helpers |

C++/Haxe file mapping: [FILE_MATRIX.md](FILE_MATRIX.md). Symbols: [CALL_INDEX.md](CALL_INDEX.md).

---

## 3. Runtime topology

### Headless

```
CLI / test
  → SessionConfig (.env / flags)
  → connect + LOGIN
  → poll_event / drain
  → LiveWorld + ClientMap + MoveState
  → scripted click_tile / encode_*
  → wire log / asserts
```

### Soft-FB (`ohol-client`)

```
minifb loop
  ├─ net: session.poll / step_anims / step_move
  ├─ input: screen_to_tile → click_tile / rmb / hold-slide
  ├─ hover_pick (hitMap, clothing, contained)
  ├─ SceneRenderer.draw → Framebuffer
  └─ HUD + speech + optional ClientScreen overlays
```

Interpolation and draw clocks stay out of pure protocol parse.

---

## 4. Core types (conceptual)

```text
ClientSession
  world: LiveWorld
  map: ClientMap
  move: MoveState
  content: ClientContent
  anims / sprites / sounds / ground / music (banks)
  food / heat / our_id / …

LiveWorld
  players, location_speech, says_pointers, …

ClientCommand / ObjectAction
  Move | Use | Drop | Remv | Self | Sremv | Say | Emot | Ka | Force | …

InboundMessage
  PU PM MC MX PS PE FX HX … (parse.rs)
```

---

## 5. Content load path

```
OHOL_CONTENT_DIR/   (or well-known OneLifeData7)
  objects/ transitions/ sprites/ animations/ …
  cache/
    manifest.json
    olc1_objects.bin      # + dummies, sim fields, use-vis (v6)
    olt1_transitions.bin  # expanded flag, max-use, switch
    ola1_anims.bin
    ols1_sprites.bin      # meta only
    olg1_ground_index.bin
    olo1_overlays.bin
    olsn_sounds.bin       # index only
    olsa_sprite_atlas.bin # OPTIONAL full sprite pages
    olga_ground_atlas.bin # OPTIONAL full ground pages
```

```
boot: load_prefer_cache*
  if manifest / dataVersion OK → load blobs (fast)
  else → text and/or rebake
play: ensure(id) → TGA/AIFF/OGG as needed → runtime BinPack
optional: load_prefer_atlas_cache → sealed pages, no re-TGA for packed ids
```

Formats: [CONTENT_BINARY.md](CONTENT_BINARY.md). Timings: [LOAD_PERF.md](LOAD_PERF.md).

HUD chrome TGAs come from `OneLifeGameSourceData/graphics/` (not OLC1), with procedural fallbacks.

---

## 6. Feature flags

| Feature | Default | Effect |
|---------|---------|--------|
| `gpu` | **on** | `ohol-client` + minifb + winit/pixels present |
| `audio` | **on** | cpal device output |
| (none) | `--no-default-features` | pure headless, no window/audio deps |

Runtime off switches (no rebuild): Settings → **Graphics** = Soft; Settings → **Audio device** = OFF
(`graphics=soft` / `audio=0` in ini; `OHOL_GRAPHICS` / `OHOL_AUDIO` / `OHOL_AUDIO_DISABLE`).

Music decode uses **lewton** always compiled (lazy on ensure only).

---

## 7. Testing

| Level | How |
|-------|-----|
| Unit | `cargo test --lib` (banks, pathfind, click_tile, parse, …) |
| Fixture peer | `ohol-headless --self-check` |
| Live | probes against server `:8005` |
| Load | `--bench-load` |
| Goldens | anim sample vectors; multi-use dummy id oracle (see TODO P4#34–35) |

---

## 8. Code rules

1. Split by domain — no LivingLifePage dump.  
2. Pure `apply_*` / encode paths unit-tested.  
3. `// C++: file:tag` on non-obvious ports.  
4. Headless green on every meaningful change.  
5. Prefer sealed restored atlases (`BinPack::sealed`) so cache pages are not overwritten.

---

## 9. Server relationship

| Shared | Client-only | Server-only |
|--------|-------------|-------------|
| OLC1/OLT1 (`ol-binary`) | OLA1/OLS1/OLSA/OLG1/OLGA/OLSN/OLO1, render, input | sim, AI, saves, OLW1 |
| LOGIN key shape | soft-FB pages, pan/listener | ticket authority |

---

## 10. Target multi-crate (optional later)

Not required for play. Possible split: `ol-client-proto`, `ol-client-content`, `ol-client-world`, `ol-client-net`, render/audio features. Keep until module boundaries hurt compile times.
