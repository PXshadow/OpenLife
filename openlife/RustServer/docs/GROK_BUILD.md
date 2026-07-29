# Grok Build instructions — Open Life Reborn

Instructions for working on this repo with **Grok Build**. Project rules in
`AGENTS.md` always apply. This file is the detailed session playbook.

---

## 1. Budget (read first)

### Slash command: `/usage`

**Subscription / weekly SuperGrok pool usage is tracked with the Grok Build slash command:**

```
/usage
```

| Command / tool | What it tells you |
|----------------|-------------------|
| **`/usage`** (or `/cost`) | Weekly pool usage & reset — **budget source of truth** |
| `.\scripts\record-usage.ps1` | Save that % into a local snapshot for automation |
| `.\scripts\read-usage.ps1` | Soft-ceiling math + status |
| `.\scripts\check-budget.ps1` | Exit non-zero if over soft ceiling (loop gate) |
| `/context` | Context window fill for **this session** (not SuperGrok quota) |
| `/session-info` | Model, turns, session stats |
| `/compact` | Compress session history to save context |

Full automation notes + X community practices: `docs/USAGE_AUTOMATION.md`.

### Rules

1. Run **`/usage`** at the **start** of any non-trivial session.
2. Run **`/usage` again** before spawning subagents, large refactors, or multi-crate work.
3. Stay under a **hard planning cap of 80%** of the allowance until refresh.
4. **Pace** spend: cumulative soft ceiling = `80% × (day_index / days_until_refresh)`.
5. Log rough estimates in `docs/USAGE_BUDGET.md` after each working day.
6. If `/usage` shows you are at or above today's soft ceiling → **do not implement**;
   only document, or ask the user.

### Example (10 days to refresh)

- Day 1 soft max cumulative: **8%**
- Day 2: **16%**
- …
- Day 10: **80%** (never plan to use the last 20%)

Under-spend may roll modestly to the next day, but never blow through the cumulative soft ceiling without user approval.

---

## 2. Session contract (paste / follow every time)

```markdown
## Goal this session
<one concrete bullet>

## Budget
- Ran /usage at session start: <yes>
- Soft ceiling today: <e.g. day 2/10 → 16% cumulative>
- This session tier: T0 | T1 | T2 | T3 | T4
- Stop if over soft ceiling

## In scope paths
- crates/...
- docs/...

## Out of scope
- Full AiBase port
- SQL / databases
- Client rewrite
- Unrelated refactors

## Architecture
- Sim sole writer of world/player state
- Net/AI enqueue Intents only
- No blocking I/O on sim thread
- No DB

## Done when
- cargo test passes (relevant packages)
- Short note in docs/USAGE_BUDGET.md if work used Grok
```

---

## 3. Session tiers (cost)

| Tier | Name | When | Rough cost intent |
|------|------|------|-------------------|
| **T0** | Local only | `cargo test` / edits you do yourself | **0%** subscription |
| **T1** | Micro | One file, one API, tests | Small |
| **T2** | Feature | One vertical slice in one crate | Medium |
| **T3** | Review | Review a diff only | Small |
| **T4** | Spike | Unknown design; write research doc | At most one daily slice |

Prefer **T0 + T1**. Avoid T4 unless research is blocked.

---

## 4. Daily workflow

```
1. /usage                          → check subscription budget
2. Pick ONE task from phase list   → docs/architecture plan
3. Implement (T1/T2)               → stay in scoped paths
4. cargo test && cargo check       → local, free
5. Update docs/USAGE_BUDGET.md     → estimate
6. Stop                            → do not "just one more" if near ceiling
```

---

## 5. Phase checklist (high level)

See full design: `docs/architecture/RUST_SERVER_REBUILD_PLAN.md`

| Phase | Focus |
|-------|--------|
| **A** | Skeleton (current): protocol, metrics, binary boots |
| **B** | Content load + chunked empty world |
| **C** | Core verbs (MOVE/USE/DROP) + intents |
| **D** | Open Life rules (season, YUM, combat, …) |
| **E** | Server AI (modular, parallel) |
| **F** | Web + lineage character pages |
| **G** | Scale (cold chunks, load tests) |

Do not skip to E/F until C intent architecture is solid.

---

## 6. Legacy reference

- Haxe server: `C:\OhOl\OpenLife` (do **not** vendor into this git repo)
- Product rules bible: `C:\OhOl\OpenLife\TODO.MD`
- Extract facts into `docs/research/*` instead of re-ingesting huge sources

---

## 7. Quality bar

- `#![forbid(unsafe_code)]` in library crates unless there is a documented exception
- Unit tests for protocol and pure logic
- No secrets in git
- English docs/comments
- Small commits with clear messages when the user asks to commit

---

## 8. What “clean from day one” means

- Separate git history from the Haxe project
- Minimal compiling workspace
- Strict `.gitignore` (no saves, no huge content trees, no targets)
- Explicit AGENTS + budget policy
- Architecture plan in-repo
- No half-copied server dump

When in doubt: **smaller change, measured by `/usage`, tested with cargo**.
