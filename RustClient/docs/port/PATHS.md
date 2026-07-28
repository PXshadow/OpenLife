# Canonical paths

Update this file if trees move.  
Client hub: [README.md](README.md).

## Upstream C++ (Jason Rohrer OneLife)

```
C:\Users\marti\source\repos\ledgerweb\third_party\OneLife\
  gameSource\          ← client code (LivingLifePage.cpp ~25k lines)
  server\protocol.txt  ← wire protocol bible
  server\              ← official server (reference only)
  documentation\
```

**Engine:** C++ uses `minorGems` (graphics, game loop, UI). Rust does **not** link minorGems; reimplement only what we need.

## Open Life (Haxe + content)

```
C:\OhOl\OpenLife\
  openlife\client\           Haxe client
  openlife\resources\        ObjectBake, Resource paths
  openlife\data\             ObjectData, transitions, map
  OneLifeData7\              full content tree
  OneLifeGameSourceData\     graphics, groundTileCache, languages, settings
  RustClient\                THIS client workspace
  openlife\RustServer\       game server + content load reference
```

## Env vars (client)

| Var | Use |
|-----|-----|
| `OHOL_HOST` / `OHOL_PORT` | server |
| `OHOL_EMAIL` / `OHOL_PASSWORD` / `OHOL_ACCOUNT_KEY` | login |
| `OHOL_CONTENT_DIR` | path to OneLifeData7 or baked `content.bin` dir |
| `OHOL_HEADLESS=1` | force no window |
| `OHOL_WIRE_LOG` | default wire log path |
| `OHOL_DEBUG=1` | prefill `settings.debug` (F9 / SNAP play snapshots) |
| `OHOL_SHOW_FPS` | prefill show FPS in title |

**Settings file:** `ohol_client_settings.ini` (cwd) — created on first run; volumes, mutes, show_fps, **debug**.

Credentials: `RustClient/.env` (gitignored). See `.env.example`.
