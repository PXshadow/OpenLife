# SETTINGS-LONG-TAIL / ScoreFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (ScoreFactor promoted; long-tail continues)
- **Haxe:** `ServerSettings.ScoreFactor = 0.2`; `PlayerAccount.ChangeScore` on death
- **Rust:** `LiveSettings.score_factor` → `GameplayKnobs` → `blend_score_ema` / death `change_score`

## Implemented this fire
1. `score_factor` on ServerConfig / LiveSettings / GameplayKnobs (default 0.2)
2. FIELD_MAP `ScoreFactor` ModuleConst → Live
3. `blend_score_ema` (2-decimal round) + `AccountBook::change_score` (sex-split + family prestige, founder ×2)
4. Death path calls ChangeScore with lineage prestige (else session scoreboard) and live factor

## Residual
- Dynasty `familyPrestige[myDynastyId]` not folded
- `AiTotalScoreFactor` still ModuleConst (totalScore getter)
- Next: `OldGraveDecayMali` / `AncestorPrestigeFactor`

## Verify
```powershell
cargo test -p ol-config --lib -- ScoreFactor score_factor field_map
cargo test -p ol-sim --lib -- change_score_ema_matches_haxe apply_live_settings_gameplay_knobs
```
