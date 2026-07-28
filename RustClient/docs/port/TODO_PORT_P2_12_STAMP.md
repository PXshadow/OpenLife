# TODO stamp — P2#12 mMapGlobalOffset

**Apply to [TODO_PORT.md](TODO_PORT.md):**

### P2 table row 12

```text
| 12 | ~~**`mMapGlobalOffset` sendX/sendY**~~ | L-ACT | **DONE-NA** — wire frame == storage; `MapGlobalOffset::ZERO` + `encode_move_with_offset`; see MAP_GLOBAL_OFFSET.md |
```

### P2 checklist

```text
- [x] MOVE `sendX`/`sendY` + `mMapGlobalOffset` (**DONE-NA** identity offset=0; MAP_GLOBAL_OFFSET.md)
```

### Deferred / non-goals (add)

```text
| Non-zero mMapGlobalOffset local maps | DONE-NA — client stores wire coords; no float-tile precision path |
```

### Next chunks

```text
Start at **P2 path fidelity** (multi-MOVE ultimate-goal repath) unless playtest feedback says otherwise.
```

### Changelog (prepend)

```text
| 2026-07-27 | **P2#12 mMapGlobalOffset sendX/sendY** (**DONE-NA**): C++ GPU local-map offset audit; Rust storage==wire (`session.map` / `move_state` / `encode_move`); identity API `MapGlobalOffset::ZERO` + `encode_move_with_offset`; doc `MAP_GLOBAL_OFFSET.md`; not birth-relative (server frame separate) |
```
