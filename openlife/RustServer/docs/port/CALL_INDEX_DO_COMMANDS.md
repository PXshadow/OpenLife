
## DO-COMMANDS / say_commands (2026-07-26)

| Symbol | File | Role |
|--------|------|------|
| `GlobalPlayerInstance.doCommands` | `server/GlobalPlayerInstance.hx` | Natural-language SAY: I EXILE/FOLLOW/HIRE/GIVE, ORDER,, OWN THIS, HOME! |
| `processFollowCommand` / `processHireCommand` / `redeem` | same | Follow self/name; hire AI for coins; clear exile edges |
| `getLeaderWhoExiled` / `isFollowerFrom` | same | Exile gates on follow/hire; redeem chain depth |
| `NamingHelper.GetName` / `GetPlayerByName` | `server/NamingHelper.hx` | Third token + closest name |
| Rust `parse_do_command` / `parse_roman_coin_amount` / `compute_hire_cost` / `find_player_by_name` | `ol-sim/speech.rs` | Pure DO-COMMANDS |
| Rust `apply_do_commands_live` | `ol-sim/do_commands_wire.rs` | Live SAY wire + hire prestige regain |
| Rust `SocialState::redeem` / `leader_who_exiled` / `hired_by` / `set_hired` | `ol-sim/social.rs` | Clear exile edges + exile gate + hire map |
| Wire | `apply_say_or_remv` after EXILE | live natural-language forms |
| Tests | `speech::*` / `social::redeem_*` / `leader_who_exiled_*` / `say_do_commands_*` | pure + live |

Residual: multi-owner `addOwner`; AiBase MAKE/CRAFT hear path; HOME! firePlace side-effect. Delayed follow → **LEADERSHIP-UX** DONE.
