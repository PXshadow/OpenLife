# Chunk protocol — C++/Haxe → Rust client

**Hub:** [README.md](README.md) · **what to pick:** [TODO_PORT.md](TODO_PORT.md)

Same spirit as the server port kit: **no silent detail loss**, clean Rust structure.

---

## 1. Pick a chunk

From `TODO_PORT.md` queue or `FILE_MATRIX` id (`L-NET-PARSE`, `C-OBJ`, …).

**Size:** one subsystem or ~50–200 logical C++ units (handlers, not 25k lines).

---

## 2. Audit (read-only)

| Field | Content |
|-------|---------|
| C++ range | file + functions / message tags |
| Haxe ref | if assets/tags/render related |
| Wire | protocol.txt section |
| State | LiveObject / map / banks touched |
| Rust today | modules + status |
| Gaps | concrete missing behaviors |
| Headless test | how to prove without GPU |

---

## 3. Design

- Prefer **pure apply/parse** + thin session glue  
- Headless must gain the feature when possible  
- Content: text first OK; plan binary cache fields  
- `// C++: LivingLifePage.cpp PU` anchors  
- Steal Haxe packing/bake only when better  

---

## 4. Implement

```powershell
cd C:\OhOl\OpenLife\RustClient
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test
cargo run -- --self-check
# optional live:
# cargo run -- --probe-play --log logs/chunk.log
```

Never commit `.env`.

---

## 5. Document

- [ ] FILE_MATRIX row  
- [ ] TODO_PORT checkboxes  
- [ ] CALL_INDEX if new public API  
- [ ] README current-state blurb if user-visible  

---

## 6. Modes

| Mode | Action |
|------|--------|
| `audit` | gaps only |
| `implement` | code + tests + docs |
| `verify` | tests only / fix regressions |

---

## 7. Definition of DONE

1. Behaviors in scope implemented or **explicitly deferred** with reason  
2. Unit tests and/or headless probe  
3. Docs updated  
4. Code remains simple and modular  

---

## 8. Parallelism

OK when modules differ (e.g. content bake vs tag parse).  
Avoid two writers on `session.rs` / `main.rs` without coordination.
