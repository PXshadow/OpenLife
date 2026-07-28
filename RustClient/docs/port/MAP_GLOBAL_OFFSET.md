# `mMapGlobalOffset` / `sendX` / `sendY` — audit + DONE-NA

**Hub:** [README.md](README.md) · **TODO:** [TODO_PORT.md](TODO_PORT.md) P2#12

**Date:** 2026-07-27  
**Status:** **DONE-NA** (not applicable — identity offset=0 API)  
**Code:** [`src/map_global_offset.rs`](../../src/map_global_offset.rs)

---

## 1. C++ audit (LivingLifePage)

| Symbol | Role |
|--------|------|
| `mMapGlobalOffset` / `mMapGlobalOffsetSet` | Optional origin subtracted from **received** coords so local storage stays near 0 for GPU float tiles |
| `applyReceiveOffset(*x,*y)` | `*x -= offset.x` (when set) on every inbound map/player coord |
| `sendX` / `sendY` | `return inX + offset.x` before **any** client→server line (MOVE start, USE, DROP, …) |

### First-MC policy (`LivingLifePage.cpp` ~16127–16153)

On the first `MC` header center `(x,y)`:

- If both axes in **±16384** → `offset = (0,0)` (typical towns / birth-relative ~0)
- Else → `offset = (x,y)` so local cells stay small

### MOVE send path (`LivingLifePage.cpp` ~26510–26518)

```text
MOVE sendX(pathToDest[0].x) sendY(pathToDest[0].y) @seq …
  then relative steps path[i] - path[0]   // same local frame; deltas unchanged by offset
```

Path **deltas** are never offset-converted (they are relative to start in the storage frame, which equals wire deltas when start was converted by sendX/Y).

### Why C++ needs this

32-bit float tile rendering loses precision for huge absolute world integers. **Protocol / server always want the same frame the server last put on the wire** (protocol.txt: “absolute world position”; Haxe/OpenLifeReborn may use **birth-relative** world = client + birth — that is a *server* frame choice, not `mMapGlobalOffset`).

---

## 2. Rust client today

| Layer | Coordinate frame |
|-------|------------------|
| `session.map` (`ClientMap`) | Wire keys from MC origin + MX `(x,y)` — **no** receive subtract |
| `session.world` / PU/PM | Wire positions as parsed |
| `move_state.x/y` | Bound from our PU; advanced in **same** frame |
| `encode_move(xs,ys,@seq,…)` | Emits `xs,ys` unchanged |
| Pathfind / click_tile | Operates on map + move_state tiles (wire frame) |

**End-to-end:** storage frame **==** wire frame. Therefore `sendX`/`sendY` must be the **identity**.

This holds for:

1. **Absolute world** servers (classic OHOL when spawn within ±16384 → C++ also uses offset 0; probe historically `MOVE 488 488 @2 …`)
2. **Birth-relative** OpenLifeReborn bootstrap (`PU`/`MC` near 0,0; server `client_to_world` / `world_to_client`) — client still echoes the frame it was given

No second local map origin is introduced, so implementing non-zero `mMapGlobalOffset` would be **wrong** unless every receive path also applied `apply_receive`.

---

## 3. offset=0 API (shipped)

```rust
use ohol_headless::{MapGlobalOffset, encode_move_with_offset, PathDelta};

let o = MapGlobalOffset::ZERO; // set=true, x=0, y=0
assert!(o.is_identity());
assert_eq!(o.send_x(488), 488);
assert_eq!(o.send_y(0), 0);

// Same wire as encode_move when offset is zero:
let line = encode_move_with_offset(o, 488, 488, 2, &[PathDelta { x: 1, y: 0 }])?;
// "MOVE 488 488 @2 1 0#"
```

`MapGlobalOffset::from_first_mc_center(cx, cy)` documents the C++ ±16384 threshold but **always returns ZERO** under the Rust DONE-NA policy (i32 wire storage, no float-tile path).

---

## 4. Verdict

| Question | Answer |
|----------|--------|
| Does MOVE need non-zero sendX/sendY? | **No** — client already matches server wire frame |
| Is full C++ local-map offset required for play? | **No** (DONE-NA) |
| Residual risk | Only if a future renderer stores float-local tiles **and** subtracts a non-zero offset without reversing on send |

**Mark TODO P2#12 / L-ACT mMapGlobalOffset: DONE-NA.**

---

## 5. Not this ticket

- Server **birth-relative** vs absolute world (see OpenLifeReborn `docs/BIRTH_RELATIVE_COORDS.md`) — separate from GPU `mMapGlobalOffset`
- Multi-MOVE ultimate-goal repath (**P2#11 DONE**)
- useWaypoint polish
