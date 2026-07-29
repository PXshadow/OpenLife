# OHOL connect path: Haxe vs Rust

Audit of whether the Rust server matches the Haxe Open Life server on **port** and **messages the client needs on connect**.

## Port

| | Haxe | Rust |
|--|------|------|
| Game TCP | **8005** (`ThreadServer`) | **8005** (`server.toml` `game_port`) |
| Web | 8080 | 8080 |

**Port is correct** for a stock OHOL client custom-server entry.

---

## On TCP accept (before LOGIN)

### Haxe (`Connection.new`)

```haxe
send(SERVER_INFO, ["0/0", challengeString, '$version']);
// SERVER_INFO == "SN"
// version == ObjectData.dataVersionNumber  (e.g. 437 from dataVersionNumber.txt)
// message: "SN\n0/0\n{challenge}\n{version}\n#"
```

### Rust (after fix)

```
SN
{players}/{max_players}
{48-char challenge A-Z0-9}
{dataVersion from content, else 437}
#
```

| Check | Status |
|-------|--------|
| Tag `SN` | OK (Haxe alias `SERVER_INFO`) |
| Multiline + `#` | OK |
| Challenge length 48 alnum | OK |
| Version = content `dataVersionNumber` | OK (now prefers content, not stale 87) |
| Players field | Haxe hardcodes `0/0`; Rust uses live count — usually fine |

---

## After LOGIN / RLOGIN

### Haxe (`initConnection`) — **required for a real client**

1. `ACCEPTED\n#`
2. **MAP_CHUNK** 32×30 zlib of `biome:floor:obj` cells around player
3. **TOOL_SLOTS** `TS` / `0 1000`
4. **PU** all close players (+ **NM** names)
5. Lineages / following / exile lists (can be empty)
6. **FX** food update
7. **FM** frame
8. **BB** bad biomes

### Rust (after fix)

Sends a **minimal Haxe-shaped bootstrap**:

1. `ACCEPTED\n#` (was wrong as `ACCEPTED#` — fixed)
2. **MC** 32×30 zlib empty-ish map from bootstrap world
3. **TS** `0 1000`
4. **PU** self (person object 19, spawn 0,0)
5. **NM** name line
6. **FX** food
7. **FM** frame
8. **BB** bad biomes

| Check | Status |
|-------|--------|
| ACCEPTED wire format | Fixed to match Haxe |
| Map chunk after login | Present — now from **shared sim world** (`Arc&lt;RwLock&lt;World&gt;&gt;`) |
| PU / FX / FRAME | Present (minimal); **MX+PU+FX after USE** via OutboundHub |
| Full lineage / multi-player PU | **Not yet** (open question) |
| Live map = sim world | **Yes for MC at login + MX after USE** |
| Account ticket verify | **Not yet** (Haxe hits ticket server) |

---

## Honest answer

- **Port:** yes, **8005**.
- **SN on connect:** yes, same tag and shape as Haxe; version now follows content (**437**).
- **Everything the client needs after login:** **not fully** before this fix; **much closer after** (ACCEPTED+MC+PU+FX+FM+BB). Still not a full playable handoff: map is empty bootstrap, no ticket auth, no continuous MX/PU stream from sim, no mother/leader packets.

A vanilla client may **connect and pass SN/LOGIN**, and **no longer hang on missing ACCEPTED/MC**, but will still look broken for real play until sim world is streamed and auth is implemented.

---

## Code references

| Side | Location |
|------|----------|
| Haxe SN | `openlife/server/Connection.hx` `new` → `SERVER_INFO` |
| Haxe send format | `Connection.send` → `'$tag\n${data.join("\n")}\n#'` |
| Haxe post-login | `Connection.initConnection` |
| Rust SN | `ol_protocol::format_sn` |
| Rust post-login | `ol_net::login_bootstrap::build_login_bootstrap` |
