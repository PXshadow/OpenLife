# Open Life Reborn — Grok Build project rules

This repository is a **clean Rust rewrite** of the Open Life Reborn game server
(and its embedded website). It is **not** the Haxe `OpenLife` tree.

Legacy reference (read-only, separate folder): `C:\OhOl\OpenLife`  
Legacy design notes: `C:\OhOl\OpenLife\TODO.MD` and `docs/architecture/` here.

---

## Budget (mandatory)

Super Grok / subscription usage is **limited**. Stay under a hard planning cap of
**80% of the allowance** until the next refresh, **paced evenly over remaining days**.

### Always check real usage with slash command + local gate

Before starting substantial work in a session, and again before long or parallel work:

```
/usage
```

Use `/usage` as the **source of truth** for SuperGrok weekly pool usage. Do not
guess remaining budget. If usage is already near the soft ceiling for today, **stop
implementing**, write a short status note, and wait for the next day slice or user OK.

**Fetch and gate (scripts — preferred for loops):**

```powershell
# Canonical toolkit (see scripts/usage-budget/MANUAL.md):
.\scripts\usage-budget\fetch-usage.ps1
.\scripts\usage-budget\read-usage.ps1
.\scripts\usage-budget\check-budget.ps1
```

Budget-aware build loop: `scripts/usage-budget/GROK_BUILD_LOOP.md`  
Endpoint (same as `/usage`): `GET https://cli-chat-proxy.grok.com/v1/billing?format=credits`

Also useful:

- `/context` — session context window (not the same as subscription budget)
- `/compact` — shrink conversation when context is fat

### Pacing formula

```
Cap            = 0.80 × full allowance (never plan past 80%)
Days left      = days until usage refresh (ask user if unknown)
Daily slice    = Cap / Days left
Soft ceiling(t)= Cap × (day_index / total_days)
```

Example for a 10-day refresh window:

| Day | Cumulative soft max |
|-----|---------------------|
| 1   | 8%                  |
| 2   | 16%                 |
| 3   | 24%                 |
| …   | …                   |
| 10  | 80%                 |

Record estimates after sessions in `docs/USAGE_BUDGET.md`. Prefer under-spend early.

### Cost control rules

1. **One goal per session** — one crate, one feature, or one research doc.
2. **No drive-by refactors** outside the stated goal.
3. **Prefer local tools** (`cargo test`, `cargo check`, `rg`) over re-reading huge
   legacy files with the model.
4. **Do not spawn multiple heavy implementer subagents** unless `/usage` shows
   clear headroom under today's soft ceiling.
5. **Do not paste** multi-thousand-line Haxe files into context; summarize into
   `docs/research/` instead.
6. **No image/video generation** for this project unless the user explicitly asks.
7. If blocked or over budget: write findings to `docs/` and stop.

Full session contract: `docs/GROK_BUILD.md`.

---

## Architecture constraints (non-negotiable)

1. **Simulation is the sole writer** of world tiles and live player components.
2. **Network and AI only enqueue Intents** — never mutate world/player fields directly.
3. **No traditional database** (no SQL/Redis required at runtime). Use files:
   snapshots, WAL, mmap chunk packs, append-only lineage indexes.
4. **No blocking disk I/O on the sim thread.**
5. **Web serves snapshots / indexes**, never holds sim locks to build HTML.
6. **Metrics are first-class** (`ol-metrics`, `/health` later).
7. **Scope is server + web**, not a full client rewrite.
8. Keep crates small; prefer pure functions + unit tests.

Target design: `docs/architecture/RUST_SERVER_REBUILD_PLAN.md`.

---

## Clean repo rules

- Do not copy the entire Haxe tree into this repo.
- Do not commit `SaveFiles/`, large `OneLifeData7/` trees, or binaries (see `.gitignore`).
- Do not commit secrets or `.env` files.
- Keep the default branch buildable: `cargo test` should pass on the skeleton.
- Prefer English for code comments and public docs.

---

## Build & test

```bash
cargo test
cargo check
cargo run -p ol-server
```

Install Rust via https://rustup.rs if `cargo` is missing.

---

## Session workflow

1. User (or agent) runs **`/usage`** and checks soft ceiling for the day.
2. Read `docs/GROK_BUILD.md` and the phase checklist if needed.
3. Implement **one** scoped task.
4. Run `cargo test` (and `cargo check`).
5. Update `docs/USAGE_BUDGET.md` with estimated spend note.
6. Stop if near budget or when the single goal is done.

---

## Out of scope unless explicitly requested

- Porting all of `AiBase.hx` in one go
- SQL / ORM
- Client graphics engine
- Force-push / rewriting published history
- Spending past the 80% cap or today's soft ceiling
