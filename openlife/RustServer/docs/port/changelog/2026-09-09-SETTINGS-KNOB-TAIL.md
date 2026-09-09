# 2026-09-09 SETTINGS-KNOB-TAIL

**status:** **DONE**

Haxe `ServerSettings` leftover knobs → `LiveSettings` + `field_map` + one live reader each.

Landed (Haxe default, Live):

- `DisplayScoreOn` (true) — age-58 extra prestige GMs
- `DisplayScoreFactor` (already Live; still the GM multiplier)
- `MaxCoinsPerChest` (200) / `MaxCoinsPerPouch` (50) — chest/pouch store cap
- `ChanceForFemaleChild` (0.6) — spawnAsChild roll; Eve founder `>= 0.5`
- `ChanceForOtherChildColor` (0.2) / `ChanceForOtherChildColorIfCloseToWrongSpecialBiome` (0.3)
- `LittleKidsPerMother` (3) — mother fitness hard-reject
- `NewChildExhaustionForMother` (0)
- `AiMotherBirthMaliForHumanChild` (3) / `HumanMotherBirthMaliForAiChild` (1)
- `SpwanAtLastDead` (false, Haxe typo) — Eve origin last-death `spawn_x/y`
- `TemperatureOwnTileRate` (0.05) / `TemperatureBalanceRate` (0.9) / `TemperatureLocalHeatFactor` (0.005)
- `AverageSeasonTemperatureImpact` (0.2) — scales stored season impact vs Haxe 0.2

Skipped (no invented readers): mutex/debug/secret/file-path; Wool/RabbitFur patch constants; unused ModuleConst long tail listed in `field_map.rs` TODO.

Tests: `age58_display_score_off_header_only` / `score_age::display_score_off_sends_header_only` / `chest_coins::store_caps_at_max_chest` live cap / `little_kids_per_mother_hard_rejects` / `average_season_temperature_impact_scales_identity_at_haxe_default` / tile lerp rates.

`cargo check -p ol-server` Finished.

Next **IDLE-COMPLETE**.
