# 2026-09-09 NAME-FULL-LINEAGE

**status:** **DONE**

Haxe `Connection.sendToMePlayerInfo` NAME + `Lineage.createLineageString` LN.

- NM: `p_id first getFullName(true, true)` = Eve `familyName` + optional AI `X` + prestige class, spaces to `_` (e.g. `7 ADA SNOW_Commoner`)
- LN bootstrap / birth: mother-id chain (`createLineageString`); truncated chains append ` eve_id=`
- `LineageNode.wire_line` kept (not on the NAME/LN client tags)

Tests: `lineage_get_full_name_name_packet_shape` / `create_lineage_string_appends_eve_id_when_truncated` / `social_bootstrap_ln_is_create_lineage_string_nm_full_name` / `send_to_me_all_close_players_nm_full_lineage_name`

`cargo check -p ol-server --offline` Finished.

Residual: ol-net login_bootstrap still `NEWBORN FAMILY`; AiNameEnding not LiveSettings (`X`). Next **LOCATION-SAYS-MARKERS**.
