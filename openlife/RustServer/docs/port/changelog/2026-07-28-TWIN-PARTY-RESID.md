# TWIN-PARTY-RESID / twin_wait_edges (2026-07-28)

## Scope
Same-server twin residual edges for Open Life Rust port. **No multi-server** peer sockets/handoff (parked stub stays).

## Haxe / product
- `Connection.hx` login: `// TODO twins` — protocol `LOGIN … twin_code_hash twin_count`
- OHOL twins plan #10: murder of a party member → remaining get broken-heart wound and die soon after
- Haxe `ObjectData.male` + `GlobalPlayerInstance.isFemale`

## Implemented
1. **`TwinHeartLinks`** (`twin_heart.rs`) — register party p_ids on birth; on murder death return siblings
2. **`apply_twin_heart_link_on_murder`** (`twin_party_live.inc.rs`) — max wound stacks + food cut + `TWINHEART` PS
3. **Combat wire** — KILL/HIT kill paths call heart-link when `is_murder_death_reason`; else `remove_player`
4. **Natural death** — vitals death clears `twin_heart` without sibling wound
5. **`ObjectDef.male`** — parse `male=` from object files; `player_is_female` prefers content male when person race set
6. **Wait timeout** — tick `poll_twin_timeouts` + `TWIN_WAIT_TIMEOUT_SECS` (300s) + `TWINWAIT FAIL timeout` PS
7. **PS polish** — ready/wait/heart/timeout; `format_twin_wait_ps_code` on join

## Out of scope
- Multi-server `TwinRegistry` sockets / LOGIN handoff / cross-server wait queue (**parked**)

## Apply
```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
# build.rs piggybacks via build_do_commands → build_twin_party_resid → python apply
cargo test -p ol-sim --lib -- twin_heart
cargo test -p ol-content --lib -- parse_male_flag
```

Or: `python crates/ol-sim/src/_fix_and_apply_twin.py`
