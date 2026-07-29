# Reading Grok usage & budgeting agent loops

## Short answer

| Method | Weekly SuperGrok pool % | Good for loops? |
|--------|-------------------------|-----------------|
| **`.\scripts\usage-budget\fetch-usage.ps1`** | **Yes** — same API as `/usage` | **Best for automation** |
| **Grok Build `/usage`** (or `/cost`) | Yes (TUI) | Manual |
| **Local snapshot** (auto-saved by fetch) | Yes | Gate with `check-budget.ps1` |
| **`POST grok.com/rest/rate-limits`** | Usually **no** | Ignore for SuperGrok pool |
| **xAI API console credits** | Separate billing path | Different system |

### How `/usage` works (open-source Grok Build)

```
/usage
  -> Effect::FetchBilling
  -> ACP extension "x.ai/billing"
  -> GET https://cli-chat-proxy.grok.com/v1/billing?format=credits
     Authorization: Bearer <token from ~/.grok/auth.json>
     X-XAI-Token-Auth: xai-grok-cli
     x-userid: <user_id>
```

Response: `config.creditUsagePercent`, `config.currentPeriod`, optional `config.productUsage[]`.  
Our `scripts/usage-budget/fetch-usage.ps1` calls that same HTTP path with local auth.  
Full manual: `scripts/usage-budget/MANUAL.md` · build loop: `scripts/usage-budget/GROK_BUILD_LOOP.md`.

---

## Official facts (xAI docs + product)

Shared **weekly SuperGrok usage pool** across Chat, Build, Imagine, Voice, etc.

- Check: **Settings → Usage** on web/app, or **`/usage`** in Grok Build  
- Shown as **% used**, product breakdown, **reset time**, extra credits  
- Exhausted pool → paid features pause until reset (or buy credits / upgrade)  
- **API team spending limits** in console.x.ai are a **separate** system from the weekly SuperGrok bar  

Docs: [Grok FAQ — Usage & Limits](https://docs.x.ai/grok/faq)

---

## Scripts in this repo

```text
scripts/usage-budget/
  MANUAL.md                     # human manual
  GROK_BUILD_LOOP.md            # how to build Rust under budget
  fetch-usage.ps1               # LIVE fetch (same as /usage API)
  read-usage.ps1                # live fetch + soft ceiling + status
  check-budget.ps1              # exit 2 if over soft ceiling
  record-usage.ps1              # optional manual paste fallback
  data/
    config.json                 # cap 80%; period dates synced from API
    usage-snapshot.json         # last fetch (gitignored)
```

### Workflow (recommended)

```powershell
cd C:\OhOl\OpenLifeReborn

# Live pull (requires prior: grok login)
.\scripts\usage-budget\fetch-usage.ps1

# Status + soft ceiling:
.\scripts\usage-budget\read-usage.ps1

# Gate before heavy agent loops:
.\scripts\usage-budget\check-budget.ps1
# exit 0 = OK, 2 = STOP, 3 = no data, 1 = error
```

### Soft ceiling (same as AGENTS.md)

```
soft_ceiling = 80% × (day_index / period_days)
```

Example weekly (`period_days = 7`):

| Day | Soft max cumulative |
|-----|---------------------|
| 1 | ~11.4% |
| 2 | ~22.9% |
| … | … |
| 7 | 80% |

(If you prefer 10-day mental model, set `"period_days": 10` and set `reset_at_utc` from `/usage`.)

### Optional web probe

```powershell
# Cookie from DevTools → Network → request headers → Cookie (logged-in grok.com)
$env:GROK_COOKIE = "..."
.\scripts\read-usage.ps1 -ProbeWeb
```

Or paste `scripts/browser-console-usage-snippet.js` into the grok.com console.

**Do not commit cookies.** `GROK_COOKIE` is env-only.

### Exit codes (`read-usage` / `check-budget`)

| Code | Meaning |
|------|---------|
| 0 | Under soft ceiling |
| 2 | Over soft ceiling or ≥ hard 80% cap |
| 3 | No snapshot yet |
| 1 | Script error |

---

## Why not “only web”?

1. **Weekly pool UI/API is not a stable public CLI.** Browser extensions that scraped Grok usage have been **archived** after API churn.  
2. Legacy `POST https://grok.com/rest/rate-limits` returns **rolling query/token windows** — useful historically, **not** the SuperGrok weekly % bar.  
3. **Build OAuth vs API keys:** headless `grok -p` JSON can report **token/cost** for API-key traffic; SuperGrok **pool %** is still the Settings/`/usage` system.  
4. Scraping requires **session cookies** (security risk if stored in-repo).

So: **web is a sensor you can try; `/usage` + local snapshot is the control loop.**

---

## How people on X constrain Grok Build loops (field notes)

Synthesized from public posts / Grok replies (mid‑2026):

### Reality of the pool

- SuperGrok is a **shared weekly pool** (Build + chat + Imagine + …).  
- **Build burns hard** — long agent sessions / tool loops can dominate the bar (users report large Build %).  
- People hit limits mid‑project; some **switch tools** (e.g. Claude → Grok or reverse) when one pool is empty.  
- **Prompt caching regressions** have burned tokens faster (xAI has fixed and issued credits before).  
- Separately, **console spending limits / API credits** can 403 even when the weekly % still has room.

### Practices that actually save budget

1. **Pace intentionally** — don’t dump a full week into day 1 (your 80% soft ceiling policy).  
2. **Check `/usage` before long loops** — Grok itself points people at `/usage` (or `/cost`).  
3. **One goal per session** — avoid multi-agent fan-out unless headroom is clear.  
4. **Shrink context** — `/compact`, research notes instead of re-reading huge files; some use **output compressors** (e.g. rtk-style filters on `git diff` / test logs) to cut tokens 60–90% on command-heavy sessions.  
5. **Local compute first** — `cargo test`, linters, targeted fixes without the model.  
6. **Local token ledger + official weekly %** — community “usage widgets” (e.g. Baby Menu-related) combine **estimated local burn** with the **official pool** rather than trusting only one.  
7. **Avoid Imagine/video** during coding weeks — same pool, much more expensive compute.  
8. **Treat top-ups as emergency**, not plan — several users argue upgrades beat repeated credits if you always exhaust.  
9. **Stop condition in the agent contract** — AGENTS.md “if over soft ceiling, document and stop” (this repo).  
10. **Wish-list API** — users (including project maintainers) have asked xAI for **script-readable `/usage`** so automated loops can self-throttle; not shipped as a stable public interface yet.

### Anti-patterns (burn the week)

- Unbounded “keep going until done” overnight loops  
- Parallel heavy implementer subagents without a gate  
- Re-ingesting multi-thousand-line files every turn  
- Mixing film/image gen into the same week as a big Build project  

---

## Ideal future (if xAI adds it)

```text
grok usage --json
→ { "used_percent": 14.2, "reset_at": "...", "by_product": { "build": ... } }
```

Until then, this repo’s **record → read → check** loop is the clean automation surface.

---

## Related files

- `AGENTS.md` — mandatory budget rules  
- `docs/GROK_BUILD.md` — session contract  
- `docs/USAGE_BUDGET.md` — human log  
- `docs/architecture/RUST_SERVER_REBUILD_PLAN.md` §11  
