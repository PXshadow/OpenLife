# Grok usage tools (local secrets via `.env`)

Fetch SuperGrok / Grok Build usage **without putting API keys in git**.

## Setup (once)

```powershell
cd C:\OhOl\OpenLife\openlife\Tools\usage-budget
copy .env.example .env
# Option A — after "grok login", export key into local .env (never commit):
.\export-auth-to-env.ps1

# Option B — edit .env by hand:
#   GROK_API_KEY=...
#   GROK_USER_ID=...
```

`.env` is listed in **`.gitignore`** and must not be committed, emailed, or pasted into chats.

## Commands

| Script | Purpose |
|--------|---------|
| `.\fetch-usage.ps1` | Live fetch + save `data/usage-snapshot.json` (**usage % only**, no keys) |
| `.\read-usage.ps1` | Status vs soft ceiling / 80% hard cap |
| `.\check-budget.ps1` | Exit non-zero if over budget (for automation) |
| `.\export-auth-to-env.ps1` | Create `.env` from `%USERPROFILE%\.grok\auth.json` |

## Auth resolution order

1. **`./.env`** → loads `GROK_API_KEY` + `GROK_USER_ID` into process env only  
2. Process environment (if already set)  
3. Optional: `%USERPROFILE%\.grok\auth.json` **only if** `GROK_ALLOW_AUTH_JSON=1`  

No secrets are written into snapshots or printed in full (keys are masked when exporting).

## What is safe to share

| Path | OK to commit? |
|------|----------------|
| Scripts, README, `.env.example` | Yes |
| `.env` | **No** |
| `data/usage-snapshot.json` | Prefer no (personal usage %; gitignored) |
| `auth.json` | **Never** put under this tree |

## Parent ignore

`C:\OhOl\OpenLife\.gitignore` also ignores  
`openlife/Tools/usage-budget/.env` and snapshot files.
