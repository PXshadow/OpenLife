# Protocol matrix (stub)

Source of truth for wire messages. Expand during Phase A/B.

Legacy: `C:\OhOl\OpenLife\protocol.txt`, `ServerTag.hx`, `ClientTag.hx`.

## Framing

- ASCII messages
- Terminated by `#`
- First token = tag

## Client → server (incomplete)

| Tag | Purpose | Status |
|-----|---------|--------|
| `LOGIN` | New life login | TODO |
| `RLOGIN` | Reconnect | TODO |
| `KA` | Keepalive | TODO |
| `USE` | Use / transition | TODO |
| `DROP` | Drop held | TODO |
| `MOVE` / path moves | Movement | TODO |
| `SAY` | Speech | TODO |
| `KILL` | Attack | TODO |
| … | … | … |

## Server → client (incomplete)

| Tag | Purpose | Status |
|-----|---------|--------|
| `SN` | Sequence / challenge | TODO |
| `PU` | Player update | TODO |
| `MX` | Map change | TODO |
| `FX` | Food / effects | TODO |
| … | … | … |

## Implementation

Rust: `crates/ol-protocol` — framing only for now.
