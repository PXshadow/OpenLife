# Headless client mode

**Binary:** `ohol-headless` (`C:\OhOl\OpenLife\RustClient`)  
**Purpose:** playtest servers, CI, protocol regression — **no GPU required**.  
**Overview:** [README.md](README.md) · **status:** [TODO_PORT.md](TODO_PORT.md)

---

## 1. Why headless is first-class

- Server port can be validated without a human clicking  
- Wire logs are reproducible  
- Future `LiveWorld` logic is shared with the GPU client  

GUI is a **feature**, not the core.

---

## 2. Quick commands

```powershell
cd C:\OhOl\OpenLife\RustClient
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH

cargo test
cargo run -- --self-check

# bake shared OLC1/OLT1 cache (CONTENT_BINARY)
cargo run -- --bake-content --src C:\OhOl\OpenLife\OneLifeData7
# → writes <src>/cache/olc1_objects.bin, olt1_transitions.bin, manifest.json
# optional full atlas dumps (large; not in bake_content):
# cargo run -- --bake-sprite-atlas --src ...   # OLSA → cache/olsa_sprite_atlas.bin
# cargo run -- --bake-ground-atlas --src ...   # OLGA → cache/olga_ground_atlas.bin
cargo test --lib content_binary
cargo test --lib sprite_bank::tests::olsa

# against local Rust server :8005
copy .env.example .env   # once; fill keys
cargo run -- --probe-move --log logs/wire-move.log
cargo run -- --probe-play --log logs/wire-play.log
cargo run -- --probe-test --log logs/wire-probe-test.log

# Play-point snapshot (AI / regression)
cargo run -- --snapshot-self-check
# live (needs server + .env):
# cargo run -- --snapshot logs/snapshots/after_login.txt --snapshot-label login --timeout 15
```

Env: `OHOL_HOST`, `OHOL_PORT`, `OHOL_EMAIL`, `OHOL_PASSWORD`, `OHOL_ACCOUNT_KEY`  
(see `.env.example`; **gitignored** `.env`).

Content (optional): `OHOL_CONTENT_DIR` → OneLifeData7 for object/sprite text+TGA load; prefer `cache/` OLC1/OLT1 when present.

Sprite/atlas tests (no GPU):

```powershell
cargo test --lib binpack
cargo test --lib sprite_bank
cargo test --lib tga
cargo test --lib render
cargo test --lib content_binary
cargo test --lib session force_  # FORCE / flush / KA / logout_reset
cargo test --lib pathfind
cargo test --lib click_tile
```

Synthetic RGBA packing works without a content tree. Real TGA path is skipped if `sprites/144.tga` is absent.

---

## 3. Library API (stable intent)

```rust
// connect_and_login / connect_and_login_logged
// encode_move, encode_use, encode_drop, ...
// click_tile / plan_click_tile / ClickTileExt::click_tile_to_move (ground → cumulative MOVE)
// click_drop_clothing(slot) / click_self / click_sremv_clothing / resolve_clothing_equip_slot
// clothing_char_to_slot / encode_drop c 0..5 / send_sremv
// apply_click_gates / encode_jump / NO_MOVE_AGE — playerActionPending, 0-speed, baby JUMP
// find_path / PathFindResult (pathFindingD=32, closest fallback)
// FrameReader, parse_sn, parse_pu_line, parse_inbound, ServerTag, ...
// MoveState — client-side motion rules (+ send_move_repath)
// SessionEvent::{PlayerUpdate { pu, force_ack_sent, .. }, MapChanges, FoodChange, ...}
// FrameReader inflates CM → inner body before parse_inbound
// session.food / session.heat last FX/HX
// FORCE: cancel pending action + send FORCE mid-dispatch; done_moving flushes queue
// trunc PM cancels pending + dest_truncated; artificial FORCE on dest mismatch
// baby-held (held_id < 0 for ourID) cancels pending
// maybe_send_ka / KA_IDLE_SECS (15) / logout_reset
// SpriteBank::ensure / ensure_rgba / write_ols1 / load_ols1_meta
// BinPack::pack, load_tga_path, Framebuffer::blit_sprite
// SceneRenderer::draw (CPU RGBA)
// bake_content / load_prefer_cache / write_olc1 / write_olt1 (CONTENT_BINARY)
```

Expand with:

- `LiveWorld::apply(NetEvent)` / LiveObject map from full `PlayerUpdate`  
- MC binary decode + map store  
- `SceneRenderer::draw` → RGBA framebuffer (no window); `screen_to_world` for scripted clicks  
- scripted scenarios (`Scenario::pickup_and_eat`)  
- replay from wire log

---

## 4. CI sketch

```text
1. cargo test -p ohol-headless
2. start ol-server (fixture world) OR mock peer
3. ohol-headless --self-check
4. ohol-headless --probe-move --timeout 15
5. fail if REJECTED or missing PM/PU
```

---

## 5. Roadmap flags

| Flag / mode | Status |
|-------------|--------|
| `--self-check` | DONE |
| `--probe-*` | DONE (several) |
| `--bake-content` | **DONE** (OLC1/OLT1 + manifest) |
| `--scenario FILE` | MISSING |
| `--replay LOG` | MISSING |
| `--content DIR` | PARTIAL (`OHOL_CONTENT_DIR` / `--src` for bake) |
| `--headless` default GUI off | DONE (binary is headless-only today) |
