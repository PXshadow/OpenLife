# Playable done bar

**Repo:** `C:\OhOl\OpenLife\RustClient`  
**Snapshot:** 2026-07-28  

This file answers only: **can you live a life on a compatible server with this client?**  
**P1–P5 priority list:** **COMPLETE** — see **[TODO_PORT.md](TODO_PORT.md)** for residuals only.

---

## Met

| Lane | What’s in |
|------|-----------|
| **Headless** | `#` frames, LOGIN/RLOGIN/HMAC, FM batching, FORCE/KA, LiveObject + map, pathfind, multi-MOVE, USE/DROP/REMV/SELF/clothing/containers, probes + `--self-check` |
| **Click → play** | `click_tile` path-to-adjacent, hold-slide, biomes/rideable/waypoint, soft-FB hover hitMap + clothing/contained slots |
| **Content** | Prefer-cache OLC1 v6 / OLT1 v2 / OLA1 / OLS1 / OLG1 / OLO1 / OLSN; shared `ol-binary`; category expand + max-use; optional OLSA/OLGA pixel atlases |
| **Present** | Soft-FB SceneRenderer, dual-fade anim packs, PE/emotes, speech, HUD food/heat, wall layers, rideable draw order |
| **Audio** | Lazy OLSN + triggers + optional cpal pan; music OGG (`lewton`) under `--features audio` |
| **Product UI** | Loading progress, Account, Death/rebirth, Settings (`ClientScreen` soft-FB) |
| **GUI shell** | `ohol-client` (default `gpu`+`audio`) |

**Deps (policy):** hmac, sha1, hex, thiserror, anyhow, dotenvy, flate2, path `ol-binary`; optional minifb / cpal; lewton for music beds only.

---

## Outside the bar (optional)

| Item | Doc |
|------|-----|
| In-row residuals (reverb, music step, chrome, …) | notes on DONE rows in TODO |
| readyPending mid-move PU hold | TODO deferred |
| Photo / full editor / wgpu | non-goals |

---

## Run

```powershell
cd C:\OhOl\OpenLife\RustClient
cargo test --lib
cargo run --bin ohol-headless -- --self-check
cargo run --release --bin ohol-client
```

Overview + doc map: [README.md](README.md).  
Architecture: [ARCHITECTURE_RUST_CLIENT.md](ARCHITECTURE_RUST_CLIENT.md).
