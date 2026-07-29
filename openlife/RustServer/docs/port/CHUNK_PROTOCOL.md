# Chunk protocol — how to port without losing details

Mandatory process for humans and AI agents (including `haxe-port-chunk` workflow).

---

## 1. Pick a chunk

Sources of truth (in order):

1. `TODO_PORT.md` → “Recommended next 10 chunks”  
2. `FILE_MATRIX.md` → chunk ids (`GPI-FOOD`, `TIME-WORLD`, …)  
3. Explicit args: `haxe_file` + `chunk` or `matrix_id`

**Size rule:** one function family or ~50–200 lines of Haxe logic.  
Never start with all of `GlobalPlayerInstance.hx` or `AiBase.hx`.

---

## 2. Audit (read-only)

Produce a short **audit note** (in scratch or commit message):

| Field | Content |
|-------|---------|
| Haxe range | file + function names + approx lines |
| Callers | who calls this (from CALL_INDEX / grep) |
| Callees | what it calls |
| Wire | tags in/out if any |
| State | which fields of player/world mutate |
| Haxe TODOs | list with port decision |
| Rust today | modules + status DONE/PARTIAL/MISSING |
| Gaps | bullet list of missing behaviors |
| Tests | existing tests that touch this |

**Do not implement before gaps are listed.**

---

## 3. Design (keep simple)

- Prefer **pure functions** + thin wiring in `apply_intent` / tick.  
- Match Haxe **outcomes**, not Haxe structure.  
- If Haxe is buggy and product relies on bug: **port-as-is** + comment.  
- If Haxe TODO is clearly desired: implement + test.  
- Name modules/functions so an AI can grep them later.  
- Add `// Haxe: TimeHelper.DoSeason` style anchors on non-obvious ports.

---

## 4. Implement

1. Write/adjust code in the Rust module from FILE_MATRIX.  
2. Add/extend unit tests in the same crate.  
3. Run:

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- <filter>
# if net/server touched:
cargo test -p ol-server
```

4. No secrets in tree; no `.env` commits.

---

## 5. Document (same change)

Update:

- [ ] `FILE_MATRIX.md` row status  
- [ ] `TODO_PORT.md` checkboxes  
- [ ] `CALL_INDEX.md` if new public fns  
- [ ] `PROGRESS.md` one-liner if user-visible  

Optional: short entry in `docs/port/changelog/` (if created later).

---

## 6. Verify

Minimum:

- [ ] Unit tests green for the chunk  
- [ ] No accidental protocol break (PU/MX fields)  
- [ ] Intentional deltas listed if behavior differs  

Stronger (when relevant):

- [ ] Self-play 8–12s agents still move  
- [ ] Manual client check for movement/USE  

---

## 7. Workflow modes

| Mode | Agents do |
|------|-----------|
| `audit` | Steps 1–2 only; write gap report; no code |
| `implement` | Steps 1–5; code + tests + docs |
| `verify` | Step 6 only on existing work |

---

## 8. Definition of DONE for a chunk

A chunk is **DONE** only if:

1. Every Haxe behavior in scope is implemented **or** explicitly deferred in TODO_PORT with reason.  
2. Tests cover pure rules; integration covers wire if applicable.  
3. Matrix + TODO updated.  
4. Code is clean: no dead comments of secrets, no giant unexplained blocks.

**PARTIAL** is OK and expected for large files — but partial must list remaining gaps.

---

## 9. Anti-patterns

- Porting entire AiBase in one PR  
- “Looks done” pure modules never wired to `apply_intent`  
- Dropping edge cases silently  
- Duplicating Haxe mutex patterns in Rust  
- Editing only PROGRESS without matrix/TODO  
- Committing usage `.env` or auth.json  

---

## 10. Template: audit note

```markdown
## Chunk: TIME-ANIMAL / TIME-ANIMAL-CHASE
### Haxe
- TimeHelper.doAnimalMovement, DoAnimalDamage, MakeAnimalsRunAway, GetClosestBoneGrave
### Calls
- callers: DoWorldMapTimeStuff, doTimeTransition
- callees: WorldMap.*, GetClosestPlayerAt, isSpawningIn, CalculateNonBlockedTarget
### Wire
- MX on animal tile change; possible PU damage
### Rust today
- animals.rs + animal_move.rs + animal_damage.rs — wander + damage_escape + **chase_biome DONE**
### Gaps
1. offspring / die-in-place / failedMoves>20
2. ~~cursedGraves global index~~ → **CURSED-GRAVES-INDEX DONE**; ~~!TCG/!TV consumers~~ → **CURSED-GRAVE-TELEPORT DONE**
### Haxe TODOs
- L2268 fleeing / L2269 offspring / L2270 meat — deferred
### Plan
- pure chase helpers; wire tick_animals_dt
### Tests
- animal_move::resolve_*; goto_target_*; is_spawning_in_*
```
