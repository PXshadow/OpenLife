# Shared content binary format (client + server)

**Hub:** [README.md](README.md) · **priority status:** [TODO_PORT.md](TODO_PORT.md)

**Goal:** Super-fast startup by loading **pre-baked** blobs instead of parsing thousands of text files every launch.

| Consumer | Blobs |
|----------|--------|
| **Server** `ol-content` | OLC1/OLT1 via shared **`ol-binary`** (+ prefer_cache) |
| **Client** | OLC1 **v6** / OLT1 **v2** / OLA1 / OLS1 / OLG1 / OLO1 / OLSN (+ optional **OLSA** / **OLGA** pixel pages) |
| **CLI** | `ohol-headless --bake-content` → all default index blobs + `manifest.json` |

Text `OneLifeData7` remains the **source of truth** for authoring; binaries are a **cache**.

**Not binary-cached (tiny text):** `contentSettings/emotionWords.ini` + `emotionObjects.ini` (L-EMOT / `EmotionBank`) — session/scene bind only.

---

## 1. Layout on disk

```
$OHOL_CONTENT_DIR/
  dataVersionNumber.txt          # upstream integer
  cache/
    manifest.json                # versions + sha1 of each blob
    olc1_objects.bin             # objects + multi-use dummy id lists
    olt1_transitions.bin         # transitions
    ols1_sprites.bin             # sprite meta (pos, size, anchor) [client+]
    ola1_anims.bin               # animation records [client+]
    olg1_ground_index.bin        # groundTileCache index [client+]
    olsn_sounds.bin              # sound index only (lazy AIFF) [client+]
    (optional) atlas/page_NN.rgba  # pre-packed GPU pages
```

Server may load only `olc1` + `olt1`. Client loads all.

---

## 2. Manifest (`cache/manifest.json`)

```json
{
  "format": 1,
  "data_version": 437,
  "created_utc": "unix:1720000000",
  "source": "OneLifeData7",
  "blobs": {
    "olc1_objects.bin": { "sha1": "...", "bytes": 12345, "count": 4400 },
    "olt1_transitions.bin": { "sha1": "...", "bytes": 6789, "count": 7500 },
    "ola1_anims.bin": { "sha1": "...", "bytes": 90000, "count": 3500 }
  }
}
```

Boot: if `data_version` matches tree and hashes match files → **fast path**. Else rebuild or fall back to text.

> v0 uses **sha1** (already a client dep). Can upgrade field to sha256 later without breaking magic.

---

## 3. Blob header (all `OL*` files)

Little-endian:

| Offset | Type | Field |
|--------|------|-------|
| 0 | `[u8;4]` | magic `OLC1` / `OLT1` / `OLS1` / `OLA1` / `OLG1` |
| 4 | `u32` | format_version (start at 1) |
| 8 | `u32` | data_version |
| 12 | `u32` | record_count |
| 16 | `u32` | flags |
| 20 | `u32` | header_crc32 (optional 0) |
| 24… | | payload |

Payload: dense length-prefixed records (section table optional later for mmap).

---

## 4. OLC1 — objects (format 6 write; v1–v6 load)

Per object record (dense, after 24-byte header):

```
i32 id
u16 name_len + utf8 name
u16 desc_len + utf8 description
u32 flags          # bit0 permanent, bit1 blocks_walking, bit2 containable, bit3 floor
                   # bit4 draw_behind_player
i32 food_value
i32 num_uses
f32 min_pickup_age
i32 person
f32 held_offset_x, held_offset_y
u8  clothing       # 'n','h','t','s','b','p'
f32 clothing_offset_x, clothing_offset_y
i32 num_slots
u16 n_slots; repeat n: f32 x, f32 y
u16 n_sprites; repeat n: ObjectSprite
u16 n_dummies; repeat n: i32 dummy_id
i32 dummy_parent   # 0 = not a dummy
f32 use_chance     # format ≥ 2 only (Haxe numUses=N,chance)
# --- format ≥ 3 trailer (sim + path-map radii) ---
i32 left_blocking_radius
i32 right_blocking_radius
f32 map_chance
f32 heat_value
f32 speed_mult
f32 r_value
f32 decay_factor
i32 decays_to_obj
f32 winter_decay_factor
f32 spring_regrow_factor
u16 n_biomes; repeat n: i32 biome_id
# --- format ≥ 4 trailer (object SoundUsage strings from sounds=CSV) ---
u16 creation_sound_len + utf8  # C++ creationSound
u16 using_sound_len + utf8     # C++ usingSound (also floor footstep sub)
u16 eating_sound_len + utf8    # C++ eatingSound
u16 decay_sound_len + utf8     # C++ decaySound
# --- format ≥ 5 trailer (multi-use sprite vis indices for setupSpriteUseVis) ---
u16 n_use_vanish; repeat n: i32 sprite_index   # C++ useVanishIndex sparse
u16 n_use_appear; repeat n: i32 sprite_index   # C++ useAppearIndex sparse
# Runtime only (not stored): ObjectSprite.skip_drawing from setup_sprite_use_vis
# --- format ≥ 6 trailer (variableDummyIDs) ---
u16 n_variable_dummies; repeat n: i32 dummy_id
```

**ObjectSprite:**

```
i32 sprite_id
f32 x, y, rot
u8  spr_flags      # bit0 h_flip, bit1 invis_holding, bit2 invis_worn, bit3 behind_slots
                   # bit4 behind_player
f32 age_start, age_end, r, g, b
i32 parent
```

Multi-use dummies: Haxe `ObjectBake` / server `assign_multi_use_dummies` — allocate free ids from `nextObjectNumber`, store `dummy_ids` on parent; `dummy_parent` map rebuilt on load. Runtime `materialize_dummy_object_records` clones parent sprites into dummy ids for soft-FB `get(dummy_id)` (skip masks not stored). OLC1 payload omits dummy records (parents only).

**Format defaults:** v1 → `use_chance=0`, no sim trailer; v2 → `use_chance` only, radii/`map_chance` default 0, `speed_mult=1`, `decay_factor=1`; v3 → full trailer. Client `load_prefer_cache` auto-rebakes when on-disk format &lt; write version. Server rebuilds `biome_spawn` from trailer on load; prefer-cache still text-falls-back if every `map_chance==0` (legacy cache).

API: shared crate **`ol-binary`** (`parse_olc1` / `encode_olc1` / DTO records, format 1..=6); client `write_olc1` / `load_olc1` / `peek_olc1_format` / dummies in `content_binary.rs`; server `ol_content::load_olc1` (maps sim subset; accepts v6).

---

## 5. OLT1 — transitions (format 2 write; v1+v2 load)

**Header flags** (blob offset 16, shared OL* layout):

| Bit | Name | Meaning |
|-----|------|---------|
| 0 | `OLT1_F_CATEGORY_EXPANDED` | Lite+pattern category transitions already concrete in payload (**P4#28**). Cache load skips `expand_category_transitions`; still loads `categories/*.txt` for `pick_from_prob_set` / `find_ptrans`. Absent → re-expand (correct, slower). |

Per transition:

```
i32 actor_id, target_id, new_actor_id, new_target_id
u8  flags
  # bit0 last_use_actor, bit1 last_use_target
  # bit2 reverse_use_actor, bit3 reverse_use_target  (format ≥ 2)
  # bit4 no_use_actor, bit5 no_use_target            (format ≥ 2)
  # bit6 max_use_target table row                    (P4#29)
  # bit7 switch_number_of_uses                       (P4#29)
f32 auto_decay_seconds
f32 actor_min_use_fraction   # format ≥ 2
f32 target_min_use_fraction  # format ≥ 2
i32 move_dist                # format ≥ 2
i32 desired_move_dist        # format ≥ 2
```

Dense list holds **normal**, **last-use**, and **max-use** records. On load: last-use bits → `transitions_last_use`; bit6 → `transitions_max_use`; else → `transitions` (so `A_B` / `A_B_LA` / max-use complete do not collide). Format 1 still loads (craft subset only). Text load uses Haxe `targetRemains` pair logic (`insert_normal_or_max_use`). `switch_number_of_uses` is also re-applied from ServerSettings key patches after load (legacy OLT1 without bit7).

API: shared crate **`ol-binary`** (`parse_olt1` / `encode_olt1` / flags); client `write_olt1` / `load_olt1` / `find_transition_max_use`; server `ol_content::load_olt1` + `load_from_cache` / `load_prefer_cache` (bit6/bit7 via shared path — **P4#30**).

---

## 6. OLS1 / OLA1 / OLG1 / OLO1 (client-heavy)

| Blob | Content | Status |
|------|---------|--------|
| OLS1 | sprite_id → width, height, center, multipane tag; optional file offset | **DONE** (format 2) |
| **OLSA** | optional multi-page sprite atlas (rects + RGBA pages) | **DONE** (format 1, **P4#40**) |
| OLA1 | object_id × anim_type × extra → param tracks | **DONE** (format 1) |
| OLG1 | groundTileCache + overlay (`ground_tN`) path index | **DONE** (format 1) |
| **OLGA** | optional multi-page ground atlas (rects + RGBA pages) | **DONE** (format 1, **P4#32**) |
| OLO1 | editor `overlays/{tag}/{id}.tga` path index | **DONE** (format 1) |
| **OLSN** | sound **index only** (id → path/rate/samples); **lazy AIFF** decode | **DONE** (format 1) — `sound_bank.rs` |

### OLSN — sound index (efficient load)

**Do not** decode all `sounds/*.aiff` at startup (hundreds of files). Implemented in `src/sound_bank.rs`.

```
cache/olsn_sounds.bin   # magic OLSN, format=1
  24-byte OL* header: magic, format_version, data_version, count, flags=0, crc=0
  repeat dense records:
    i32 id
    u32 sample_rate      # 0 until header peek / ensure
    u32 num_samples      # 0 if unknown until first open
    u16 path_len + utf8 relative path (e.g. sounds/123.aiff)
    u32 flags            # bit0 mono16_verified, bit1 is_ogg, bit2 header_peeked
```

- **Bake:** scan `$CONTENT/sounds/*.{aiff,ogg}` only (skip `soundsRaw`, `*.txt`); optional 54-byte AIFF header peek for rate/samples; write blob + `manifest.json` `olsn_sounds.bin` entry. No packed PCM.  
- **Boot:** `SoundBank::load_prefer_cache` → HashMap index only (`aiff_opens == 0`). Prefer-cache rebakes if missing/stale.  
- **First `SoundBank::ensure(id)`:** disk open + `read_mono16_aiff` (Haxe layout: mono @20–21, samples @22–25 BE, bits @26–27, rate @30–31, PCM @ byte 54 BE→i16). OGG → `None` in v1.  
- **SoundUsage:** `id:vol#id:vol` parse + `play_random` (`SoundUsage` / `play_usage`).  
- **Playback:** `play_id` / `play_usage`; with `--features audio` queues cpal mixer (soft-fail without device; `OHOL_AUDIO_DISABLE`); headless decode + `last_played` without the feature. 
- Sparse ids to ~1948; ~15KB index blob.

### OLS1 record layout (implemented in `sprite_bank.rs`)

Header as §3 with magic `OLS1`. Payload is a dense sequence of records (no section table yet).

**format_version = 2** (current write path):

```
repeat record_count:
  i32 id
  u32 width
  u32 height
  i32 center_anchor_x
  i32 center_anchor_y
  u32 flags          # bit0 = multiplicative_blend, bit1 = no_flip
  u16 tag_len
  u8[tag_len] tag    # utf-8
  i32 center_x_offset   # alpha-bbox center vs w/2 (C++ centerXOffset)
  i32 center_y_offset
  u32 visible_w         # maxX-minX of a>=64 region
  u32 visible_h
  u32 max_d             # max(w,h)
  u16 author_len
  u8[author_len] author # utf-8; empty if none
```

**format_version = 1** (legacy, still readable): same through `tag`; no bbox/author trailer.

Pixels remain on-demand from `sprites/{id}.tga` into runtime atlases (Haxe-style) by default.

API: `SpriteBank::write_ols1` / `load_ols1_meta` (v1+v2) / `load_prefer_cache`;
`bake_ols1_from_dir` / `bake_ols1_to_dir` / `scan_sprites_dir` (**P4#31** in `bake_content`).

### OLSA — optional full multi-page sprite atlas (**P4#40**)

Magic `OLSA`, **format_version = 1**. Optional full dump of packed rects + raw RGBA
atlas pages (mirror of ground **OLGA**). Default play path stays **OLS1 meta + lazy TGA**.

```
# 24-byte OL* header + 12-byte fixed meta:
u32 page_size        # atlas edge (usually 4096)
u16 num_pages
u16 reserved
u32 reserved2

repeat record_count:            # variable tag trailer
  i32 id
  u16 atlas_index
  i32 rect_x, rect_y, rect_w, rect_h
  u16 width, height             # source image size
  i32 center_anchor_x, center_anchor_y
  u32 flags                     # bit0 mult, bit1 no_flip
  i32 center_x_offset, center_y_offset
  u32 visible_w, visible_h, max_d
  u16 tag_len + utf8 tag

repeat num_pages:
  u32 width, height
  u32 byte_len                  # width*height*4
  [u8; byte_len]                # raw RGBA
```

- **Bake:** `bake_olsa_from_dir` / `bake_olsa_to_dir` → `cache/olsa_sprite_atlas.bin`;
  `pack_all_from_meta` (sorted ids) + `write_olsa`. CLI:
  `ohol-headless --bake-sprite-atlas [--src] [--out] [--version]` prints
  pack/write/total ms (`OlsaBakeStats`).
- **Load:** `SpriteBank::load_olsa` / `load_olsa_timed` (`OlsaLoadStats`); restored pages use
  `BinPack::sealed` so later packs open new pages (no pixel overwrite). HitMaps rebuilt
  from page alpha (not stored in v1).
- **Boot opt-in:** `load_prefer_atlas_cache` — if blob `data_version` matches tree
  `dataVersionNumber.txt`, restore atlas; else fall through to OLS1 / scan.
- **Not** written by default `bake_content` (large). Bench: step `sprite_atlas_load` when
  file present (else skipped note). See [LOAD_PERF.md](LOAD_PERF.md).

API: `write_olsa` / `load_olsa` / `load_olsa_timed` / `load_prefer_atlas_cache` /
`pack_all_from_meta` / `bake_olsa_*`.

### OLA1 record layout (implemented in `anim_bank.rs`)

Header as §3 with magic `OLA1`, **format_version = 2** (write); **v1 load still accepted**.

Source text: `animations/{id}_{type}.txt` and extras `animations/{id}_7x{i}.txt`
(C++ `AnimType`: 0 ground, 1 held, 2 moving, 4 eating, 5 doing, 7 extra).

```
repeat record_count:
  i32 object_id
  i32 anim_type          # C++ AnimType integer
  i32 extra_index        # -1 if not extra; else 0..n for type 7
  f32 rand_start_phase   # 0/1 from text randStartPhase
  u8  flags              # bit0 random_start, bit1 force_zero_start
  u16 n_sounds
  u16 n_sprites
  u16 n_slots
  repeat n_sounds:
    u16 usage_len + utf8 SoundUsage string
    f32 repeat_per_sec, repeat_phase, age_start, age_end
    u8  footstep
  repeat n_sprites: SpriteAnimParam   # 23 × f32 fixed
  repeat n_slots:   SpriteAnimParam
  # format 2 only:
  u16 author_len + utf8 authorTag     # empty if none (C++ author=)
```

**SpriteAnimParam** (23 little-endian f32, C++ `SpriteAnimationRecord` / Haxe `AnimationParameter`):

```
offset_x, offset_y, start_pause_sec
x_osc_per_sec, x_amp, x_phase
y_osc_per_sec, y_amp, y_phase
rot_center_x, rot_center_y, rot_per_sec, rot_phase
rock_osc_per_sec, rock_amp, rock_phase
duration_sec, pause_sec
fade_osc_per_sec, fade_hardness, fade_min, fade_max, fade_phase
```

API:

```rust
write_ola1 / load_ola1 / load_ola1_with_version
bake_ola1_from_dir(src, data_version)
AnimBank::write_ola1 / from_ola1 / load_prefer_cache / load_all_text
parse_animation_txt / parse_anim_filename
```

`bake_content` always writes `ola1_anims.bin` (empty payload OK if no `animations/`).  
`AnimBank::load_prefer_cache(root)` loads `root/cache/ola1_anims.bin` when valid; else text lazy-load.

**Cache validation (client):** when `cache/manifest.json` is present, `load_prefer_cache` checks:

1. OLA1 magic + `format_version == 1`
2. blob sha1 of `ola1_anims.bin` vs manifest entry
3. manifest `data_version` vs tree `dataVersionNumber.txt` (when both set)
4. OLA1 header `data_version` vs manifest (when both non-zero)

Mismatch → fall back to empty bank + text lazy-load (no auto-bake from this path).

**Sample fidelity (format-independent; C-ANIM DONE→stronger):** `SpriteAnimParam::frame_time` matches C++
`processFrameTimeWithPauses` (continuous wall time when pause+startPause are 0;
freeze at `(blocks+1)*duration` during pause). Fade uses C++/Haxe hardness
(`fadePhase+0.25`, power-square). Soft-FB multiplies blit alpha by `AnimSample.fade`
and applies `rotationCenterOffset` / `rot_center_*` pivot adjust. `sample_slot` for
containers; `ANIM_GROUND2` aliases to ground at sample time.

**OLA1 open residuals:**
- dual-anim / type-switch timeline fields are runtime (not binary)
- ~~authorTag~~ **DONE** (format 2)
- SoundAnimParam is stored; OLSN index + lazy AIFF + `play_usage`/`play_footstep` + `handle_anim_sound` period gate + footstep→floor **DONE** (L-SOUND-TRIG v2); real device / stereo pan residual
- ~~golden CI vs live OneLifeData7~~ **DONE** (P4#34 — `golden_anim_sample_vectors_*`)

Not folded into `ClientContent` (server does not need anims).

### OLG1 record layout (implemented in `ground_sprites.rs`)

Header as §3 with magic `OLG1`, **format_version = 1**, then **8-byte fixed meta**:

```
u16 num_overlays     # typically 4 (graphics/ground_t0..t3)
u16 tiles_w          # 4
u16 tiles_h          # 4
u16 max_biome        # highest biome id seen
```

Dense records (Haxe `SaveGroundData.bin` **index** replacement — path/presence only by default):

```
repeat record_count:
  u8  kind           # 0=overlay, 1=biome_tile, 2=unknown (99999)
  i32 id             # overlay N, biome id, or 99999
  u8  tile_x, tile_y # 0 for overlays
  u8  flags          # bit0=has_square, bit1=exists
  u16 width, height  # from TGA header at bake (0 if unknown)
  u16 path_len + utf8 rel_path
```

Runtime keys (Rust bank, not Haxe packed sequential map):

- biome: `biome*16 + x + y*4` (`ground_variation_index`)
- unknown: `99999 + x + y*4`
- overlay: separate map by overlay id 0..3

**Default play path:** pixels stay lazy TGA → runtime multi-page BinPack (2048²), matching
sprite policy. Haxe packs all tiles eagerly + writes `ground.png` / `SaveGroundData.bin`;
OLG1 is the presence/path index only so miss probes skip multi-root scans.

### OLGA — optional full multi-atlas dump (**P4#32**)

Magic `OLGA`, **format_version = 1**. Haxe `SaveGroundData` + `ground.png` analogue with
**multi-page** support (Haxe uses one `MAX_TEXTURE` page; C++ uses per-biome sheets —
see residual below).

```
# 24-byte OL* header + 12-byte fixed meta:
u32 page_size        # 2048
u16 num_pages
u16 num_overlays     # packed overlay count (informational)
u32 reserved

repeat record_count:            # 29 bytes each
  u8  kind                      # 0=overlay, 1=biome, 2=unknown
  i32 id
  u8  tile_x, tile_y
  u16 atlas_index
  i32 rect_x, rect_y, rect_w, rect_h
  u16 width, height             # source tile size

repeat num_pages:
  u32 width, height
  u32 byte_len                  # width*height*4
  [u8; byte_len]                # raw RGBA
```

API:

```rust
write_olg1 / load_olg1 / bake_olg1_from_roots / bake_olg1_to_dir
bake_olga_from_roots / bake_olga_to_dir          # optional full dump
GroundBank::load_prefer_cache                     # OLG1 default
GroundBank::load_prefer_atlas_cache               # OLGA then OLG1
GroundBank::pack_all_from_index / write_olga / load_olga
ensure_tile / ensure_overlay / ground_overlay_slot / scan_ground_index
```

`bake_content` always writes `olg1_ground_index.bin` (empty records OK if no game data);
**does not** write OLGA (large). Optional CLI: `ohol-headless --bake-ground-atlas`
→ `cache/olga_ground_atlas.bin`.

`GroundBank::load_prefer_cache(root)` loads `root/cache/olg1_ground_index.bin` when valid;
else scans disk into memory index. Overlay sheets preloaded via `preload_overlays`.

**Residual vs C++ / Haxe multi-atlas layout:**

| Aspect | Haxe | C++ | Rust OLGA / bank |
|--------|------|-----|------------------|
| Pages | Single `ground.png` (`MAX_TEXTURE`) | Per-biome `GroundSpriteSet` sheets | Multi-page 2048² BinPack (sprite-like) |
| Rect map | Haxe serializer `Map<Int,Rect>` sequential+special keys | Internal sheet UVs | Dense OLGA records + runtime bank keys |
| Unknown | Recolor white → several tinted packs | Unknown ground sheet | Base `biome_99999` only (no tint variants) |
| Default boot | Eager pack or load save | Load sets | **OLG1 + lazy TGA** (OLGA opt-in) |

Not folded into `ClientContent` (server does not need ground art).

### OLO1 record layout (implemented in `overlay_bank.rs`)

Header as §3 with magic `OLO1`, **format_version = 1**, then dense records (no fixed meta):

```
repeat record_count:
  i32 id
  u16 tag_len + utf8 tag          # folder name under overlays/
  u16 path_len + utf8 rel_path    # e.g. overlays/Dots/6.tga
  u32 width, height               # 0 until first TGA decode (optional)
```

C++ `overlayBank` loads every TGA + `loadSpriteBase` (and thumbnailSprite) at init;
Rust keeps a **lite** bank: id/tag/path index from disk or OLO1 at boot, then
`OverlayBank::ensure_image` lazy TGA. Not folded into `ClientContent`.

**Out of scope / remaining editor-only gaps (C-OVL DONE→v1):**

- `addOverlay` / `deleteOverlayFromBank` mutators
- thumbnailSprite + soft-FB multiplicative blit for import UI
- OverlayPickable / EditorImportPage integration

API:

```rust
write_olo1 / load_olo1 / bake_olo1_from_root / bake_olo1_to_dir
OverlayBank::load_prefer_cache / scan_from_root / get_overlay / ensure_image / search_overlays
scan_overlay_index
```

`bake_content` writes `olo1_overlays.bin` (empty body OK if no `overlays/`).
`OverlayBank::load_prefer_cache(root)` loads `root/cache/olo1_overlays.bin` when valid;
else scans `overlays/{tag}/{id}.tga`.

---

## 7. Baker CLI (implemented)

```text
ohol-headless --bake-content [--src path/to/OneLifeData7] [--out path/to/cache]
# defaults: OHOL_CONTENT_DIR or C:\OhOl\OpenLife\OneLifeData7 ; out = <src>/cache
```

Steps:

1. Read `dataVersionNumber.txt`  
2. Parse objects + transitions (text)  
3. Expand multi-use dummies (`assign_multi_use_dummies`)  
4. Write OLC1/OLT1  
5. Parse all `animations/*.txt` → write OLA1  
6. Scan groundTileCache + `graphics/ground_tN` (content + game-data roots) → write OLG1  
7. Scan `overlays/{tag}/{id}.tga` → write OLO1  
8. Write manifest with sha1  

**Client integration:** `ClientContent::load_prefer_cache(root)` — cache if valid, else auto-rebuild bake then text + dummies.  
Bake expands categories (via `load_from_dir`) and sets **`OLT1_F_CATEGORY_EXPANDED`** on OLT1.  
After OLC1/OLT1 load: if expanded flag set → **`maybe_load_category_bank_from_root`** only (no re-expand); else **`maybe_load_categories_from_root`** re-reads `categories/*.txt` and runs `expand_category_transitions` (lite + pattern; probSet left abstract for `find_ptrans`). Prefer-cache rebuilds once when OLT1 lacks the expanded flag (perf upgrade path).  
`AnimBank::load_prefer_cache(root)` for animations.  
`GroundBank::load_prefer_cache(root)` for OLG1 + lazy ground TGA.  
`OverlayBank::load_prefer_cache(root)` for OLO1 + lazy overlay TGA.  
**Server integration:** `ol_content::load_prefer_cache` / `load_from_cache` (OLC1/OLT1). When OLC1 lacks `map_chance` (client draw subset), server falls back to full text for world gen; direct `load_from_cache` remains for tools.

Library (client):

```rust
bake_content(src, out_dir) -> BakeResult  // OLC1+OLT1 + OLA1 + OLG1 + OLO1 + manifest
load_from_cache(cache_dir, expected_data_version) -> ClientContent
load_prefer_cache(root) -> ClientContent  // auto-rebuild on stale
write_olc1 / load_olc1 / write_olt1 / load_olt1
assign_multi_use_dummies / materialize_dummy_object_records
write_ola1 / load_ola1 / bake_ola1_from_dir
write_olg1 / load_olg1 / bake_olg1_from_roots
write_olo1 / load_olo1 / bake_olo1_from_root  // overlay_bank
```

Library (server `ol-content`):

```rust
load_from_cache(cache_dir, expected_data_version) -> ContentDb
load_prefer_cache(root) -> ContentDb
load_olc1 / load_olt1 / finish_cache_boot
```

---

## 8. Compatibility with C++ caches

We do **not** require reading Jason’s `folderCache` / `binFolderCache` file format.  
We **do** require the same *information* and similar *startup speed*.  
Optional later: importer from C++ cache for migration.

---

## 9. Status (H-BAKE / olc1_olt1_bake_v1)

| Item | Status |
|------|--------|
| Format design | **this doc** |
| OLS1 meta R/W (client) | **DONE** (`sprite_bank` format 2 + v1 load; **P4#31** written by `bake_content` → `ols1_sprites.bin`) |
| Runtime TGA + BinPack atlas | **DONE** |
| Alpha bbox + hitMap (client) | **DONE** (`compute_alpha_info` / `get_sprite_hit`) |
| Baker tool | **DONE** (`--bake-content` → OLC1 v6 + OLT1 v2 + OLA1 + OLG1 + OLO1 + OLSN + OLS1 + manifest) |
| OLC1 R/W + dummies | **DONE** (format **6** write / v1–v6 load; use vanish/appear + variableDummyIDs; materialize + **setupSpriteUseVis** at load) |
| OLT1 R/W | **DONE** (format 2 write / v1+v2 load; last-use + **max-use** maps; reverse/move/min-use; **P4#28** expanded; **P4#29** bit6/bit7; **P4#30** shared `ol-binary` client+server) |
| Shared OLC* crate | **DONE (P4#30)** — `ol-binary` zero-dep DTO loaders; client+server wired |
| OLA1 R/W + bake + load | **DONE** (format 1 unchanged; `anim_bank` + bake_content; sample/draw fidelity is runtime-only) |
| OLG1 R/W + bake + load | **DONE** (format 1; `ground_sprites` + bake_content; ground overlays + lazy TGA) |
| OLO1 R/W + bake + load | **DONE** (format 1; `overlay_bank` + bake_content; lazy TGA) |
| Client load from cache | **DONE** (`load_prefer_cache` auto-rebuild + format-upgrade rebake + `AnimBank` / `GroundBank` / `OverlayBank` / `SoundBank` / `SpriteBank::load_prefer_cache`) |
| Server load path | **DONE** (`ol-content` `binary_cache`; OLC1 v3 sim + biome_spawn; prefer-cache sticks when map_chance present) |
| Headless without content | OK (wire-only probes; synthetic atlas tests) |

### Open gaps (post v1)

| Gap | Notes |
|-----|-------|
| Shared client/server crate | OLC1/OLT1 loaders **duplicated** (client `content_binary.rs` vs server `binary_cache.rs`) |
| Dummy sprite-use masks | **DONE (P4#25)** — `setup_sprite_use_vis` + OLC1 v5 vanish/appear; residual golden vis CI |
| `variableDummyIDs` | **DONE P4#26** — `assign_variable_dummies` / OLC1 v6 lists; residual: `setupNumericSprites`, `+varSerialNumber` |
| Category-expanded transitions | **Lite+pattern DONE** + **bake expanded OLT1 P4#28** (`OLT1_F_CATEGORY_EXPANDED`). Bake includes concrete member/pattern rows; cache load skips re-expand when flag set (bank-only for pick). Legacy OLT1 still re-expands from `categories/*.txt`. Residual: reverse-category play consumers |
| OLT1 use tables | **DONE (P4#29)** — max-use rows (flag bit6) + `switch_number_of_uses` (bit7); patches after load |
| OLS1 in baker | **DONE P4#31** — `ols1_sprites.bin` via `bake_ols1_from_dir`; residual: bake-time alpha-bbox without full TGA decode |
| Full sprite atlas dump | **DONE (P4#40)** optional OLSA multi-page dump; OLS1+lazy TGA remains default; residual: full-tree size; optional SHA1 beyond dataVersion |
| Full ground atlas dump | **DONE (P4#32)** optional OLGA multi-page dump; OLG1 remains default; residual: Haxe unknown recolors / C++ per-biome sheets / not PNG+serializer |
| Editor overlay UI (C-OVL residual) | OLO1/lite bank **DONE→v1**; out of scope: `addOverlay`/`deleteOverlayFromBank`, thumbnailSprite mult-blit, OverlayPickable/EditorImportPage |
| Golden dummy-id CI | No live OneLifeData7 golden vs server dummy id sequence |
| Global string table | Names/descs still inline per record (no shared string table in OLC1/OLT1) |
