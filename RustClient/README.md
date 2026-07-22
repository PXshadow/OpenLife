# ohol-headless (RustClient)

Headless (no graphics) One Hour One Life / OpenLife client for playtesting a compatible server.

Wire protocol follows Jason Rohrer’s original client and `protocol.txt` (see parent OpenLife tree and/or upstream OneLife `server/protocol.txt`).

## Features

- `#`-framed TCP messages
- Login: read `SN`, compute `HMAC_SHA1(password, challenge)` and `HMAC_SHA1(pure_account_key, challenge)`, send `LOGIN` / `RLOGIN`
- Movement matching original wire rules: `MOVE xs ys @seq …#`, sequence starts at **2**, deltas within **±16**, client-side in-motion / `FORCE` ack / block USE·DROP·REMV while moving
- Object commands: `USE`, `DROP`, `REMV`, `SELF`, plus `KA` keepalive

## Build

```bash
cd C:\OhOl\OpenLife\RustClient
cargo build --release
```

## Self-check (fixture peer, no real game server)

```bash
cargo run -- --self-check
```

## Connect to local Rust server (OpenLifeReborn)

Defaults match stock OHOL / OpenLifeReborn: **`127.0.0.1:8005`**.

### Local credentials (gitignored)

```bash
copy .env.example .env
# edit .env — set OHOL_EMAIL / OHOL_ACCOUNT_KEY / OHOL_PASSWORD
```

`.env` is listed in `.gitignore`. Never commit real keys.

Vars: `OHOL_HOST`, `OHOL_PORT`, `OHOL_EMAIL`, `OHOL_PASSWORD`, `OHOL_ACCOUNT_KEY`  
(CLI flags override env.)

```bash
# Probe SN + LOGIN (uses .env if present)
cargo run

# After ACCEPTED: move one step east and keep-alive
cargo run -- --move 1,0 --ka

# One-off override without editing .env
cargo run -- --email you@example.com --account-key AB-CD-EF-GH --move 1,0
```

Server config reference: `C:\OhOl\OpenLifeReborn\server.toml` (`game_port = 8005`).  
If logins are rejected, check ticket verify / account key validity.

Optional: `--reconnect` (RLOGIN), `--tutorial N`, `--drop x,y`, `--remv x,y`, `--self x,y`, `--swap x,y`, `--use-id`, `--no-email-pad`, `--host` / `--port` overrides.

### Object interactions (official wire)

Matches LivingLifePage / `protocol.txt`:

| Command | Wire |
|---------|------|
| USE | `USE x y#` / `USE x y id#` / `USE x y id i#` |
| DROP | `DROP x y c#` (`c=-1` ground) |
| REMV | `REMV x y i#` |
| SELF | `SELF x y i#` |
| SREMV / SWAP / BABY / UBABY | also supported in lib |

Actions are **queued** while mid-MOVE (same as official `nextActionMessageToSend`) and flushed when the move finishes.

```bash
cargo run -- --probe-actions --log logs/wire-actions.log
cargo run -- --use 0,0 --use-id 33 --drop 1,0 --self 0,0
```

## Library

Pure encoders live in the `ohol_headless` crate (`encode_move`, `encode_login`, `encode_use`, …) and are unit-tested; the binary uses the same functions.

## Legal

Unofficial tooling. One Hour One Life is by Jason Rohrer. This crate reimplements only the documented network protocol for automated playtesting.
