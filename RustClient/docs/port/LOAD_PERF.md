# Client load performance

**Hub:** [README.md](README.md) · **formats:** [CONTENT_BINARY.md](CONTENT_BINARY.md)

**Goal:** Measure and keep headless + graphics startup fast via binary cache (OLC1…OLS1; optional OLSA/OLGA).

## CLI

```powershell
cd C:\OhOl\OpenLife\RustClient
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH

# Full bench (text + binary + graphics asset sample)
cargo run --release --bin ohol-headless -- --bench-load --also-text --report logs/load-bench.md

# Ensure cache exists first
cargo run --release --bin ohol-headless -- --bench-load --ensure-bake --also-text
```

Env: `OHOL_CONTENT_DIR` or `--src PATH` (default: RustServer `OneLifeData7` / tree `OneLifeData7`).

## Baseline (2026-07-26, release, OneLifeData7 v437)

| Mode | Total | Notes |
|------|-------|-------|
| **headless_text** | **~1846 ms** | parse all objects+transitions txt |
| **headless_prefer_cache** | **~150–300 ms** | OLC1 **v3**+OLT1+OLA1 (~6–12× faster than text; sim trailer slightly larger than v2) |
| **graphics_prefer_cache** | **~378 ms** | content ~57 ms + anim ~43 ms + **sprite preload ~278 ms** |

Binary cache: `objects=9330` (incl. multi-use dummies), `transitions=4858`, `anim_records=3517`.

### Hotspots

1. **Text load** — only for bake/dev; production must hit cache.  
2. **Sprite TGA preload** — largest graphics cost; keep lazy + small preload sets.  
3. **Double content load** in bench (binary_load + prefer_cache) — prefer_cache alone is ~50–90 ms.

## Workflows

| Workflow | Role |
|----------|------|
| `rust-client-continue-headless` | Bench → next headless chunk → optimize → re-bench |
| `rust-client-continue-graphics` | Bench → next graphics chunk → optimize → re-bench |

## Policy

- No new Cargo crates for load path.  
- Objects/transitions: **OLC1/OLT1** only (same as server).  
- Anims: **OLA1**; sprites: OLS1 meta + on-demand TGA/atlas.  
- Sounds: **OLSN** index only at boot; **lazy mono-16 AIFF** on `SoundBank::ensure` (never full-decode all AIFFs).  
  Device open (`--features audio` / cpal) is **not** part of boot — first `play_id` only. `aiff_opens` stays 0 until ensure/play.
- **P5#36 progress:** optional `OHOL_LOAD_PROGRESS=1` logs stage/fraction during prefer_cache (session + `--bench-load`). Graphics: `draw_loading_progress` + `boot_load_prefer_cache` in ohol-client.

## Atlas bake / load measurements

Report always includes wall-clock **bake** and **load** times so we can compare disk pages vs runtime pack.

| Path | How to measure | Step / field names |
|------|----------------|--------------------|
| Default content bake (indexes) | `ohol-headless --bake-content` | prints `BakeTimings` per blob; `ols1` = **meta only** |
| Ground pixel pages (OLGA, P4#32) | `--bake-ground-atlas` | `pack_ms` / `write_ms` / `total_ms` on `OlgaBakeStats` |
| Ground pixel load | `--bench-load` when `cache/olga_ground_atlas.bin` exists | step `ground_atlas_load` (+ `OlgaLoadStats`) |
| Sprite meta load | `--bench-load` | `sprite_ols1_meta_load` |
| Sprite runtime pack (TGA+BinPack) | `--bench-load` | `sprite_runtime_atlas_pack` (alias `sprite_preload_atlas`) |
| Sprite pixel pages (OLSA, **P4#40 DONE**) | `--bake-sprite-atlas` | `OlsaBakeStats` pack/write/total ms |
| Sprite pixel load | `--bench-load` when `cache/olsa_sprite_atlas.bin` exists | step `sprite_atlas_load` (`OlsaLoadStats`) |

**Compare:** with OLSA present, `sprite_atlas_load` should beat `sprite_runtime_atlas_pack` for large id sets; invalidate on `dataVersion` mismatch.
