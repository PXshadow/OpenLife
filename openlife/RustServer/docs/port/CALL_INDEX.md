# Call index — AI lookup

Quick map of **important functions** and where they live.  
Not exhaustive; expand when porting a chunk.

Paths relative to `C:\OhOl\OpenLife\openlife\` unless noted.

---

## Haxe: boot & loop

| Symbol | File | Calls / called by |
|--------|------|-------------------|
| `Server.main` | `server/Server.hx` | → settings, ObjectData, TransitionImporter, WorldMap, WebServer, `TimeHelper.DoTimeLoop` |
| `TimeHelper.DoTimeLoop` | `server/TimeHelper.hx` | → `AiBase.StartAiThread`, loop `DoTimeStuff` |
| `TimeHelper.DoTimeStuff` | same | → season, players, world, respawn, long-term, save |
| `TimeHelper.DoTimeStuffForPlayer` | same | → stats, leadership, objects, emotes, age, food |
| `TimeHelper.DoWorldMapTimeStuff` | same | → temp, transitions, water, animals |
| `TimeHelper.DoWorldLongTermTimeStuff` | same | → decay, snow, spring, walls |
| `TimeHelper.DecayFloor` / `DecayObject` | same | long-term tile decay |
| `TimeHelper.AlignWalls` / `AlignWall` | same | wall auto-orientation |
| `TimeHelper.DoRespawnFromOriginal` / `DoSpringStuff` | same | spring regrow / bear cave |
| `TimeHelper.SpreadSnow` / `RemoveSnow` / `IsProtected` | same | seasonal biome snow |
| `TimeHelper.doTimeTransition` | same | → `doTimeTransitionHelper`, animal movement |
| `TimeHelper.doTimeForObject` | same | contained ObjectHelper timers |
| `TimeHelper.doAnimalMovement` | same | → path, damage, escape |
| `TimeHelper.DoAnimalDamage` / `DoAnimalDamageHelper` | same | path cells + player hurt |
| `TimeHelper.TryAnimaEscape` | same | attack → flee chance |
| `TimeHelper.MakeAnimalsRunAway` | same | player step → speed timers |
| `TemperatureHandler.balanceTileTemperature` | `server/TemperatureHandler.hx` | neighbor diffusion + local heat |
| `TemperatureHandler.BalanceTemperatureArea` | same | Chebyshev rings around player (`updateTemperature`) |
| `TemperatureHandler.UpdateTileTemperature` | same | map-slice own-tile lerp + balance |
| `GlobalPlayerInstance.updateTemperature` | `server/GlobalPlayerInstance.hx` | BalanceTemperatureArea + body heat + HX |
| `TransitionHelper.TransformTarget` | `server/TransitionHelper.hx` | probSet weighted random outcomes |
| `AiBase.StartAiThread` | `auto/AiBase.hx` | AI thread entry |

---

## Haxe: network

| Symbol | File | Role |
|--------|------|------|
| `Connection.login` / `loginHelper` | `server/Connection.hx` | account + spawn |
| `Connection.rlogin` / `rloginHelper` | same | reconnect |
| `Connection.close` | same | disconnect / AI takeover |
| `Connection.send` / `sendHelper` | same | ClientTag frames |
| `Connection.sendMapChunk` | same | MC |
| `Connection.sendPlayerUpdate` / `AndFrame` | same | PU + FM |
| `Connection.sendMapUpdate` | same | MX |
| `Connection.sendSayToAllClose` | same | PS (distance-only; no mute graph) |
| `Connection.keepAlive` | same | KA |
| `Connection.die` | same | DIE |
| `ThreadServer` accept | `server/ThreadServer.hx` | sockets |

---

## Haxe: actions

| Symbol | File | Role |
|--------|------|------|
| `TransitionHelper.doCommand` | `server/TransitionHelper.hx` | tag dispatch USE/DROP/… |
| `TransitionHelper.doCommandHelper` | same | body |
| `TransitionHelper.use` | same | use pipeline |
| `TransitionHelper.drop` | same | drop + clothing |
| `TransitionHelper.swap` | same | swap |
| `TransitionHelper.doContainerStuff` | same | containers |
| `TransitionHelper.doTransitionIfPossible` | same | apply TransitionData |
| `DoChangeNumberOfUsesOnActor/Target` | same | multi-use + loved-food bare-hand extra (`say`/`doEmote`) |
| `GlobalPlayerInstance.use/drop/swap` | `server/GlobalPlayerInstance.hx` | often delegates |
| `GlobalPlayerInstance.move` | same | → MoveHelper |
| `GlobalPlayerInstance.say` / `sayHelper` | same | speech + commands |
| `GlobalPlayerInstance.isObjYum` / `isObjMeh` / `isObjSuperMeh` | same | hasEatenMap + live `ServerSettings.YumBonus` |
| `GlobalPlayerInstance.canEatObj` / `canFeedToMeObj` | same | superMeh / store gates (live YumBonus) |
| `GlobalPlayerInstance.displayFood` / `DisplayBestFood` | same | LS food label + lite search (live YumBonus U-count) |
| `GlobalPlayerInstance` eat / hasEatenMap reduce | same | live YumBonus fill on eat |
| Rust `compute_eat_ex` / `is_obj_yum_ex` / `is_obj_super_meh_ex` / `format_display_food_text_ex` | `ol-sim/yum.rs` | YUM-LIVE-SETTINGS pure |
| Rust `YumState::eat_ex` / `try_eat_ex` / `display_food_label_ex` | same | live YumBonus book |
| Rust `resolve_yum_bonus` / `can_eat_obj_ex` / `score_food_candidate_ex` | same | sanitize + gates + lite score |
| Rust `ProcessFoodOpts.yum_bonus` / `process_food` isYum | `ol-sim/search_best_food.rs` | SearchBestFood live band |
| Rust `try_eat_held` / mount-eat / feed-other craving | `ol-sim/lib.rs` + `use_transition.rs` | `state.gameplay.yum_bonus` |
| `PlayerAccount.displayYum` / `!YUM` | `server/PlayerAccount.hx` / say | toggle food LS |
| `GlobalPlayerInstance.kill` / `DoDamage` | same | combat (animal factors) |
| `GlobalPlayerInstance.makeWeaponBloodyIfNeeded` | same | knife/sword → bloody on deadly animal (ttc=3) |
| `TimeHelper.DoTimeOnPlayerObjects` held bloody | `server/TimeHelper.hx` | `-1` auto-clean bloody → clean |
| `TransitionHelper` isNeverDrop / isBloody re-arm | `server/TransitionHelper.hx` | DROP refuse + ttc=3 unstick |
| Rust `make_weapon_bloody_if_needed` / `bloody_weapon_after_strike` / `try_bloody_weapon_auto_clean` | `ol-sim/weapons.rs` | COMBAT-BLOODY pure |
| Rust `bloody_weapon_auto_decay_base_ttc` / `never_drop_*` / bow damage 9/12 | same | PatchTransitions 3/2/6 + DROP polish |
| Rust `apply_bloody_weapon_transform` / `tick_held_bloody_auto_clean` | `ol-sim/lib.rs` | HIT/HUNT/DROP + vitals auto-clean |
| Rust `held_object_speed_mult` bloody override | `ol-sim/move_speed.rs` | 0.75/0.85/0.6 over content |
| Rust `BowEscapeEffects.time_to_change` | `ol-sim/animal_damage.rs` | TryAnimaEscape ttc=2 |
| `DoDamage` GetTransition(weapon,0) / doWound / setHeld wound | `server/GlobalPlayerInstance.hx` | WEAPON-WOUND-TRANS |
| `DoDamage` GetTransition(animal,0) attacker==null residual | same | WEAPON-ANIMAL-ZERO: equip/ground wound + fromObj.id=newActor TTC |
| `ObjectHelper.isArrowWound` / `isWound` | `data/object/ObjectHelper.hx` | Arrow Wound vs Wound/Snake/Hog |
| `ObjectData.woundFactor` / `damage` | `data/object/ObjectData.hx` + ServerSettings patches | default 0.5; snake 0.98 |
| Rust `resolve_weapon_zero_transition` / `plan_weapon_zero_wound` / `should_do_wound` | `ol-sim/weapon_wound.rs` | pure weapon+0 plan |
| Rust `plan_animal_zero_wound_from_content` / `plan_animal_zero_residual` / `force_no_coins_on_equip` | same | animal+0 plan + residual TTC |
| Rust `bloody_weapon_from_zero_transition` / `object_wound_factor` / `effective_wound_factor` | same | content newActor + snake shoes |
| Rust `is_arrow_wound_description` / `is_arrow_wound_object` | `ol-sim/death_polish.rs` | Arrow Wound gate |
| Rust `apply_weapon_zero_wound_hit` | `ol-sim/lib.rs` | HIT Wound/Kill equip + ground + bloody |
| `takeCoins` / `CoinsOnWoundingFactor` / `darkNosaj` | `server/GlobalPlayerInstance.hx` | WALLET-COINS: steal on lethal + first wound equip |
| Rust `coins_stolen_on_wound` / `take_coins_say_text` | `ol-sim/weapon_wound.rs` | pure amount + `Got N coin(s)!` |
| Rust `Economy::take_coins_on_wound` | `ol-sim/economy.rs` | wallet gift path (no trade prestige) |
| Rust `apply_take_coins_on_wound` | `ol-sim/lib.rs` | live HIT wire; dark_nosaj + live factor |
| Rust `Player.dark_nosaj` | `ol-sim/player.rs` | session field (not saved); ×2 factor / restore gate |
| Rust `Player.praised_jinbali` | `ol-sim/player.rs` | session; Tarr praise / Dark Nosaj punish (**DARK-NOSAJ**) |
| Rust `plan_monument_use` / `format_cursed_message_word` | `ol-sim/dark_nosaj.rs` | pure Tarr 3112 + Dark Nosaj 2466 USE plan |
| Rust `apply_monument_use_side_effects` | `ol-sim/use_transition.rs` | live USE side-effects (prestige/lost/hits) |
| Rust `maybe_monument_feedback` | `ol-sim/lib.rs` | public say + CU broadcast with word |
| Rust `dark_nosaj_attack_damage_mul` / `blocks_health_and_prestige` | `ol-sim/dark_nosaj.rs` | DoDamage ×1.2 + addHealthAndPrestige gate |
| Rust `GameplayKnobs.coins_on_wounding_factor` | `settings_live` + `ol-config` LiveSettings | Haxe default 0.5 hot-reload |
| Rust `apply_animal_path_damages` / `apply_animal_zero_wound_hit` / residual plan | same | DoAnimalDamage → DoDamage animal+0 wire |
| Rust `Animal.map_object_id` / `apply_zero_residual` / `AnimalDeathEvent.object_id` | `ol-sim/animals.rs` | attacking form map id + pop clear |
| Rust `ObjectDef.damage` / `wound_factor` / `apply_default_combat_damage_patches` | `ol-content` | combat PatchObjectData table |
| `DoDamage` mosquito non-real / fever / yellowfeverCount | `server/GlobalPlayerInstance.hx` | COMBAT-FEVER-BLEED: doesRealDamage≠2156, infect roll |
| `TimeHelper` bleedingDamage / hasYellowFever vitals / fever timer | `server/TimeHelper.hx` | ObjectDef.damage bleed + fever food/heat + clear |
| Rust `moskito_damage_factor` / `plan_mosquito_fever_infect` / `does_real_damage` | `ol-sim/weapon_wound.rs` | pure fever infect + non-real gate |
| Rust `wound_object_id_for_bleed` / `object_damage_bleed_rate` | same | held wound vs hiddenWound bleed id |
| Rust `yellow_fever_food_drain` / `yellow_fever_heat_delta` / `wound_bleed_food_extras` | `ol-sim/food_store_max.rs` | fever vitals + 2× bleed food |
| Rust `apply_mosquito_fever_candidate` / `tick_body_fever_and_hidden_wound` | `ol-sim/lib.rs` | live infect + TTC clear/re-equip/survive GM |
| Rust `player_jungle_biome_love` / path moskito scale / spawn Mosquito | `ol-sim/lib.rs` | **COMBAT-MOSQUITO-KIND** biomeLove(JUNGLE) + `scale_damage_by_moskito_factor` + seeds |
| Rust `jungle_biome_love_for_mosquito` / `scale_damage_by_moskito_factor` | `ol-sim/hunt.rs` | pure person-color jungle love + damage scale |
| Rust `AnimalKind::Mosquito` / `is_deadly_animal` | `ol-sim/animals.rs` | 2156 path-deadly; chase uses is_deadly_animal=false |
| Rust tick ObjectDef.damage bleed / `is_yellow_fever` heal gate | same | tick_vitals COMBAT-FEVER-BLEED wire |
| `TimeHelper.UpdateEmotes` / `DoTimeStuffForPlayer` tick%30 | `server/TimeHelper.hx` | FEVER-EMOTE priority PE ladder |
| `GPI.hasYellowFever` / `isSuperHot` / `isSuperCold` / `doEmote` | `server/GlobalPlayerInstance.hx` | fever PE gates + SendEmoteToAll |
| Rust `resolve_fever_pe_emote` / `resolve_update_emotes` / `UpdateEmotesInput` | `ol-sim/fever_pe.rs` | pure ladder 7/21 + wound/combat/starve/ambient |
| Rust `tick_update_emotes` / `emit_feed_too_ill_feedback` | `ol-sim/lib.rs` | tick%30 PE + feed-ill say+PE |
| Rust `Player.last_time_emote_send` | `ol-sim/player.rs` | ambient 9s rate limit stamp |
| `GlobalPlayerInstance.doBaby` / `dropPlayer` | same | baby |
| `GlobalPlayerInstance.doBabyHelper` | same | HOLD/BABY pickup restore + exhaustion + follow + PlaceObject drop |
| `GlobalPlayerInstance.isHoldingChildInBreastFeedingAgeAndCanFeed` / `getMaxChildFeeding` | same | continuous nurse gate + cap |
| TimeHelper continuous breast-feed | `server/TimeHelper.hx` | FoodRestoreFactorWhileFeeding×FoodUsePerSecond; yum_bonus before food_store; hits−0.2/s; food-ceil FX |
| Rust `can_breastfeed` / `breastfeed_tick` / `pickup_feed_amounts` / `get_max_child_feeding` | `ol-sim/feed.rs` | BREASTFEED-EDGES pure |
| Rust `can_nurse_age` / `can_pickup_breastfeed_age` / `can_pickup_player_ages` | same | age≤6 continuous; age<6 pickup; carrier ≥ target+1 (no age≥14) |
| Rust `can_pickup_baby_distance` / `PICKUP_BABY_MAX_DISTANCE` | same | euclid ≤1.9 (Haxe PickupBabyMaxDistance) |
| Rust `drain_mother_nurse_cost` / `baby_food_ceil_changed` | same | yum_bonus first; ceil FX gate |
| Rust `is_droppable_on_baby_pickup` / `needs_force_drop_nested_hold` / `can_hold_baby_hands` | same | doBaby drop + hands |
| Rust `nurse_hits_heal` / `should_set_follow_on_hold` | same | hits heal; follow reassign |
| Rust `apply_do_baby_hold` | `ol-sim/lib.rs` | SAY HOLD + BABY tag; PU fan-out; fail PU+FRAME |
| Rust continuous nurse + bare NURSE | same | tick_vitals + apply_say; yum drain + food-ceil FX |
| `GlobalPlayerInstance.doDeath` / `doDeathHelper` | same | death reason + scatter + inherit |
| `GlobalPlayerInstance.placeGrave` | same | grave object + SendGraveInfo |
| `GlobalPlayerInstance.InheritCoins` / `InheritOwnership` | same | wallet past-actions + kids; object follow |
| `PlayerAccount.coinsInherited` / `ChangeScore` | `server/PlayerAccount.hx` | inherit weight + score fold |
| `GlobalPlayerInstance.exile` / `redeem` / follow | same | social |
| `MoveHelper` (using) | `server/MoveHelper.hx` | path, speed, force |
| `MoveHelper.move` / `moveHelper` | same | waitForForce, age gate, jump floor÷10, jump rate, path accept |
| `MoveHelper.updateMovement` | same | exactTx advance, OpenDoors, forceStopOnNextTile, finish PU |
| `MoveHelper.OpenDoors` | same | 2757→2758 / 2759→2760 + MX |
| `MoveHelper.CancleMovement` | same | waitForForce + force PU + optional VOG_UPDATE + always map chunk |
| Rust `cancel_movement` / `fold_relative_around_world` / `fold_world_pos_around_world` / `cancel_should_use_vog` | `ol-sim/lib.rs` + `move_path.rs` | **MOVE-VOG-WRAP** VOG flag + world-wrap cancel |
| `MoveHelper.moveHelper` world-wrap | same L550-586 | fold relative ±map size → CancleMovement(VOG) |
| `MoveHelper.calculateSpeed` | same | floor/road/biome + held/contain + **grave curse / close enemy** + dual shoes |
| `MoveHelper.calculateNewMovements` | same | fullPathHasRoad + off-road biome trunc |
| `MoveHelper.calculateNewPos` | same | mid-path half-step recon from elapsed×speed |
| Rust `calculate_new_pos` / `reconcile_mid_path_tile` / `LET_THE_CLIENT_CHEAT_LITTLE_BIT_FACTOR` | `ol-sim/move_path.rs` | **MOVE-MIDPATH** pure + wire on `apply_move_path_start` |
| `MoveHelper.calculateObjSpeedMult` | same | contained object mult clamp |
| `GlobalPlayerInstance.hasBothShoes` | `server/GlobalPlayerInstance.hx` | clothingObjects[2] **and** [3] non-zero |
| Rust `has_both_shoes` / `shoe_pair_ids` | `ol-sim/move_speed.rs` | dual-shoe parity |
| Rust `movement_age_allowed` / `still_waiting_for_force` / `jump_quad_with_floor` / `apply_jump_cost` / `springy_door_open_id` / `decay_jumped_tiles` | `ol-sim/move_path.rs` | TIMED-MOVEMENT-DEFAULT pure gates |
| Rust `apply_move_path_start` / `tick_move_paths` / `open_doors_on_commits` / `send_forced_player_update` | `ol-sim/lib.rs` | path start gates + tick OpenDoors/forceStop/AI force |
| **JUMP-BW-FULL** `GlobalPlayerInstance.jump` | `server/GlobalPlayerInstance.hx` L5098–5120 | not-held PU+BW+FRAME; held `dropPlayer` |
| `MoveHelper.JumpToNonBlocked` | `server/MoveHelper.hx` L473–519 | E/S/W/N free neighbor + VOG force abort MOVE |
| Client `JUMP` / AI `JUMP!` | `Server.hx` / `AiBase.sayHelper` | ignore x/y; call `jump()` |
| Rust `plan_player_jump` / `plan_jump_to_non_blocked` / `JUMP_EXHAUSTED_SAY` | `ol-sim/jump_bw.rs` | pure plan + BW always-on |
| Rust `apply_player_jump` / `try_jump_to_non_blocked` | `ol-sim/lib.rs` | live JUMP tag + AI plan.jump + MOVE held/blocked gates |
| Rust MOVE jump exhausted say | `apply_move_path_start` + `send_chat_ps` | Haxe `p.say('I am too exhausted!', true)` |
| Residual JUMP | — | full `dropPlayerHelper` transform + dual close force PU; TimeHelper periodic unstick |
| **CURSED-GRAVE-TELEPORT** `GPI.doServerCommand !TCG/!TV` | `server/GlobalPlayerInstance.hx` L5515 / L5577 | admin teleport via `WorldMap.cursedGraves` / `ovens` |
| `checkIfNotAllowed` / `teleport` / `doTeleport` | same L5772–5843 | canUseServerCommands → closest unblocked + blockedTeleportLocations cycle → snap+JumpToNonBlocked+VOG |
| Rust `parse_teleport_bang` / `pick_closest_teleport` / `pick_closest_from_index_map` / `push_blocked_teleport` / `clear_blocked_teleport` | `ol-sim/teleport_cmd.rs` | pure bang parse + closest pick + blocked list |
| Rust `Player.blocked_teleport_locations` | `ol-sim/player.rs` | session linear-index cycle list |
| Rust `apply_do_teleport` / `try_apply_teleport_bang` | `ol-sim/lib.rs` | live snap + JumpToNonBlocked + cancel_movement VOG; SAY hook (godmode gate) |
| Residual teleport | — | torus wrap distance; map_linear_index y−1; SearchNewHome oven AI; CursedGraveTime LiveSettings |
| `PlayerAccount.hasCloseBlockingGrave` / `calculateCloseBlockingGraveFitness` | `server/PlayerAccount.hx` | bone graves fitness → curse speed |
| `PlayerAccount.removeDeletedGraves` | same | drop id==0 |
| `GlobalPlayerInstance.getClosePlayer` / `isFriendly` / `isHoldingWeapon` | `server/GlobalPlayerInstance.hx` | hostile+weapon scan; ally+last-attack |
| `GlobalPlayerInstance.GetNumberLifingPlayers` | same | connection living count for curse pop gate |
| `Connection.SendCurseToAll` | `server/Connection.hx` | CU wire |
| Rust `resolve_grave_curse` / `has_close_blocking_grave` / `has_close_hostile_with_weapon` | `ol-sim/move_live_gates.rs` | S-MOVE-LIVE-GATES pure |
| Rust `live_move_speed_gates` / `apply_grave_curse_live_gates` / `player_move_speed` | `ol-sim/lib.rs` | wire gates + CU/PE on **path start + path finish + cancel** (**MOVE-GRAVE-ALL-PATHS**) |
| `GlobalPlayerInstance.isClose` / `isCloseToPlayer` | `server/GlobalPlayerInstance.hx` | squared Euclidean + wrap via CalculateDistance |
| `GlobalPlayerInstance.isCloseUseExact` / `isCloseToPlayerUseExact` | same | exact float via MoveHelper |
| `MoveHelper.isCloseUseExact` / `calculateExactQuadDistance` | `server/MoveHelper.hx` | transformFloat wrap + exactTx/Ty |
| `MoveHelper.isMoveing` | same | path active gate |
| `TransitionHelper.checkIfNotMovingAndCloseEnough` | `server/TransitionHelper.hx` | USE/DROP not-moving + held useDistance |
| `TransitionHelper.use` bow min-range | same | deadlyDistance>1.9 + animal + isCloseUseExact 1.5 |
| `TransitionHelper.use` Too close say | same L762-763 | `player.say('Too close...')` + `message='too close'` |
| `GlobalPlayerInstance.killHelper` Too close | `GlobalPlayerInstance.hx` L4420-4428 | bow deadly>1.9 + isCloseToPlayerUseExact 1.5 → PU + public say (no animal check) |
| `AiHelper.CalculateDistance` | `auto/AiHelper.hx` | torus half-map wrap squared distance |
| `ObjectData.useDistance` / `deadlyDistance` / `isAnimal` | `data/object/ObjectData.hx` | content range + moves>0 animal |
| Rust `in_use_range` / `in_use_range_ex` / `effective_use_distance` / `check_if_not_moving_and_close_enough` | `ol-sim/move_path.rs` | IS-CLOSE / action_range pure |
| Rust `is_close_use_exact*` / `calculate_exact_quad_distance_f` / `refuse_ranged_kill_too_close` / `refuse_ranged_use_too_close` | `ol-sim/move_live_gates.rs` | exact range + bow min-range pure (kill=player; use=+animal) |
| Rust `note_too_close_say` / `take_too_close_say` / `take_too_close_message` / `TOO_CLOSE_SAY` / `TOO_CLOSE_MESSAGE` | `ol-sim/move_live_gates.rs` | GPI-TOO-CLOSE pending public say + debug message |
| Rust `maybe_too_close_say_feedback` | `ol-sim/lib.rs` USE + HIT + SAY KILL | public `send_chat_ps` `TOO CLOSE...` + FRAME |
| Rust `apply_use_at` / `apply_drop` / REMV range gates | `ol-sim/use_transition.rs` + `lib.rs` | live USE/DROP/REMV action range |
| Rust `ObjectDef.use_distance` / `deadly_distance` / `moves` / `is_animal` | `ol-content` | content fields + weapon/animal patches |
| Rust OLC1 v7 `deadly_distance`/`use_distance`/`moves` encode/load | `ol-binary` + `ol-content/binary_cache` | **OLC1-DISTANCES** binary_use_dist |
| Rust `ClientObjectDef` parse+bake distances | `RustClient` `content.rs` / `content_binary.rs` | OLC1 v7 client cache fidelity |
| Rust `apply_default_animal_deadly_distance_patches` | `ol-content` | Haxe `AnimalDeadlyDistanceFactor` 0.5 |
| Haxe `ObjectData.writeToFile` / `readFromFile` | `data/object/ObjectData.hx` | binary deadly(f32)+use(i32); no moves in Haxe binary |

---

## Haxe: world & content

| Symbol | File | Role |
|--------|------|------|
| `WorldMap.generate` / `writeToDisk` / `getObjectHelper` | `server/WorldMap.hx` | map I/O + tiles |
| `WorldMap.getBiomeSpeed` | same | biome table + water-floor override |
| `WorldMap.isBiomeBlocking` | same | movement gate |
| `WorldMap.PlaceObject` / `TryPlaceObject` | same | free-tile search + replace re-home + grave swallow |
| `WorldMap.TransformObject` / `PlaceObjectById` | same | cart 778/3158 transform; id wrapper |
| Rust `place_object` / `try_place_kind` / `PlaceObjectOpts` | `ol-sim/place_object.rs` (via `death_polish`) | PLACE-OBJECT free_tile_search |
| Rust `place_object_by_id` / `place_object_near` / `place_grave_object` | same | live free search callers |
| Rust `can_be_placed_in_grave` / `contain_fits_slot` | same | grave swallow + size gate pure |
| Rust baby-hold drop → `place_object_by_id` | `ol-sim/lib.rs` doBabyHelper path | Haxe PlaceObject allowReplace=false |
| Rust bow 798 → `place_object_by_id(replace)` | `ol-sim/lib.rs` TryAnimaEscape | Haxe PlaceObject allowReplace=true |
| `ObjectHelper.WriteMapObjHelpers` / `WriteToFile` / `ReadFromFile` | `data/object/ObjectHelper.hx` | nested recursive container disk (NESTED-OLW1 source) |
| `ObjectHelper.toArray` / `toString` / `readObjectHelper` | same | wire ids + colon subcontained |
| `ObjectHelper.TransformToDummy` | same | multi-use dummy rebuild on load |
| `ObjectHelper.InitObjectHelpersAfterRead` | same | post-load graves/owners rewire → `postload_wire` / `postload_owners` |
| `ObjectHelper.CalculateSurroundingWallStrength` / `FloorStrength` | same | decay protection |
| `TemperatureHandler.*` | `server/TemperatureHandler.hx` | tile heat → `map_temp_player` + `world_time` |
| `ObjectData.*` | `data/object/ObjectData.hx` | defs |
| `TransitionImporter.*` | `data/transition/TransitionImporter.hx` | tables |
| `Category.probSet` | `data/transition/Category.hx` | TransformTarget weights |
| `ObjectHelper.CalculateTimeToChangeForObj` | `data/object/ObjectHelper.hx` | decay timers |
| `ObjectHelper.groundObject` | same | leave-behind under movers |

---

## Haxe: AI

| Symbol | File | Role |
|--------|------|------|
| `AiBase.doTimeStuff` / `doTimeStuffHelper` | `auto/AiBase.hx` | **priority ladder** (AI-PRIO) |
| `AiBase.checkIsHungryAndEat` | same | hungry hysteresis |
| `AiBase.escape` | same | flee rung |
| `AiBase.isFeedingChild` / `isFeedingPlayerInNeed` | same | feed band |
| `AiBase.isMovingToPlayer` / `isChildAndHasMother` | same | follow band |
| `AiBase` profession methods | same | job bodies (AI-JOB-*) |
| `hasOrBecomeProfession` / `countProfession` | same | profession caps + sticky last |
| `doBasicFarming` / `doCarrotFarming` / `doBerryFarming` / `doAdvancedFarming` | same | farm jobs |
| `doPrepareSoil` / `doPrepareRows` / `doComposting` / `doPlant*` / `doHarvest*` / `doWateringOn` | same | farm steps |
| `doBaking` / `doBakingHelper` / `makeRawPies` | same | baker job (AI-JOB-BAKER) |
| `craftItem` / GetOrCraft | same | crafting |
| `AiHelper.GetClosest*` / food | `auto/AiHelper.hx` | spatial |
| `AiHelper.CountCloseObjects` | same | home-radius farm counts |
| `AiHelper.pies` / `AiBase.rawPies` | same | cooked/raw pie id tables |

---

## Haxe: settings (`CONFIG-SETTINGS`)

| Symbol | File | Role |
|--------|------|------|
| `ServerSettings.readFromFile` / `writeToFile` | `settings/ServerSettings.hx` | RTTI dump + override lines; boot + hot reload |
| `TimeHelper.ReadServerSettings` | `server/TimeHelper.hx` | gate reload every 200 ticks |
| `ServerSettings.EternalWinter` / `SeasonDuration` / `NumberOfAis` | same | season force / length years / AI count |

---

## Rust: fertility + twin wait queue (FERTILITY-TWINS / twin_sockets)

| Symbol | File | Role |
|--------|------|------|
| `is_fertile` / `age_fertile` / `can_birth_full` | `ol-sim/src/fertility.rs` | Haxe `isFertile` (deleted + female + age 14–42) + birth cooldown/gestation |
| `FertilityState::format_query_sex` | same | `?FERTILE` male/ready/gestating/cooldown |
| `TwinWaitQueue` / `TwinJoinOutcome` / `ReadyTwinParty` | `ol-sim/src/twins.rs` | protocol twin_code_hash party size 2–4; join/leave/status |
| `TwinRegistry` / `TwinPeer` | same | multi-server peer list (TWIN-MULTI-SERVER) |
| `TwinRegistry::add` / `from_endpoints` | same | host:port **dedup** (preserve last_pong) |
| `TwinRegistry::sync_endpoints` | same | live re-sync from config; keep pongs on match |
| `TwinRegistry::record_pong` / `clear_stale_pongs` | same | health stamp + age-out (`DEFAULT_PEER_STALE_SECS`) |
| `TwinRegistry::ping_targets` / `TwinPeer::is_fresh` | same | peers due for future socket ping |
| `parse_twin_pong_payload` | same | `TWINPONG host port` parse |
| `apply_twin_join` / `process_ready_twin_party` | `twin_party_live.inc.rs` | live party wait + simultaneous baby birth |
| `player_is_female` / `player_is_fertile` | `ol-sim/src/lib.rs` | content name heuristic / po 19 default |
| `due_mothers` / `format_twin_party_ready` / `format_twin_wait_ps` | `gestation_tick.rs` | tick poll + PS helpers |
| BIRTH / GESTATE / NURSE / HOLD feed / mother pick | `lib.rs` apply_say / tick_vitals | full `is_fertile` gate (not age-only) |
| LOGIN twin_code_hash → Raw TWINJOIN | `ol-net/src/lib.rs` | after accept LOGIN intent |
| Raw `TWINPONG host port` → `record_pong` | `lib.rs` apply_intent | inject path until inter-server sockets |
| LiveSettings `twin_peers` → `sync_endpoints` | `settings_live.rs` / `ol-config` | hot-reload peer list |
| Sim tick → `clear_stale_pongs` | `lib.rs` run_sim_loop | age `last_pong` → `?TWINS` `@-` |
| Disconnected → `twin_wait.leave` | `lib.rs` | drop waiter on disconnect |
| Tests | `fertility::*` / `twins::*` / `twin_pong_path_*` / `twin_peers_hot_reload_*` / `say_twins_lists_peers_or_none` / birth + leave | pure + live |


## Rust: twin party residual (TWIN-PARTY-RESID / twin_wait_edges)

| Symbol | File | Role |
|--------|------|------|
| `TwinHeartLinks` / `register_party` / `on_member_death` | `twin_heart.rs` | same-server party link after birth |
| `is_murder_death_reason` / `format_twin_heart_ps` | same | murder gate + PS |
| `TwinWaitQueue::poll_timeouts` / `TWIN_WAIT_TIMEOUT_SECS` | `twins.rs` / `twin_heart.rs` | wait-queue eviction |
| `apply_twin_heart_link_on_murder` | `twin_party_live.inc.rs` | live broken-heart wound |
| KILL/HIT kill → murder gate / else `remove_player` | `lib.rs` apply_intent | combat death heart-link |
| vitals death → `twin_heart.remove_player` | `lib.rs` tick | non-murder natural death cleanup |
| `poll_twin_timeouts` + `format_twin_timeout_ps` | `gestation_tick` / tick | wait eviction PS |
| `ObjectDef.male` / `parse male=` | `ol-content` | Haxe ObjectData.male |
| `player_is_female` content_male | `lib.rs` | person race → male flag |
| `format_twin_wait_ps_code` | `twin_heart` / `apply_twin_join` | wait PS with short code |
| Tests | `twin_heart::*` / `twin_heart_link_*` / `twin_poll_timeouts_*` / `parse_male_flag` | pure + live |

**Out of scope:** multi-server twin peers (parked stub).

**Haxe note:** `Connection.loginHelper` has `// TODO twins` — Rust implements protocol wait queue as product; multi-server peers are a Rust product registry (Haxe had no peer sockets).

---

## Rust: death polish (GPI-DEATH / GPI-DEATH-POLISH / GPI-PLACE-GRAVE)

| Symbol | File | Role |
|--------|------|------|
| `DeathCause` / `food_death_wire` / `hunger_death_wire` | `ol-sim/src/death_cause.rs` | reason_* tags; nursing_hunger when holding baby |
| `apply_inherit_coins` / `InheritContext` | `ol-sim/src/death_inherit.rs` | past-actions + kids + grave/treasury residual |
| `choose_new_leader` / `count_leadership_power` | same | Haxe ChooseNewLeader / countLeadershipPower (+ family_prestige) |
| `apply_inherit_ownership_on_helpers` / `remove_owner_from_helper` / `add_owner_to_helper` | same | InheritOwnership pure |
| `stamp_grave_soul` / `account_soul_token` | same | grave owners_by_account soul key |
| `apply_death_polish` / `place_grave_with_soul` | `ol-sim/src/death_polish.rs` | doDeathHelper orchestration |
| `select_grave_object_id` / `resolve_place_grave_id` | same | placeGrave 3053/752/87 (+ content fallback) |
| `is_wound_description` / `is_wound_object` | same | ObjectHelper.isWound |
| `place_grave_for_player` / `place_grave_for_conn` / `place_grave_on_death*` | same | full placeGrave + all-death wire |
| `send_grave_info_to_all` / `format_grave_place_log` | same | Haxe SendGraveInfoToAll |
| `format_grave_info` | `ol-protocol/src/wire_out.rs` | `GRAVE\nx y creator_p_id\n#` |
| `apply_death_inheritance` | `ol-sim/src/lib.rs` | live entry → `death_polish::apply_death_polish` (all death paths) |
| `AccountBook::family_prestige_for` / `set_family_prestige` / `record_grave` | `ol-sim/src/accounts.rs` | Haxe familyPrestige + graves |
| Tests | `death_inherit::*` / `death_cause::*` / `death_polish::*` | pure + integration |

---

## Rust: PlayerSoul AI context (S-SOUL)

| Symbol | File | Role |
|--------|------|------|
| `PlayerSoul` / `add_interaction` / `get_memory_text` | `ol-sim/src/player_soul.rs` | Haxe interaction FIFO (`AiMemoryMaxEntries`=20) |
| `add_chat_entry` / `get_chat_memory_text` (+ filter) | same | Haxe chat FIFO (`AiChatMemoryMaxEntries`=100); speaker filter product ext |
| `InteractionType` / `InteractionData` / `ChatEntry` | same | Haxe enums/data classes |
| `get_temperature_label` | same | absolute heat bands freezing…sweltering (≠ `heat_ideal::label_for_heat`) |
| `get_temperature_context_text` | same | body + tile ambient labels |
| `get_home_context_text` / `home_direction` | same | miles + cardinal/intercardinal; at-home &lt;20; unset home |
| `get_family_text` / `get_external_family_text` | same | father/mother only (children skipped as Haxe) |
| `get_status_text` / `get_external_status_text` | same | food% 20/50 + wound + super hot/cold |
| `get_profession_text` | same | assigned/last profession |
| `get_soul_text` / `get_external_intro` | same | first-person / third-person LLM context via `SoulView` |
| `get_prestige_class_name` | same | → `PrestigeClass::wire_name` |
| `get_combat_prestige_label` | same | → `reputation::label_from_lost_combat(...).display()` |
| `SoulView` | same | pure snapshot for prompt builders (no GPI pointer) |
| `AI_MEMORY_MAX_ENTRIES` / `AI_CHAT_MEMORY_MAX_ENTRIES` | same | Haxe ServerSettings defaults |
| Tests | `player_soul::*` | FIFO, golden strings, labels, home, status |
| `Player.soul` | `ol-sim/src/player.rs` | sticky Haxe `playerSoul` (AI-SOUL-WIRE) |
| `Player.partner_p_id` / `true_age` / `assigned_profession` / `last_profession` | same | partner link + wall-clock age + free profession strings |
| `is_angry_or_terrified` / `person_looks_female` / `person_is_female` / `haxe_season_text` / `haxe_season_roll_text_and_hardness` / `sticky_profession_pair` | `player_soul_wire.rs` | live field helpers (+ farm + free override) |
| `is_super_hot_for_person` / `is_super_cold_for_person` | same | Haxe color-person TemperatureImpactBelow thresholds |
| `Environment.season_text` / `season_hardness` | `environment.rs` | Haxe TimeHelper.SeasonText / SeasonHardness |
| `reseed_season_length_after_roll` | `settings_live.rs` | also sets season_text + season_hardness |
| `SimState::soul_view_for` / `player_soul_text` / `player_external_intro` | `soul_live.rs` | assemble SoulView from Player+world |
| `SimState::add_player_soul_interaction` / `add_player_soul_chat_entry` | same | memory writers with `ai_*_memory_max_entries` |
| `SimState.ai_memory_max_entries` / `ai_chat_memory_max_entries` | `lib.rs` | Haxe AiMemoryMaxEntries / AiChatMemoryMaxEntries (defaults 20/100) |
| Residual | — | LiveSettings hot-reload of caps; ObjectData.male; connection.isAi; weapon deadlyDistance; S-AIH AiHandler; live combat/feed hooks (none in Haxe either) |

---

## Rust: combat reputation hit float (REPUTATION-HIT / hit_reputation)

| Symbol | File | Role |
|--------|------|------|
| `attack_was_legit` | `ol-sim/src/reputation.rs` | Haxe `damage < 2 * target.lostCombatPrestige` |
| `compute_hit_reputation` / `HitReputationInput` / `HitReputationDelta` | same | post-DoDamage lostCombat deltas + category ceil cost |
| `PrestigeCostCategory` / `format_prestige_cost_global_message` | same | child/elder/ally/relative/woman GM text |
| `PrestigeCostFactors` / `compute_hit_reputation_with_factors` | same | live multipliers; all five from GameplayKnobs (**PRESTIGE-ALLY-COST** + **C-SS-MORE**) |
| `combat_reputation_restore_delta` / `COMBAT_REPUTATION_RESTORE_PER_YEAR` | same | TimeHelper calm restore pure |
| `ReputationBook::apply_hit_delta` | same | invert lost deltas → reputation scores |
| `DEVIL_MASK_CLOTHING_ID` / PrestigeCost* constants | same | ServerSettings defaults; all five LiveSettings via PRESTIGE-ALLY-COST + C-SS-MORE |
| `GameplayKnobs::prestige_cost_factors` | `settings_live.rs` | LiveSettings → PrestigeCostFactors (child/elderly/ally/close/woman) |
| `apply_connecting_hit_reputation` | `ol-sim/src/lib.rs` | live HIT Wound+Kill + SAY KILL; book + stats mirror; category prestige + GM; recompute exile-aware is_ally; all live PrestigeCost* |
| `tick_combat_reputation_restore` | same | vitals tick calm restore; gates on `Player.dark_nosaj` (**WALLET-COINS**) |
| `player_wears_clothing_id` | same | Devil Mask clothing scan |
| `CombatState::resolve_kill` | `ol-sim/src/combat.rs` | kills/deaths/score prestige only — **no** `lost_combat_prestige` |
| Tests | `reputation::*` / `say_hit_*` / `say_kill_illegal_no_double_*` / `tick_restore_*` / `resolve_kill_leaves_lost_*` | pure + live |

---

## Rust: map-temp player heat (MAP-TEMP-PLAYER / vitals_tile_temps)


| Symbol | File | Role |
|--------|------|------|
| `apply_balance_temperature_area` | `ol-sim/src/map_temp_player.rs` | Haxe `TemperatureHandler.BalanceTemperatureArea` (doLocalHeat=false) |
| `ensure_tile_temperature` / `get_tile_temperature` | same | init sparse `tile_temps` / read |
| `player_ambient_from_tile_temps` / `update_player_temperature` | same | radius-5 balance + water half + body heat step |
| `clothing_factor_from_slots` | same | mild clothing insulation stub |
| `body_heat_step` / `heat_food_extra` / `clamp_heat` | `ol-sim/src/heat_ideal.rs` | Haxe impact-per-sec body heat + food extra |
| `Player.heat` / `Player.last_temperature` | `ol-sim/src/player.rs` | body heat 0..1 + ambient sample |
| `tick_vitals_with_metrics` temperature pass | `ol-sim/src/lib.rs` | live: `update_player_temperature` → `p.heat`/`last_temperature`; food drain + HX |
| path move + `player_move_speed` heat | `ol-sim/src/lib.rs` | uses `p.heat` (not `temperature_at_biome`) |
| Rust `resolve_grave_curse` / `has_close_blocking_grave` / `has_close_hostile_with_weapon` | `ol-sim/move_live_gates.rs` | S-MOVE-LIVE-GATES |
| Rust `live_move_speed_gates` / `apply_grave_curse_live_gates` | `ol-sim/lib.rs` | wire speed + is_cursed on path start/finish/cancel |
| login bootstrap HX | `SimState` bootstrap packets | `format_heat_change(p.heat, …)` |
| Tests | `map_temp_player::*` / `heat_ideal::*` / `tick_vitals_map_temp_player_*` / HX + desert/snow/indoor food | pure + live wire |

---

## Rust: AI priority ladder (AI-PRIO / AI-PRIO-LIVE)

| Symbol | File | Role |
|--------|------|------|
| `resolve_priority_rung` / `PriorityRung` / `PrioritySensors` | `RustServer/crates/ol-sim/src/priority_ladder.rs` | Haxe `doTimeStuffHelper` order |
| `goal_from_rung` / `pick_goal_from_ladder` / `pick_goal_with_sensors` | same | rung → Goal action layer |
| `update_is_hungry` / `check_is_hungry_and_eat_effects` | same | hungry hysteresis + eat side-effects |
| `resolve_escape_threat` / `escape_target_xy` / `escape_side_effects` / `skip_escape_for_hunt` | same | Haxe `escape` pure gates |
| `compute_do_stuff` / `is_superbad_temp` / `effective_do_stuff` | same | superbad heat + threat-dist policy |
| `age_job_index` / `age_rotated_job_sequence` / `AgeRotatedJobKind` | same | berry/basic/baking/pottery/sheep cycle |
| `sensors_from_simple` / `sensors_from_ext` / `LiveSensorExtras` | same | sensor fill (simple + live extras) |
| `fill_live_sensors` / `LiveSensorInput` / `LiveSensorBundle` / `pick_goal_from_live_sensors` | same | **AI-PRIO-LIVE** world→sensor bundle |
| `get_close_deadly_player` / `DeadlyPlayerCandidate` / `is_deadly_player_candidate` | same | Haxe `GetCloseDeadlyPlayerHelper` pure |
| `get_close_player_target` / `PlayerTargetCandidate` | same | Haxe `GetClosePlayerTargetHelper` pure |
| `escape_context_from_threats` / `threat_quad_from_deadly` | same | EscapeContext from live animal/player |
| `is_moving_to_player_needed` / `child_with_mother_follow_tiles` / `ordered_follow_max_tiles` | same | Haxe `isMovingToPlayer` distance gates |
| `decide_follow_walk` / `plan_follow_sticky_clear` / `ally_goto_speaker_xy` / `tick_ai_follow_walk` / `try_ai_follow_path_to` | `ol-sim/ai_follow_walk.rs` + lib live.inc | **AI-FOLLOW-WALK** continuous follow + ally Goto pathfind |
| `PriorityBand` (Flee/Food/Feed/Craft/Job/Follow) | same | coarse labels |
| `pick_goal_ext` / `pick_goal` | `RustServer/crates/ol-sim/src/ai_goals.rs` | thin self-play ladder |
| `AnimalWorld::get_close_deadly_animal` / `CloseDeadlyAnimal` | `RustServer/crates/ol-sim/src/animals.rs` | Haxe `GetCloseDeadlyAnimal` (moves², deadly kinds) |
| selfplay live ladder | `ol-server/src/selfplay.rs` | `pick_goal_from_live_sensors` on threat/mother/superbad |

---

## Rust: farmer profession (AI-JOB-FARM / AI-JOB-FARM-WIRE / AI-JOB-FARM-LIVE)

| Symbol | File | Role |
|--------|------|------|
| `has_or_become_profession` / `FarmProfessionRuntime` / `FarmTaskState` | `ol-sim/src/farmer_profession.rs` | sticky last + max caps + task hysteresis |
| `Player.farm_profession` / `Player.farm_task` | `ol-sim/src/player.rs` | sticky runtime + task hysteresis across ticks (AI-JOB-FARM-LIVE) |
| `parse_farm_profession_speech` / `assign_farm_from_speech` | `farmer_profession.rs` | `FARMER!`/`WHEAT!`→BASIC, `CARROT!`/`BERRY!`/`ROW!`/`SOIL!`/`WATER!` → assigned+last |
| `keep_bushes_alive` / `keep_bushes_alive_count` / `KEEP_BUSHES_ALIVE_*` | same | Haxe keepBushesAlive: living bushes &lt;20 → ShortCraft(1137,389) |
| `do_critical_farm_slice` | same | age-gated bushes + basic + carrot (doCriticalStuff farm slice) |
| `short_craft_apply` / `short_craft_apply_resolved` / `ShortCraftApply` / `ShortCraftInput` / `farm_action_short_craft_apply` / `_ex` | same | pure shortCraft edges: hungry gate, USE/drop/seek(+craft_if_needed), snow/ocean, weak skewer resolved, carrot row, maxNewActor |
| `new_actor_count_with_held` | same | Haxe CountCloseObjects(newActor)+held==newActor |
| `decide_farm_job` / `do_basic_farming` / `do_carrot_farming` / `do_berry_farming` / `do_advanced_farming_step` | same | job sequences → `FarmAction` |
| `do_plant` / `do_harvest_wheat` / `do_harvest_corn` / `do_watering_on` | same | hysteresis helpers |
| `do_prepare_soil` / `do_prepare_rows` / `do_composting` | same | soil/rows/compost; rows call keep_bushes_alive when dying present |
| `pick_farmer_goal` / `farmer_pipeline_targets` | same | reverse-craft pipeline seek |
| `age_rotated_farm_profession` / `resolve_farm_assigned_job` | same | AssignedJob / age job 0–1 |
| `fill_farm_counts_from_map` / `fill_farm_counts_from_map_with_floor` / `FarmMapObj` | `farmer_profession` + `farm_spatial_inc.rs` | bulk home-radius snapshot (exclusive square) |
| `count_close_objects_at` / `count_close_objects_ex` / `count_close_objects_with_piles` | same | Haxe CountCloseObjects: +1 parent, +uses pile, 233/300 specials |
| `in_count_close_square` / `is_ignored_floor` / `count_close_pile_specials` | same | half-open square geometry; IsIgnoredFloor; pile specials |
| `count_corn_seeds_near` / `farm_radius_table` / `soil_units_from_map` | same | countCorn r=20 (held 1115/1120/1247 only); radii; soil units |
| `try_decide_farm_from_rung` / `farm_action_to_goal` / `farm_job_rung_label` | same | ladder bridge + goal map |
| `farm_goal_from_map_and_rung` / `farm_goal_from_counts_and_rung` | same | fill→decide→goal composition for live tick |
| selfplay farmer plan | `ol-server/src/selfplay.rs` | uses `pick_farmer_goal` (not map-fill yet) |

---

## Rust: baker profession (AI-JOB-BAKER / AI-JOB-BAKER-WIRE / AI-JOB-BAKER-LIVE)

| Symbol | File | Role |
|--------|------|------|
| `has_or_become_baker` / `has_or_become_baker_filtered` / `BakerProfessionRuntime` / `BakerTaskState` | `ol-sim/src/baker_profession.rs` | sticky last + max caps + stage 0/1/2/3 + makeRawPies + kindling/carrot/wheat task flags |
| `count_baker_peers` / `count_baker_peers_filtered` / `BakerPeerSnapshot` | same | Haxe countProfession filters (deleted/age/wound/food/home/follow) |
| `Player.baker_profession` / `Player.baker_task` | `ol-sim/src/player.rs` | sticky runtime + task hysteresis across ticks (AI-JOB-BAKER-WIRE) |
| `parse_baker_profession_speech` / `assign_baker_from_speech` / `resolve_baker_assigned_job` | same | `BAKER!` → assigned+last sticky |
| `do_baking` / `decide_baker_job` / `BakeAction` / `BakeCounts` | same | pure doBakingHelper → craft/shortCraft |
| `pre_profession_dough` / `hot_oven_bake` / `knife_bread_stage` / `make_raw_pies` | same | dough, hot oven pies (mutton maxNewActor=4), knife bread, pie hysteresis |
| `handle_milk` / `craft_item_max` / `make_or_collect` | same | milk pouch/cream/butter; craftItemMax; kindling collect hysteresis |
| `note_raw_pie_crafted` / `count_pies` | same | Haxe `countPies` for `extraPies % 4` (mutton/carrot bias) |
| `plant_carrots_for_baker` / `harvest_wheat_for_baker` / `sheep_herding_for_baker` | same | sequenced mid fallthrough; sheep defers to full `is_sheep_herding` when lambs/calves/milk present |
| `fill_berry_bowl_if_needed` / `make_seats_and_cleanup` / `make_seats_and_cleanup_ex` | same | berry bowl; seats craft 2828 when BOWLFILLER / force DeferSeatsCleanup |
| `bake_counts_from_nearby` / `should_drop_near_oven` / `consider_drop_near_oven` / `drop_near_oven_anchor` | same | mock snapshot + bakery drop staging (home/oven anchor) |
| `fill_bake_counts_from_map` / `_ex` / `_with_floor` / `BakeMapObj` | same | CountCloseObjects fill: half-open square, pile uses, IsIgnoredFloor |
| `bake_action_short_craft_apply` / `_ex` / `baker_short_craft_apply` / `baker_short_craft_limits` | same | shortCraftOnTarget USE/drop/seek + maxNewActor + craft_if_needed + hungry ex (AI-JOB-BAKER-LIVE) |
| `pick_oven_near_home` / `OvenCandidate` / `baker_chebyshev` / `baker_radius_table` | same | spatial oven GetClosest (Hot→Burn→Adobe→Wood) |
| `try_decide_baker_from_rung` / `baker_max_people_for_dispatch` | same | ladder bridge (Assigned 100 vs age default 1) |
| `baker_goal_from_map_and_rung` / `baker_goal_from_counts_and_rung` | same | fill → decide → Goal compose (AI-JOB-BAKER-WIRE) |
| `infer_baker_pipeline_stage` | same | inventory → stage for selfplay / pick_goal |
| `pick_oven_parent` / `OvenState` / `is_oven_id` | same | Hot 250 → Burning 249 → Adobe 237 / Wood-filled 247 |
| `pick_baker_goal` / `baker_pipeline_targets` / `bake_action_to_goal` | same | reverse-craft pipeline seek |
| `Profession::Baker` / `BAKER_TARGET_ID` | `ol-sim/src/ai_goals.rs` | thin self-play profession (Cooked Carrot Pie 273) |
| selfplay baker plan | `ol-server/src/selfplay.rs` | `pick_baker_goal` + `infer_baker_pipeline_stage` when `Profession::Baker` |

---

## Rust: shepherd profession (AI-SHEPHERD / sheep_herding)

| Symbol | File | Role |
|--------|------|------|
| `has_or_become_shepherd` / `ShepherdProfessionRuntime` | `ol-sim/src/shepherd_profession.rs` | sticky last + max people + weight; max&lt;0 high-prio no-assign |
| `Player.shepherd_profession` | `ol-sim/src/player.rs` | sticky last/assigned/weight across ticks |
| `parse_shepherd_profession_speech` / `assign_shepherd_from_speech` / `resolve_shepherd_assigned_job` | `shepherd_profession.rs` | `SHEPHERD!` → assigned+last |
| `count_shepherd_peers` / `count_shepherd_peers_filtered` / `ShepherdPeerSnapshot` | same | pure `countProfession('SHEPHERD')` |
| `is_sheep_herding` / `SheepHerdingResult` / `ShepherdAction` / `ShepherdCounts` | same | full isSheepHerding SM + handleMilk return-false quirk + profession clear |
| `do_feed_lambs_and_calfs` | same | early mid: 659+1489, 258+603/542, 1247+1462/1459; cap 10 |
| `handle_milk_for_shepherd` | same | ungated craft 4081 from zero (baker `handle_milk` stays stock-gated) |
| `count_corn` / `fill_berry_bowl_for_shepherd` | same | countCorn held 1115/1120/1247 only; held 253→bush (TODO BOWLFILLER dead) |
| `sheep_herding_steps_for_baker` / `try_decide_shepherd_from_rung` | same | baker mid align + ladder bridge |
| `ProfessionScanKind::Shepherd` / `shepherd_profession_scan_tick` / `shepherd_counts_from_scan` | `profession_scan.rs` | scan→counts→shortCraft USE/DROP |
| `age_rotated_to_scan_step(SheepHerding)` / assigned SHEPHERD plan | same | jobByAge==4 + assignedProfession SHEPHERD → isSheepHerding(100) |
| `BakeAction::DeferSheepHerding` expand | same | baker mid → full is_sheep_herding(2,5) |
| `make_stuff` / `make_stuff_ordered` / `MakeStuffInputs` / `MakeStuffAction` | `shepherd_mid_sites.inc.rs` | Haxe makeStuff full order: sharpie→bake→farm→sheep→fire (AI-SHEPHERD-MID) |
| `make_stuff_try` / `make_stuff_try_sheep` / `make_stuff_scan_tick` | shepherd + `profession_scan.rs` | pure expand + late LowPriority/AgeRotated live wire |
| `make_fire_food` / `FireFoodAction` / `FireFoodCounts` / `FireFoodProfessionRuntime` | `fire_food_profession.rs` | Haxe makeFireFood pure body (AI-MAKE-STUFF) |
| `ProfessionScanKind::FireFood` / `fire_food_profession_scan_tick` / `try_decide_fire_food_from_rung` | `profession_scan.rs` + `fire_food_rung.rs` | assigned/last FIREFOODMAKER makeFireFood(100) (AI-FIREFOOD-RUNG) |
| `is_handling_fire` / `is_handling_fire_fire_fuel_tail` / `HandlingFireAction` / `FireKeeperProfessionRuntime` / `FireFoodDispatchPath` | `handling_fire.rs` | isHandlingFire + Fire82 fuel cascade + late/hungry residual (AI-HANDLING-FIRE) |
| `ProfessionScanKind::HandlingFire` / `handling_fire_profession_scan_tick` / `expand_handling_fire_do_baking` / `late_make_fire_food_scan_tick` | `profession_scan.rs` + `handling_fire_live.inc.rs` | mid/temp/hungry/FIREKEEPER + DoBaking expand + late makeFireFood(1) |
| `ProfessionScanInput.is_winter` / plan TEMPERATURE + CONSIDER_MAKE_FOOD | `profession_scan.rs` | Season winter kindling; handleTemperature(2); hungry early isHandlingFire |
| `fire_food_peers_from_players_ex` / sticky `fire_food_assigned` | `profession_scan.rs` | FIREFOOD peer count + plan_assigned_job |
| `make_stuff_try_bodies` / `make_stuff_bake_has_work` / `make_stuff_fire_has_work` | `shepherd_mid_sites.inc.rs` | makeStuff bake+fire body expand |
| `fire_food_action_to_live_intent` / `Player.fire_food_profession` | `make_stuff_live.inc.rs` + `player.rs` | makeFireFood → shortCraft/craft live intent + sticky |
| `ladder_profession_scan_tick` fire_rt | `profession_scan.rs` | makeStuff fallthrough bake+fire with fire sticky writeback |
| `basic_farm_mid_try_sheep` / `FarmAction::DeferSheepHerding` | shepherd + farmer | doBasicFarming mid isSheepHerding(1); sticky BASICFARMER=1; max_profession for advanced |
| `do_basic_farming(max)` / `BASIC_FARM_*_MAX_PROFESSION` | `farmer_profession.rs` | outer maxProfession (2 / assigned 100) AI-FARM-STICKY |
| `do_basic_farming_after_sheep` / `make_sharpie_food` / `do_advanced_farming` | same | late plants → age&lt;20 sharpie → DeferAdvancedFarming(max) |
| `expand_advanced_farming_or_clear` / `FarmAction::ClearBasicFarmerWeight` | same | advanced body or profession['BASICFARMER']=0 |
| `apply_basic_farmer_weight_side_effect` | same | pure sticky weight write for mid/clear |
| `basic_farmer_weight_from_runtime` | same | read BASICFARMER weight (default 1.0) AI-FARM-STICKY |
| `farm_action_to_live_intent(..., farm_rt)` | `profession_scan.rs` | side-effect write + shortCraft intent; mid sheep(1)+advanced max |
| `apply_profession_*` farm_rt writeback | same | live `Player.farm_profession` sticky |
| `make_stuff_scan_tick` farm_rt | `make_stuff_live.inc.rs` | makeStuff→doBasicFarming sticky write path |
| wet 625 recount in `do_composting` | same | countCurrentObject+CountCloseObjects double-map quirk |
| `Profession::Shepherd` / `SHEPHERD_TARGET_ID` / `pick_shepherd_goal` | `ai_goals.rs` + shepherd | ladder + self-play goal |

---

## Rust: potter profession (AI-POTTER / pottery_job)

| Symbol | File | Role |
|--------|------|------|
| `has_or_become_potter` / `PotterProfessionRuntime` | `ol-sim/src/pottery_profession.rs` | sticky last + max people + stage 0/2/3/10; max&lt;0 critical no-assign |
| `Player.pottery_profession` | `ol-sim/src/player.rs` | sticky stage/last/assigned across ticks |
| `parse_potter_profession_speech` / `assign_potter_from_speech` / `resolve_potter_assigned_job` | `pottery_profession.rs` | `POTTER!` → assigned+last |
| `count_potter_peers` / `count_potter_peers_filtered` / `PotterPeerSnapshot` | same | pure `countProfession('POTTER')` |
| `pick_kiln_near_home` / `kiln_id_priority` / `pick_firing_kiln_near_home` / `KilnCandidate` | same | GetKiln wood-filled→cold→firing→sealed; r=20 |
| `fill_pottery_counts_from_map` / `PotteryMapObj` / `PotteryCounts` | same | CountCloseObjects + kiln + clay floor fill |
| `do_pottery` / `decide_potter_job` / `try_decide_potter_from_rung` | same | doPottery/Helper body → `PotteryAction` |
| `gather_clay` / `GatherClayInput` | same | basket/deposit/home pure spatial; `full_basket_near_deposit` |
| `PotteryAction::DropHeld { allow_piles, max_distance_to_home }` | same | Haxe dropHeldObject(0/1/10/40, allowAllPiles) |
| `pick_closest_clay_source` / `ClaySourceCandidate` / `apply_clay_source_to_gather_input` | same | Haxe L2968 deposit(125)+pit(409) closest |
| `empty_basket_drop_is_deposit_staging` | same | Haxe L2992 empty-basket drop near deposit maxDist 0 staging |
| `empty_basket_at_home_is_drop_extract` | same | **AI-POTTER-RESID**: L3013 empty hands → DROP extract (not USE) |
| `EMPTY_BASKET_HOME_SEARCH_RADIUS` | same | Haxe home basket-with-clay r=10 |
| `wet_nozzle_cleanup_action` / `CLAY_WITH_NOZZLE` | same | **AI-POTTER-RESID**: cleanUp shortCraft(285,285) / shortCraft(0,2110) |
| `do_pottery` wet-nozzle early wire | same | after charcoal basket / before fire kiln gather |
| `pottery_action_to_live_intent` EmptyBasketAtHome | `profession_scan.rs` | DropAt on clay-in-basket (not UseAt) |
| `apply_drop` empty-hand path | `ol-sim/src/lib.rs` | container take index 0 + non-perm floor swap |
| `do_pottery_on_fire_action` / `pottery_on_fire_counts_from_pottery` / `smith_pottery_action_to_pottery` | same | on-fire via shared smith body |
| `do_pottery_on_fire` L2946 residual | `smith_profession.rs` | **AI-POTTER-L2946**: wet-bowl fire; shortCraft(233,233); craftItem(296) nozzle; adobe after |
| `WET_CLAY_NOZZLE` / `FIRED_NOZZLE_TONGS` / `DEFAULT_MAX_CLAY_NOZZLES` | same | nozzle ids + cap |
| `count_wet_nozzle` / `count_nozzle` / `count_wet_crock_raw` | `pottery_profession.rs` | L2946 count helpers |
| `do_pottery` FIRED_NOZZLE_TONGS shortCraftOnGround | same | after residual craftItem(296) place/use tongs like bowl/plate/crock |
| `smith_action_to_live_intent` DeferPottery pottery fill | `profession_scan.rs` | live fill `PotteryOnFireCounts` → expand L2946 crafts |
| `pottery_action_short_craft_apply` | same (+`pottery_action_apply.inc.rs`) | ShortCraft → `SmithApply` USE/DROP |
| `pottery_action_to_goal` / `pick_potter_goal` / `potter_goal_from_*` | same | Goal compose + reverse-craft bias |
| `POTTER_TARGET_ID` / `Profession::Potter` / `parse_profession_token` | `ol-sim/src/ai_goals.rs` | clay bowl 235 seek default |
| ladder Potter arm | `priority_ladder.rs` | AssignedJob/AgeRotated/Idle → SeekObject(POTTER) |
| build wire | `ol-sim/build_ai_job_potter.rs` | lib mod+export + sticky + goals (idempotent) |

---

## Rust: smith profession (AI-JOB-SMITH / AI-JOB-SMITH-WIRE / AI-JOB-SMITH-LIVE / AI-JOB-SMITH-RESID)

| Symbol | File | Role |
|--------|------|------|
| `has_or_become_smith` / `SmithProfessionRuntime` | `ol-sim/src/smith_profession.rs` | sticky last + **max one SMITH** hard cap + stage |
| `Player.smith_profession` | `ol-sim/src/player.rs` | sticky runtime across ticks (AI-JOB-SMITH-WIRE) |
| `wipe_smith_on_eat` / `apply_consider_making_food_smith_wipe` | `smith_profession.rs` | Haxe `isConsideringMakingFood` SMITH=0 + last→Eating; hungry-path gate |
| `parse_smith_profession_speech` / `assign_smith_from_speech` / `resolve_smith_assigned_job` | same | `SMITH!` → assigned+last sticky |
| `count_smith_peers_filtered` / `SmithPeerSnapshot` / `smith_peer_snapshot` | same | pure `countProfession` filters + snapshot builder |
| `NpcSmithPeerRow` / `smith_peer_count_from_npc_rows` / `NpcSmithPeerRow::from_snapshot_fields` | same | NPC max-one smith pop without full `Player` (**AI-JOB-SMITH-RESID**) |
| `peer_home_coords` / `peer_is_wounded_from_held` | same | home fidelity + wound peer gate pure |
| `SteelChiselFamilyTable` / `from_content` | same | load-time `objectIdArrays[455]` cache (PatchObjectData once) |
| `PlayerSnapshot.home_x/y` / `is_last_smith` / `is_last_baker` / `is_last_potter` / `is_last_shepherd` / `is_last_farm` / `is_last_fire_food` | `player.rs` | published home + lastProfession sticky for NPC peer count |
| `NpcProfessionPeerRow` / `npc_peer_count_for_kind` | `profession_scan.rs` | multi-prof NPC peer roster (smith/baker/pottery/shepherd/farm/fire) |
| `pick_forge_parent` / `forge_id_priority` | `smith_profession.rs` | Firing 304 → Charcoal 305 → Forge 303 (id list) |
| `pick_forge_near_home` / `ForgeCandidate` / `chebyshev` | same | spatial GetForge r=20 closest per priority |
| `fill_smith_counts_from_map` / `fill_smith_counts_from_map_ex` / `MapObj` / `STEEL_COUNT_RADIUS` | same | CountCloseObjects filler (+ optional chisel family) |
| `prepare_smithing_tools` / `do_smithing` / `do_smithing_products` | same | prepare + product ladder → `SmithAction` (DeferPottery surfaces) |
| `critical_smith_shortcrafts` | same | hammer/stone+bloom; tongs+forge; charcoal |
| `STEEL_CHISEL_FAMILY` / `count_steel_chisel_stock` / `attach_chisel_family*` | same | objectIdArrays[455] seed + extras |
| `is_steel_chisel_family_description` / `collect_steel_chisel_family_ids` / `steel_chisel_family_from_content` / `chisel_family_extras_beyond_static` | same | Haxe `description.contains('Chisel')` content scan |
| `ProfessionScanInput.chisel_family_extra` | `profession_scan.rs` | live scan tick chisel extras |
| `plan_critical_craft_steps` CRITICAL_CRAFT tail | same | early sticky + critical shortCraft for all AIs |
| npc_ai multi-prof peer + home/wound + chisel table | `ol-server/src/npc_ai.rs` | primary-kind `npc_peer_count_for_kind` + snapshot home/last + load-time chisel extras |
| `SmithAction::DeferPottery` / `ShortCraftOnGround` | `smith_profession.rs` | pottery gate; cool crucible 324 |
| `smith_action_apply` / `SmithApply` / `SmithApplyInput` | same | live USE/DROP/craft/drop-near-forge intents (AI-JOB-SMITH-LIVE) |
| `smith_action_short_craft_apply` / `check_hungry_work_cost_by_id` / `check_hungry_work_cost_lookup` / `HungryWorkCostLookup` / `FLOOR_PLACE_ACTOR_IDS` | same | ShortCraft → shared short_craft_apply; food gate + floor 96/470/881 + container drop |
| `craft_and_drop_near_forge_apply` / `short_craft_on_ground_apply` | same | GetCraftAndDropItemsCloseToObj + shortCraftOnGround pure edges |

---

## Rust: shortCraft live intent (CRAFT-LIVE-IO)

| Symbol | File | Role |
|--------|------|------|
| `ShortCraftLiveIntent` / `ShortCraftIntentCtx` / `ShortCraftLiveApplyResult` | `ol-sim/src/short_craft_intent.rs` | tile-aware USE/DROP/seek staging from pure apply |
| `short_craft_apply_to_live_intent` / `short_craft_to_live_intent` | same | ShortCraftApply (+ resolved weak skewer) → UseAt/DropAt/Seek |
| `smith_apply_to_live_intent` | same | SmithApply → UseAt/DropNearForge/UseOnEmptyGround/GotoForge |
| `pick_ground_use_tile` / `short_craft_on_ground_to_live_intent` | same | Basket of Soil 336 well→home→any empty |
| `apply_short_craft_live_intent` / `short_craft_intent_use` | same | wire UseAt/DropAt via `apply_use_at` / `apply_drop` (resolve SeekOrCraft first via get_or_craft) |
| `is_floor_place_actor` | same | floor specials 96/470/881 |


## Rust: craftItem multi-step (AI-CRAFT-MULTI)

| Symbol | File | Role |
|--------|------|------|
| `craft_item` / `craft_item_helper` / `CraftItemDecision` / `ItemToCraftState` | `ol-sim/src/craft_item.rs` (via `get_or_craft::craft_item`) | pure AiBase.craftItem / craftItemHelper multi-step |
| `craft_item_helper_ex` / `craft_item_with_runtime_scan` | same | multi-step + `CraftScanFilters` path-reach (**AI-CRAFT-LIVE-RESID**) |
| `craft_obj_in_dual_center` / `craft_have_set_ex` / `closest_craft_obj_dual_center` | `ol-sim/src/craft_dual_center.inc.rs` | dual home/player craft scan (**AI-CRAFT-DUAL**) |
| `craft_have_set_ex_filtered` / `closest_craft_obj_dual_center_filtered` | same | dual-center + scan filters (**AI-CRAFT-LIVE-RESID**) |
| `reanchor_craft_actor_near_target` / `_filtered` / `CraftActorReanchor` | same | pile*1.5 + r=6 near-target re-anchor (+ path filters) |
| `ACTOR_NEAR_TARGET_R` / `PILE_VS_LOOSE_QUAD_FACTOR` | same | Haxe r=6 / *1.5 |
| `search_best_object_for_crafting` / `_ex` / `CraftTransPair` | same | top-down filtered reverse-graph pair (radius expand); `_ex` takes `CraftTopDownOpts` |
| `PlayerCraftAi` / `Player.craft_ai` | `ol-sim/src/craft_ai_sticky.rs` + `player.rs` | sticky itemToCraft + failedCraftings + itemToCraftId + craftingTasks + itemToCraftName (AI-CRAFT-STICKY) |
| `wipe_on_birth` / `prepare_for_product` / `add_task` / `do_make_craft_command` | same | newBorn clear + interrupt re-queue + MAKE order |
| `note_successful_use` / `NoteUseOutcome` / `select_sticky_craft_for_tick` | same | countDone after USE; sticky continue vs craftingTasks shift |
| `sticky_craft_sensor_flags` / `apply_sticky_flags_to_craft_sensors` | same | CraftQueue ladder sensors from sticky state |
| `craft_item_with_player_craft_ai` / `expand_craft_item_player_sticky` / `_scan` / `resolve_seek_or_craft_player_sticky` | same | sticky multi-tick craft expand (+ path filters) |
| `Player::wipe_craft_on_birth` / `craft_ai_begin_tick` | `player.rs` | birth wipe (revive path) + per-tick calledCraftItem guard |
| `apply_sticky_craft_queue_tick` | `profession_scan.rs` | live CraftQueue → expand sticky + path-reach scan + apply USE/DROP |
| `FailedCraftings` / `CraftAiRuntime` / `CraftLiveExpandOpts` | same | failedCraftings 15s + sticky multi-tick state |
| `retarget_water_source` / soil / bowl-fill / forge-bias specials | same | craftItemHelper pure specials |
| `craft_item_with_runtime` / `expand_craft_item_live_opts` / `_opts_scan` / `_sticky` / `_sticky_scan` | same + get_or_craft | sticky expand + home/smith/now + scan |
| `resolve_seek_or_craft_live_ex` | get_or_craft | CraftItem expand with opts + optional runtime |
| `resolve_seek_or_craft_live_scan` / `resolve_seek_or_craft_live_ex_scan` | same | multi-step + `CraftScanFilters` + is_moving Wait (**AI-CRAFT-NPC-ENQUEUE**) |
| `npc_enqueue_get_or_craft` / `npc_enqueue_get_or_craft_ex` | same | NPC pure helper: SeekOrCraft/CraftItem → wire USE/DROP (+ sticky runtime); `_ex` takes pile_id_for + full_pile_tiles |
| `get_pile_obj_id` / `get_pile_obj_id_from_map` / `pile_obj_id_from_content` | `get_or_craft.rs` | Haxe ObjectData.getPileObjId pure + ContentDb wire |
| `full_pile_tiles_from_scan` | `profession_scan.rs` | ScanTile.is_full_uses → CraftScanFilters.with_full_piles |
| `get_or_craft_objs_from_scan` | same | ScanTile → GetOrCraftWorldObj (prefers tile.num_slots) |
| `craft_item_decision_to_live_intent` / `resolve_craft_item_live` / `expand_craft_item_live` | same + get_or_craft | USE/DROP/SeekOrCraft from multi-step |
| `FORGE_IDS` / smith gate / TIME WaitTime | same | forge 303/304/305 + TIME actor |
| `craft_item_max_needed` | same | craftItemMax count < max gate |
| npc_ai multi-step GetOrCraft enqueue | `ol-server/src/npc_ai.rs` | profession SeekOrCraft/CraftItem → `npc_enqueue_get_or_craft_ex` + pile/full_piles/peer blockedByAI + NetIntent USE/DROP/MOVE; `NpcProfessionState.craft_rt` |


## Rust: craft top-down filters (AI-CRAFT-TOPDOWN)

| Symbol | File | Role |
|--------|------|------|
| `do_transition_search_skip_reason` / `CraftTransMeta` | `ol-sim/src/craft_topdown.rs` (nested under craft_item) | pure DoTransitionSearch skip gates |
| `should_skip_transition_top_down` / `should_skip_craft_edge` | same | bool wrappers; edge uses optional meta |
| `search_best_object_for_crafting_topdown` | same | filtered reverse-graph craft pair |
| `search_best_object_for_crafting_ex` | `craft_item.rs` | public opts-forwarding wrapper |
| `closest_craft_obj_filtered` / `CraftScanFilters` | same | hostile/unreachable/full pile scan |
| `CraftObjectIndex` / `from_objs` / `with_ai_craft_limits` | same | count + closest uses + aiCraftMax/Min |
| `CraftTopDownOpts` / `with_meta_map` / `with_search_current` / `meta_for` | same | last/pile/hardened/scan/index/meta_by_edge + dual `search_current_position` (**AI-CRAFT-DUAL**) |
| `hardened_row_forces_hoe_soil_ignore` / HARDENED_ROW | same | dynamic hoe+soil ignore (848→850/857+1138) |
| `time_transition_exceeds_ai_ignore` / `auto_decay_time_base_seconds` | same | autoDecaySeconds (neg hours×3600) > 120 |
| `AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN` | same | AiIgnoreTimeTransitionsLongerThen=120 |
| `CraftWorldObj.max_uses` / `with_max_uses` | `craft_item.rs` | ObjectData.numUses for reverseUse/minUseFraction |

## Rust: content AI ignore patches (C-SS-AI-IGNORE)

| Symbol | File | Role |
|--------|------|------|
| `ContentDb.ai_should_ignore` | `ol-content` | primary (actor,target) craft-AI ignore side-table |
| `ContentDb.ai_should_ignore_last_use` | `ol-content` | last-use-only ignores (pond 141/142 LT) |
| `apply_default_ai_should_ignore_patches` / `_ex` | `ai_should_ignore_patches.inc.rs` | ServerSettings.PatchTransitions table + synthetics |
| `insert_synthetic_ai_ignore_product_bodies` | same | Haxe `new TransitionData` coals/calf/skewer/butter bodies |
| `mark_by_new_actor_with_decays_to` | same | broken steel 858/862 ignore + actor decaysToObj |
| `AiShouldIgnorePatchOpts` / `transition_ai_should_ignore` / `_ex` | same | oven/kiln allow + primary/last-use lookup |
| `ContentDb::transition_ai_should_ignore` / `_ex` | `ol-content` | resolve_base_id + side-table lookup |
| `ReverseCraftGraph::load_ai_should_ignore_from` / `ai_should_ignore_edge` | `ol-sim/craft_graph.rs` | seed + pathfinding/seek skip (primary only) |
| `build_reverse_craft_graph` / `_capped` | `ol-sim/lib.rs` | seeds graph from content + ignore table |
| `craft_trans_meta_map_from_content` | `craft_topdown.rs` | CraftTransMeta map; primary vs last-use-only |
| `craft_item_helper_with_meta` | `craft_item.rs` | craftItemHelper + optional meta_by_edge |

## Rust: GetOrCraftItem pure I/O (AI-CRAFT-GRAPH-IO)

| Symbol | File | Role |
|--------|------|------|
| `get_or_craft_item` / `get_or_craft_item_ex` / `get_item` / `GetOrCraftInput` / `GetOrCraftResult` / `GetOrCraftWorldObj` | `ol-sim/src/get_or_craft.rs` | pure AiBase.GetOrCraftItem / GetItem search + staging; `_ex` skips blocked tiles |
| `closest_obj_by_id` / `closest_obj_by_id_filtered` / `get_or_craft_chebyshev` | same | min/max Chebyshev closest (GetClosestObject* + path-reach) |
| `get_or_craft_result_to_live_intent` / `get_or_craft_to_live_intent` | same | dropIsAUse UseAt vs dropTarget DropAt + craft staging |
| `resolve_seek_or_craft_live` / `_ex` / `_scan` / `_ex_scan` / `npc_enqueue_get_or_craft` / `_ex` / `apply_resolved_seek_or_craft` / `expand_craft_item_live*` | same | SeekOrCraft/CraftItem → multi-step expand (+ home/smith/sticky/scan/pile/full) → apply USE/DROP; **AI-CRAFT-NPC-ENQUEUE** |
| `get_pile_obj_id` / `get_pile_obj_id_from_map` / `pile_obj_id_from_content` | same | Haxe getPileObjId pure + live ContentDb |
| `world_objs_from_ids` / `GET_OR_CRAFT_*` constants | same | snapshot builder + pile close r=5 / target r=10 / max 40 |
| `ReverseCraftGraph::seek_ingredient_for` / `find_path_to_product` | `ol-sim/src/craft_graph.rs` | craft miss leaf seek (craftItem first cut) |

## Rust: profession scan tick (CRAFT-LIVE-TICK)

| Symbol | File | Role |
|--------|------|------|
| `scan_world_radius` / `ScanTile` | `ol-sim/src/profession_scan.rs` | live World → tile snapshot (incl empty; resolve_base_id; num_slots/num_uses/contains_id/contained_count) |
| `farm_map_from_scan` / `smith_map_from_scan` / `bake_map_from_scan` | same | → FarmMapObj / MapObj / BakeMapObj |
| `closest_by_parent_id` / `closest_by_parent_id_ex` / `closest_by_parent_id_to_target` | same | getClosestObjectById + minDistance + target-relative |
| `closest_by_parent_contains` / `closest_by_parent_contains_ex` | same | **NPC-SCAN-RESID** searchContained (e.g. clay 126 in basket 292) |
| `ProfessionPathFilters` / `filter_scan_tiles_path` / `filter_scan_tiles_path_owned` / `closest_by_parent_id_path` / `target_reachable_for_tile` | same | **NPC-SCAN-RESID** isObjectNotReachable/hostile pure |
| `path_filters_from_player` / `apply_path_filters_to_tiles` | same | **PATH-REACH** live maps → ProfessionPathFilters |
| `held_contained_from_helper` / `held_contained_from_player` / `held_contains_clay` / `held_contains_clay_from_player` | same | held nest cargo + clay-in-basket flag |
| `get_or_craft_objs_from_scan` / `full_pile_tiles_from_scan` | same | ScanTile → GetOrCraftWorldObj (prefers tile.num_slots); full multi-use coords for ignoreFullPiles |
| `closest_empty_tile` / `closest_empty_tile_ex` / `ClosestEmptyOpts` | same | getClosestObjectById empty + not-floored + home clearance |
| `closest_well` / `empty_near_well` / `empty_near_well_ex` / `WELL_IDS` | same | well 663/662 + empty near well |
| `NEEDS_NOT_FLOORED_PLACE` / `DONT_DROP_CLOSE_HOME_IDS` / `DONT_DROP_CLOSE_HOME_MIN` | same | Haxe dropHeld empty filters |
| `has_carrot_seeds_from_scan` / `has_bean_seeds_from_scan` / `count_parent_ids_in_scan` | same | Haxe countSeeds / hasBeanSeeds |
| `build_intent_ctx` / `build_intent_ctx_ex` / forge·oven | same | ShortCraftIntentCtx from scan + held |
| `smith_peers_from_players` / `_ex` / `baker_peers_from_players` / `_ex` / `peer_count_for_kind` | same | live roster → countProfession (+wound/follow) |
| `peer_roster_flags_for_player` / `peer_roster_flags_pure` / `PeerRosterFlags` | same | isWounded + playerToFollow pure |
| `farm_peers_from_players` / `_ex` / `count_farm_peers_for_job` / `FarmPeerSnapshot` | same | farm countProfession (MaxAge-2/wound/food/follow) |
| `farm_profession_scan_tick` / `smith_profession_scan_tick` / `baker_profession_scan_tick` | same | decide → shortCraft → live intent |
| `pottery_profession_scan_tick` / `pottery_action_to_live_intent` / `pottery_counts_from_scan` / `gather_clay_input_from_scan` / `pottery_map_from_scan` | same | **NPC-SCAN-FULL** / **AI-POTTER-NEST** USE/DROP; kiln-home; deposit UseOnBasket; DropHeld maxDist |
| `ScanTile.contains_extra` / `contains_parent` any-slot / `with_contains_list` | same | Haxe ObjectHelper.contains all nested slots |
| `held_contains_clay` / `held_nest_contains_parent` / `held_contains_clay_from_player` | same | Haxe heldObject.contains([126]); ProfessionScanInput.held_contains_clay |
| `merge_scan_tiles` / `pottery_scan_tiles_from_world` / `CLAY_DEPOSIT_SEARCH_RADIUS` | same | home craft r=30 + player clay deposit r=80 |
| `potter_peers_from_players` / `_ex` / `POTTERY_SCAN_RADIUS` | same | countProfession('POTTER') + maxSearch 30 |
| `shepherd_profession_scan_tick` / `shepherd_action_to_live_intent` / `shepherd_peers_from_players_ex` | same | isSheepHerding → USE/DROP + peer filters |
| `profession_scan_tick` / `apply_profession_scan_tick` | same | unified farm/smith/baker/**pottery**/shepherd + sim USE/DROP apply |
| `ProfessionStickySnapshot` / `ProfessionJobSensorFlags` / `job_sensor_flags_from_sticky` | same | sticky (+pottery/shepherd) → AssignedJob/AgeRotated/CriticalCraft sensors |
| `apply_job_flags_to_live_input` | same | write job flags into `LiveSensorInput` |
| `age_rotated_to_scan_step` / `plan_assigned_job_steps` / `plan_age_rotated_steps` / `plan_critical_craft_steps` / `plan_profession_ladder_steps` | same | PriorityRung → ProfessionLadderStep[] (Pottery age jobByAge==3) |
| `ladder_profession_scan_tick` / `apply_profession_ladder_tick` / `apply_profession_scan_from_sensors` | same | ladder→scan→USE/DROP (NPC-CRAFT-LADDER / NPC-SCAN-FULL; pottery_rt) |
| `ProfessionScanKind` / `ProfessionScanInput` / `ProfessionScanTickResult` / `ProfessionLadderStep` | same | tick I/O types (Farm/Smith/Baker/**Pottery**/Shepherd/**FireFood**) |
| `BakeAction::DeferPottery` expand | same | baker mid → `pottery_profession_scan_tick` else stage DeferPottery |
| npc_ai profession scan wire | `ol-server/src/npc_ai.rs` | multi-prof sticky (farmer/smith + age-rotated forager/hunter) → ladder (shepherd_rt+pottery_rt) → NetIntent USE/DROP |
| npc_ai `NpcProfessionState.path_reach` | same | **PATH-REACH** local notReachable/hostile; filter scan; walk-fail mark_goto |
| npc_ai `npc_next_step_to` / `npc_mark_goto_path_fail` | same | **AI-ANIMAL-GOTO** animal footprints + dual-pass hostile vs not_reachable |

## Rust: PATH-REACH / not_reachable_maps

| Symbol | File | Role |
|--------|------|------|
| `AiPathReachMaps` / `add_not_reachable` / `add_hostile_path` / `cleanup` / `blocked_coords` / `blocks_target` | `ol-sim/src/ai_path_reach.rs` | Haxe notReachableObjects + objectsWithHostilePath |
| `add_blocked_by_ai` / `cleanup_blocked_by_ai` / `blocked_coords_from_live` | same | static blockedByAI |
| `DONT_BLOCK_BY_AI` / `BlockTargetClaim` / `try_add_target_blocked_by_ai` / `would_block_target_by_ai` | same | Haxe AddTargetBlockedByAi filters |
| `block_claim_number_of_uses` | same | instance uses for claims (no ObjectData.num_uses fallback) |
| `AiAgentBlockSource` / `HumanBlockClaim` / `add_agent_to_blocked_by_ai` / `calculate_blocked_by_ai` / `apply_calculate_blocked_by_ai` | same | pure CalculateBlockedByAi rebuild |
| `AiStickyBlockTargets` / `StickyBlockBodyRow` / `rebuild_blocked_by_ai_from_sticky` / `should_set_block_target_for_ai` | same | sticky claims + pure live rebuild; player_block → ai_block chain-stop |
| `rebuild_blocked_by_ai_live` / `note_ai_block_targets_from_live_intent` | `lib.rs` | tick wipe+rebuild (held wound≠hidden gate) + shortCraft note |
| `Player.ai_block_targets` | `player.rs` | sticky food/use/drop/block claims |
| `apply_use_at` player_block | `use_transition.rs` | human / smith hammer 441 `blockTargetForAi` after USE |
| `mark_not_reachable_on_player` / `mark_use_path_fail` / `mark_food_path_fail` / `mark_goto_path_fail` | same | USE/food/Goto fail marks (age / 30s / animal) |
| `mark_use_or_food_path_fail` / `apply_food_action_fail` / `settle_pending_food_use_fail` | same | food 30s vs age-gate; async settle |
| `pending_food_tile_still_actionable` / `mark_food_pickup_action_fail_on_maps` / `merge_path_reach_maps` | same | container settle gate; DROP/REMV 30s; dual-map merge pure |
| `sync_path_reach_bidirectional` / `merge_npc_path_reach_from_views` | `ai_path_reach.rs` / `lib.rs` | **PATH-REACH-MERGE** max both ways + tick absorb NPC marks |
| `preserve_view_path_reach_on_publish` / `publish_player_view` / `publish_all_player_views` | `ai_path_reach.rs` / `lib.rs` | **PATH-REACH-MERGE** keep unabsorbed NPC maps on publish (no clobber) |
| `PlayerSnapshot.ai_path_reach` / npc `pull_player_path_reach` / `push_npc_path_reach_to_views` | `player.rs` / `npc_ai.rs` | **PATH-REACH-MERGE** pull once per think (all arms) + push; **AI-TAKEOVER** push too |
| `mark_path_fail_after_use_live` | `lib.rs` | **AI-FOOD-FAIL-MARK** live AI USE fail → maps |
| `mark_path_fail_after_food_pickup_action_live` / `note_ai_food_remv_claim` | `lib.rs` | live DROP/REMV food fail 30s + edible REMV note |
| `food_action_fail_effects` / `apply_food_action_fail` / `is_food_action_fail_at` / `is_empty_hand_food_use_fail` | same | **AI-FOOD-FAIL-MARK** pure isPickingupFood USE fail 30s |
| `try_mark_food_action_fail_on_maps` / `mark_use_or_food_path_fail` | same | empty-hand sticky/edible → 30s else age-gate USE fail |
| `consider_animals_for_goto` / `blocked_by_animal_from_dual_pass` / `receding_goto_should_abort` | same | **AI-ANIMAL-GOTO** pure gates (gotoAdv/gotoObj) |
| `Player.ai_path_reach` | `player.rs` | sticky per-AI maps |
| `SimState.blocked_by_ai` | `lib.rs` | global AI target claims (rebuild each tick) |
| tick_vitals PATH-REACH + BLOCKED-BY-AI | `lib.rs` | personal path cleanup + `rebuild_blocked_by_ai_live` |
| `search_best_food_full` not_reachable / hostile | `search_best_food_live.inc.rs` | live food skip from maps |
| `apply_profession_scan_tick` / ladder fail USE | `profession_scan.rs` | filter + `mark_path_fail_after_use_live` (food 30s / age) |
| `apply_short_craft_live_intent` USE/DropAt fail | `short_craft_intent.rs` | USE food/age mark; DropAt held-unchanged → food 30s |
| NetIntent::Use !applied | `lib.rs` | note sticky + `mark_path_fail_after_use_live` |
| NetIntent::Drop fail / Raw REMV fail | `lib.rs` | AI food sticky → `mark_path_fail_after_food_pickup_action_live` |
| `settle_npc_pending_food_action` / `pending_food_container` | `npc_ai.rs` | next-tick settle; container REMV gate; sticky until settle |
| Tests | `ai_path_reach::*` / `path_reach_*` / `food_action_fail_*` / `mark_use_or_food_*` / `mark_path_fail_after_food_pickup_*` | pure + live food 30s |

## Rust: AI-ANIMAL-GOTO / animal_goto_marks

| Symbol | File | Role |
|--------|------|------|
| `is_deadly_animal_for_path` / `animal_moves_covers_tile` | `ol-sim/src/pathfind.rs` | Haxe isAnimalDeadlyForMe simplified + moves footprint |
| `collect_deadly_animal_blocked_tiles` / `_around` | same | CreateCollisionChunkHelper animal branch |
| `is_walkable_with_animals` / `next_step_consider_animals` | same | terrain + animal footprints |
| `GotoPathOutcome` / `goto_path_outcome` | same | dual-pass Goto animals-on then animals-off |
| `GOTO_COLLISION_RAD` | same | MapData.RAD (=16) chunk half-width |
| `npc_next_step_to` / `npc_mark_goto_path_fail` | `ol-server/src/npc_ai.rs` | live profession + food walk dual-pass mark (`did_not_reach_food`) |
| Tests | `pathfind::*` dual-pass / footprint / gate | corridor wolf → BlockedByAnimal; wall → NotReachable |

## Rust: AI-GOTO-FOOD / food_explore_goto

| Symbol | File | Role |
|--------|------|------|
| `goto_quad_distance` / `LastGotoObj` / `GotoObjPlan` / `plan_goto_obj` | `ol-sim/src/ai_path_reach.rs` | Haxe gotoObj receding + lastGoto bookkeeping |
| `StickyFoodTarget` / `resolve_sticky_food` / `sticky_food_still_valid` | same | Haxe foodTarget sticky until pickup/fail |
| `FoodGotoFailEffects` / `food_goto_fail_effects` / `apply_food_goto_fail` | same | clear sticky + didNotReachFood++ + resetTargets |
| `food_pickup_success_reset_did_not_reach` | same | didNotReachFood = 0 after pickup |
| `Player.ai_last_goto_obj_*` / `ai_did_not_reach_food` | `player.rs` | live sticky fields (Haxe lastGotoObj / didNotReachFood) |
| `NpcFoodGotoState` / sticky SeekFood / Explore animals | `ol-server/src/npc_ai.rs` | isPickingupFood lite + dual-pass + receding |
| takeover food walk | same | same dual-pass + sticky as NPC |
| selfplay SeekFood | `ol-server/src/selfplay.rs` | `next_step_consider_animals` when Goal::SeekFood |
| Tests | `ai_path_reach::*` plan_goto_obj / sticky_food_resolve | receding abort; fail effects clear |
| selfplay job sensors | `ol-server/src/selfplay.rs` | `apply_job_flags_to_live_input` from Profession role |
| `do_pottery_on_fire` / `PotteryOnFireCounts` / `fill_pottery_on_fire_counts_from_map` / `resolve_smith_defer_pottery` | same | prepare fallthrough pottery body (bowl/plate/crock/adobe + L2946 residual) |
| `smith_action_to_live_intent` DeferPottery expand | `profession_scan.rs` | **AI-POTTER-L2946**: fill pottery from scan map; `smith_action_apply` expands CraftItem/ShortCraft |
| `SmithJobSlot` / `decide_smith_job_for_slot` / `smith_job_slot_priority` | same | early/critical/assigned(100)/mid/low/elder slots |
| `try_decide_smith_from_rung` / `smith_slot_for_rung` / `smith_job_rung_label` | same | ladder bridge; `EARLY_STICKY_SMITH` → EarlySticky |
| `smith_goal_from_map_and_rung` / `smith_goal_from_counts_and_rung` / `smith_action_to_goal` | same | fill → decide → Goal compose |
| `pick_smith_profession_goal` / `infer_smith_stage_from_have` / `smith_pipeline_targets` | same | stage-aware seek + reverse-craft |
| `pick_goal_smith_craft` / `pick_goal_smith_craft_at_stage` | `ol-sim/src/ai_goals.rs` | thin craft-aware goal (stage param) |
| selfplay smith plan | `ol-server/src/selfplay.rs` | `infer_smith_stage_from_have` + `pick_smith_profession_goal` |
| build wire | `ol-sim/build_ai_job_smith.rs` | lib.rs mod+export at cargo build |

---

## Rust: world persist / NestedHelper (NESTED-OLW1)

| Symbol | File | Role |
|--------|------|------|
| `NestedHelper` | `ol-world/src/lib.rs` | recursive contained meta (Haxe ObjectHelper under containedObjects) |
| `ComplexObject.slots` / `rebuild_wire_from_slots` / `synthesize_slots_from_wire` | same | OLW3 slots ↔ wire contained/nested |
| `transform_to_dummy` | same | Haxe ObjectHelper.TransformToDummy |
| `write_world` / `read_world` / `WORLD_FORMAT_VERSION=3` | `ol-world/src/persist.rs` | OLW1 magic; save v3; load v1–v3 |
| `write_nested_helper` / `read_nested_helper` | same | Haxe WriteToFile / ReadFromFile recursive (**pub**) |
| `write_optional_nested_helper` / `read_optional_nested_helper` / `NESTED_NULL_ID` | same | null body helpers (`-100`) |
| `save_world_file` / `load_world_file` / `rotate_world_backups` | same | disk I/O + `.bak.N` |
| `init_object_helpers_after_read` / `apply_helper_postload` | `ol-world/src/postload_owners.rs` | pure InitObjectHelpersAfterRead (+owned gate, grave no-prune, removeOwner) |
| `apply_init_object_helpers_after_read` | `ol-sim/src/postload_wire.rs` | sim boot: graves + owning + lineage.owns_object |
| `rebuild_player_owning_from_world` / `rebuild_account_graves_from_world` | same | spawn re-scan / account-only graves refresh |
| `account_token_index` / `description_is_orig_grave` / `player_status_for_postload` | same | soul-token map + origGrave + Alive/Deleted/Missing/Keep |
| `LineageNode.owns_object` | `ol-sim/src/social.rs` | Haxe `Lineage.ownsObject` (session; set by postload) |
| `container_put` / `container_take` / `container_take_helper` / `*_nested` | `ol-world/src/lib.rs` | runtime nest; take preserves NestedHelper tree |
| `encode_map_object_string_nested` / `parse_map_object_string` | same | wire `base,c:sub` one level |


## Rust: clothing transitions (TH-CLOTHING-MATRIX)

| Symbol | File | Role |
|--------|------|------|
| `get_clothing_slot_index` / `is_clothing_string` | `ol-sim/src/clothing_transitions.rs` | Haxe ObjectData.getClothingSlot / isClothing |
| `allow_reset_uses_on_target` | same | Haxe resetNumberOfUses clothing rule |
| `resolve_switch_slot` / `ClothingSlotIds` | same | dual shoe + type match (doSwitchCloths) |
| `try_transition_on_clothing_pure` / `_with_content` | same | tryTranstionOnClothing multi-use + TransformToDummy live |
| `empty_hand_container_take_index` / `sremv_resolved_index` | same | DoContainerStuff empty→first; SREMV −1→last |
| `take_from_clothing_nest_checked` / `refuse_take_permanent_contained` | same | permanent-contained refuse |
| `put_into_clothing_nest` / `take_from_clothing_nest` | same | DoContainerStuffOnObj on worn clothing |
| `can_put_into_clothing` / `can_put_into_clothing_sized` | same | nest put gate + **containSize/slotSize** |
| `try_drink_water_pure` / `apply_drink_self` | same | doSelf drink water bowl/pouch before clothing |
| `apply_switch_cloths` / `apply_switch_cloths_on_other` | same | self equip + UBABY age-gated other cloth |
| `apply_place_obj_in_clothing` / `apply_sremv_from_clothing*` | same | place/nest + SREMV permanent check + size gate |
| `apply_self_clothing` / `SelfClothingPath` | same | doSelf: drink → trans → switch → place |
| `format_clothing_set` / `format_clothing_helper_string` / `crown_say_line` | same | clothing_set + colon sub-nest + king/mask say |
| `format_player_update_line_full_clothing` | `ol-protocol` | PU with live clothing_set field |
| DROP c / SELF / SREMV / UBABY wire | `ol-sim/src/lib.rs` | drop clothingIndex; doSelf; specialRemove; doOnOther cloth |
| Tests | `clothing_transitions::*` / clothing_cmds | slot matrix, dual shoe, nest, drink, UBABY age, empty-hand first, size gate |

## Rust: containSize / slotSize (CLOTHING-CONTAIN-SIZE)

| Symbol | File | Role |
|--------|------|------|
| `ObjectDef.contain_size` / `slot_size` | `ol-content` | Haxe containSize / slotSize (defaults 0 / 1) |
| `ObjectDef::contain_fits_in_container` | same | pure size gate on defs |
| text `containSize=` / `slotsSize=` / `slotSize=` | `ol-content` parse | object-file load |
| OLC1 v8 trailer | `ol-binary/olc1.rs` | f32 contain_size + f32 slot_size |
| `apply_default_contain_size_patches` | `ol-content/lib_tail.inc.rs` | ServerSettings.PatchObjectData containSize/containable (desc + id table) |
| `contain_fits_slot` / `contain_slot_sizes` / `object_contain_fits_container` | `ol-sim/place_object.rs` (via death_polish) | pure put-in size helpers |
| `transition_result_fits_container` / `_from_content` | same | TH L1087 USE-on-container post-transition fit |
| `can_be_placed_in_grave` | same | grave swallow uses content sizes |
| DROP container put / PUTNEST | `ol-sim/lib.rs` | live size refuse |
| `apply_use_at_ex(..., container_index)` | `ol-sim/use_transition.rs` | USE x y id **i** retarget + L1087 gate + in-slot place |
| NetIntent::Use `index` | `ol-sim/lib.rs` | wires containerIndex into `apply_use_at_ex` |
| Tests | `parse_contain_size*` / `apply_default_contain_size*` / `clothing_contain_size*` / `contain_size_from_object_def*` / `use_on_container_*` / `transition_result_fits*` / `olc1_v8_*` | load + patches + gates |

## Rust: player body NestedHelper (NESTED-CLOTHING-PERSIST)

| Symbol | File | Role |
|--------|------|------|
| `Player.held_helper` / `clothing_helpers[6]` / `hidden_wound` / `fever` / `yellowfever_count` | `ol-sim/src/player.rs` | Haxe heldObject / clothingObjects / hiddenWound / fever |
| `Player::set_held_helper` / `set_clothing_helper` / `wear_held` / `strip_slot` | same | nest-preserving equip/strip |
| `Player::is_holding_hidden_wound` / `is_wounded_held` | same | light wound vs combat wound |
| `PlayerBodyObjects` / `write_player_body_objects` / `read_player_body_objects` | `ol-sim/src/nested_body.rs` | WritePlayers/ReadPlayers ObjectHelper slice |
| `player_set_held_object` / `apply_set_held_wound_rules` / `is_light_wound` | same | setHeldObject light-wound→hiddenWound |
| `alias_hidden_wound_to_held` | same | ReadPlayers L862 same-id alias |
| `apply_transform_to_dummy_on_helper` / `clear_if_timer_elapsed` | same | TransformToDummy + fever/wound timer clear |
| `tick_body_fever_and_hidden_wound` | `ol-sim/src/lib.rs` | COMBAT-FEVER-BLEED: clear fever/hiddenWound TTC + re-equip empty hands + survive GM |
| `player_take_container_into_hands` | same | REMV → held nest |
| REMV path | `ol-sim/src/lib.rs` | uses `container_take_helper` + nest into hands |
| Tests | `nested_body::*` / clothing_cmds / `container_take_helper` | round-trip, light wound, wear nest, container take |

## Rust: sticky players disk (PLAYERS-BIN / clothing_held_disk)

| Symbol | File | Role |
|--------|------|------|
| `PlayerDiskRecord` / `PlayersSnapshot` | `ol-sim/src/players_persist.rs` | PLB1 sticky roster row + file snapshot |
| `write_players` / `read_players` / `write_player_record` / `read_player_record` | same | pure codec (magic `PLB1`) |
| `save_players` / `load_players` | same | atomic tmp+rename; missing file → empty Ok |
| `capture_player_snapshot` / `apply_player_snapshot` | same | Player ↔ disk (body via NestedHelper) |
| `capture_players_snapshot` / `apply_players_snapshot` | same | roster + dual-pass refs + AI rehydrate |
| `get_player_id_for_write` / `get_player_from_id` / `resolve_player_ref` | same | Haxe L542–548 null sentinel `-100` |
| `apply_player_cross_refs` | same | mother/follow/held/attack/exile second pass |
| `PlayersShare` / `SimBootLive.players_share` | same + `settings_live.rs` | sim ↔ ol-server autosave mirror |
| `players_save_path` | `ol-config` | `players_v1.bin` under save dir |
| boot / autosave / shutdown | `ol-server/src/main.rs` | load PLB1; save with world/lineage/accounts |
| Tests | `players_persist::*` | multi-player nest/yum/exile, null refs, alias, second-pass held, missing file, atomic file |

## Rust: horse mount/dismount (TH-HORSE / HORSE-MOUNT / HORSE-MOUNT-POLISH)

| Symbol | File | Role |
|--------|------|------|
| `is_horse_mount_held` / `is_drugs` | `ol-sim/src/horse_mount.rs` | doHorseStuffPossible gate (770/778/3158; 837/838) |
| `is_horse_cart_held` / `is_hitched_cart` / `is_hitch_anchor` | same | hitch classifiers (778/3158; 779/3159; 4154/550) |
| `default_hitched_id_for_cart` / `default_cart_id_from_hitched` | same | 778↔779, 3158↔3159 defaults |
| `is_grave_basket_target` / `BASKET` / `HITCHED_*` | same | grave 87–89/357 + basket 292 ids |
| `is_horse_drop_trans` / `should_nest_swap_helpers` | same | isHorseDropTrans + isPickupOrDrop nest path |
| `basket_refuse_if_changing_held` | same | Haxe L1322–1326: 292+cargo refuses changeHeld |
| `pickup_or_drop_slots_ok` | same | "empty first" slot gate (newActor vs newTarget slots) |
| `put_down_ground_id` / `empty_ground_dismount_transition` | same | held+-1 / held+0 put-down & dismount lookup |
| `horse_eat_plan` / `try_horse_eat` | same + `use_transition.rs` | eat food while mounted (keep horse held) |
| `complex_to_nested` / `nested_to_complex` / `apply_ids_after_nest_swap` | same | tile↔held NestedHelper swap preserving cargo |
| `apply_use_at` horse + basket paths | `ol-sim/src/use_transition.rs` | nest swap; basket refuse gate; held+-1; slots refuse |
| `apply_drop` put-down transform | `ol-sim/src/lib.rs` | 770→1421, 778→1422, 3158→3161 + nest preserve |
| `Transition.is_pickup_or_drop` | `ol-content` Transition | Haxe TransitionData.isPickupOrDrop |
| `apply_default_horse_transition_patches` | `ol-content` lib_tail | PatchTransitions horse carts/tire/hitch/timers/graves |
| lib re-exports | `ol-sim/src/lib.rs` | hitch helpers + HITCHED_* / BASKET constants |
| Tests | `horse_mount::*` / `live_hitch_*` / `live_grave_*` / `live_basket_*` / `use_transition::horse_*` | hitch/unhitch cargo, grave basket, basket refuse, tire hitch, pickup/drop, eat |

## Rust: key / lock / lockpick (TH-LOCK / LOCKPICK / lock_flow)

| Symbol | File | Role |
|--------|------|------|
| `key_match` / `random_key_id` | `ol-sim/src/locks.rs` | Key 917 ↔ Locked* `externId` pair / assign / mismatch |
| `try_lockpick` / `LockpickSettings` / `lockpick_settings_for_player` | same | Haxe LockPick (success 5% / fail 10% / exh 3 / coin 1; female ×0.5 exh, ×0.8 fail) |
| `lockpick_coins_to_wallet_i32` | same | Haxe Float coins → i32 wallet floor (fractional live coin_cost) |
| `evaluate_lock_use_gate` / `LockUseGate` | same | USE pre-gate bundle (917/1003/904·4058/912·1000) |
| `description_is_locked` / `is_blank_lock_target` / `is_lock_and_key_held` | same | description `Locked` + object id helpers |
| `owner_may_open_empty_hand` / `owner_account_of` | same | empty-hand owner open via 917 transition |
| `note_lock_say` / `take_lock_say` | same | private PS from key/lockpick feedback |
| `LockState` | same | **session** tile HashSet (SAY LOCK/UNLOCK) — orthogonal to object keys |
| `Player.exhaustion` | `ol-sim/src/player.rs` | lockpick / jump / combat / heal accumulator |
| `calculateNotReducedFoodStoreMax` / `calculateFoodStoreMax` | `server/GlobalPlayerInstance.hx` | grown-up base × health × age − hits − exhaustion half-floor |
| `CalculateHealthFoodStoreMaxFactor` / `CalculateHealthFactor` | same | yum vs median prestige health factor |
| `DoDamage` hits/exhaustion/food_store_max | same | hits+=dmg (real), exhaustion+=dmg always, recompute max; combat death max&lt;0 |
| `killHelper` CombatExhaustionCostPerAttack | same | attacker exhaustion += 0.1 |
| `TimeHelper.updateFoodAndDoHealing` | `server/TimeHelper.hx` | bleed→hits + 2×food; exhaustion/hits heal; recompute max; death max&lt;−0.1 |
| Rust `calculate_food_store_max` / `apply_damage_food_pipe` / `step_healing_food_pipe` | `ol-sim/src/food_store_max.rs` | **EXHAUSTION-WOUND** pure |
| Rust `CombatState::resolve_hit_full` / `cap_damage_default` | `ol-sim/src/combat.rs` | not-reduced cap + hits/exhaustion death |
| Rust HIT / animal path food_max recompute | `ol-sim/src/lib.rs` | wire DoDamage pipes + attacker combat exhaustion |
| Rust `tick_vitals` heal + food_max death | same | doHealing gates + DeathWithFoodStoreMax |
| Tests | `food_store_max::*` / `combat::resolve_hit_full_*` / `cap_damage_uses_not_reduced_base` | pure + combat |
| `apply_use_at` lock gates | `ol-sim/src/use_transition.rs` | pre-transition key match / lockpick / blank copy / claim; owner open inject; wallet floor writeback |
| `maybe_lock_say_feedback` | `ol-sim/src/lib.rs` | PS after USE for lock say |
| Haxe anchors | `server/TransitionHelper.hx` L214–258, L418–455, L997–1025; `ServerSettings` Lockpick* | |
| Tests | `locks::*` / `use_transition::lock_removal_1003_*` / `settings_live::apply_live_settings_lockpick_*` / ol-config `live_diff_keys_all_four_lockpick_only` | pure + live knobs USE success/fail/fractional + female half exh |

## Rust: long-term decay (TIME-LONG)

| Symbol | File | Role |
|--------|------|------|
| `do_world_long_term_time_stuff` | `RustServer/crates/ol-sim/src/long_term.rs` | DoWorldLongTermTimeStuff (tick-wired) |
| `floor_decay_chance` / `object_decay_chance` | same | DecayFloor / DecayObject pure chance |
| `resolve_object_decay_to` / `floor_decay_result` | same | decay products (content + trash 618) |

## Rust: contained timers (CONTAINED-TIMERS-PERSIST / rearm_after_load)

| Symbol | File | Role |
|--------|------|------|
| `clamp_creation_to_sim_time` | `ol-sim/src/contained_timers_persist.rs` | Haxe `ObjectHelper.ReadFromFile` L268 clamp |
| `timer_from_nested_slot` / `timers_from_helper_for_rearm` | same | NestedHelper → (creation, ttc); fresh seed when missing |
| `uses_from_nested_slot` / `uses_from_helper_for_rearm` | same | NestedHelper.uses_remaining → last-use path |
| `rebuild_contained_timers_from_world` | same | scan helpers → runtime timer map after OLW (first level) |
| `apply_contained_timers_to_slots` / `apply_contained_uses_to_slots` | same | stamp NestedHelper for OLW3 save (map-slice) |
| `rearm_stats` / `ContainedTimerRearmStats` | same | tiles/slots/persisted_ttc counts |
| `arm_contained_timers_for_loaded_world` | `postload_wire.rs` / `lib.rs` | fill `WorldMapTimeState.contained_timers` after load |
| `do_time_for_contained` | `world_time.rs` | Haxe `doTimeForObject` pure |
| `tick_nested_helpers_deep` / `tick_container_helper_timers` / `NESTED_TIMER_MAX_DEPTH` | `ol-sim/src/nested_timers.rs` | Haxe L1150 nested-in-nested; recursive NestedHelper timers |
| map-slice contained | `world_time.rs` `do_world_map_time_stuff` | **`nested_timers::tick_container_helper_timers`** (first-level + deep); MX on id/structure change |
| Tests first-level | `contained_timers_persist::*` / `world_time::contained_timer_*` / `rearm_contained_*` | clamp, mid-ttc, last-use |
| Tests deep | `nested_timers::*` / `world_time::nested_in_nested_*` / `first_level_*` / `rearm_then_deep_*` | deep transform, mid-ttc, overflow refuse, cargo keep, call-site wire |

Haxe anchors: `TimeHelper.doTimeForObject`, `ObjectHelper.creationTimeInTicks` / `timeToChange` / `numberOfUses` on contained; ReadFromFile clamp; L1150 nested-in-nested (**DONE** map-slice).

## Rust: animal move / chase / pop (`TIME-ANIMAL` / `TIME-ANIMAL-CHASE` / `TIME-ANIMAL-OFFSPRING`)

| Symbol | File | Role |
|--------|------|------|
| `tick_animals` / `tick_animals_dt` / `tick_animals_dt_full` | `ol-sim/src/lib.rs` | doAnimalMovement cadence + chase + **pop die/offspring/failedMoves** |
| `apply_animal_pop_map_events` | same | MX for natural/failedMoves death + offspring birth (after move MX so origin clear) |
| `AnimalWorld::tick_wander_timed_ex` | `animals.rs` | timer/hits decay; legacy no-pop path (`apply_pop=false`) |
| `AnimalWorld::tick_movement_with_pop` | same | full pop/die/failedMoves + moves/births/deaths |
| `Animal.failed_moves` / `loved_tx` / `loved_ty` / `target` | same | Haxe ObjectHelper fields |
| `AnimalWorld::original_counts` / `capture_original_counts` | same | Haxe originalObjectsCount baseline (seed spawn) |
| `resolve_animal_chase` | `animal_move.rs` | Winter/SNOW bone grave; deadly player lock; pack alert index |
| `deadly_chase_gate` | same | season/hits/`animalsDontChase`/`chasingAnimals` |
| `get_closest_player_at` | same | Haxe GetClosestPlayerAt (Euclidean quad-dist) |
| `get_closest_bone_grave` / `is_bone_grave` / `collect_bone_graves_near` | same | GetClosestBoneGrave / IsBoneGrave |
| `clear_cursed_graves` / `clear_ovens_index` / `clear_map_index_keep` | `world_time.rs` | TimeHelper.ClearCursedGraves + ovens prune |
| `maybe_insert_cursed_grave` / `maybe_insert_oven` / `map_linear_index` / `index_positions` | same | DoWorldMapTimeStuff index fill |
| `WorldMapTimeState.cursed_graves` / `.ovens` | same | Haxe WorldMap.cursedGraves / ovens |
| `is_oven_map_id` / `CURSED_GRAVE_TIME_HOURS` / `CURSED_GRAVE_SHARP_STONE_EXTRA_SECS` | same | IsOven + CursedGraveTime*3600 (12h) |
| `should_clear_cursed_graves_ovens` / `CURSED_GRAVES_CLEAR_TICK_MOD` | same | tick%2000 prune cadence |
| `is_spawning_in` | same | ObjectData.isSpawningIn (biomes + countsOrGrowsAs) |
| `pick_animal_destination_steered` | same | preferred-biome bias + gotoTarget/gotoLovedBiome best-quad |
| `calculate_non_blocked_target` / `can_animal_end_up_here` | same | path trim + land rules |
| `resolve_pop_on_dest` / `resolve_failed_move` | `animal_pop.rs` | natural die + offspring rolls; failedMoves>20 |
| `chance_for_offspring` / `chance_for_animal_dying` / gates | same | ServerSettings Chance* pure |
| `resolve_animal_path_damage` / escape helpers | `animal_damage.rs` | DoAnimalDamage / TryAnimaEscape (prior chunk) |
| `apply_animal_path_damages` / `apply_animal_zero_wound_hit` / residual | `lib.rs` | **WEAPON-ANIMAL-ZERO** wound equip + newActor transform after path hit |
| `Animal.map_object_id` / `apply_zero_residual` / `AnimalDeathEvent.object_id` | `animals.rs` | attacking form (1323→1333) + death map clear |
| `plan_animal_zero_wound_from_content` / residual TTC | `weapon_wound.rs` | pure animal+0 plan (no takeCoins) |
| `plan_animal_zero_residual_ex` / `plan_animal_zero_wound_from_content_ex` | `weapon_wound.rs` | **C-SS-MORE-BATCH5** live WeaponCoolDown* residual TTC |
| Tests | `animal_pop::*` / `animal_move::*` / `animal_damage::*` / `animals::*` / `weapon_wound::animal_zero_*` | pop gates, chase, path blocks, animal+0 residual |

## Rust: config hot-reload (`CONFIG-SETTINGS` / server_settings_hot_reload / SETTINGS-FIELD-MAP)

| Symbol | File | Role |
|--------|------|------|
| `ServerConfig::live_settings` | `RustServer/crates/ol-config` | runtime-safe knob snapshot |
| `ServerConfig::season_length_secs` | same | Haxe SeasonDuration years × 60 |
| `ServerConfig::live_settings_key_names` | same | all LiveSettings keys for coverage tests |
| `HotReloadTracker::new` / `poll` / `force_reload` | same | mtime + due-tick re-read of `server.toml` |
| `LiveSettings` | same | live field set (speed/move/season/npc/lockpick/gameplay…) |
| `field_map::CRITICAL_FIELD_MAP` / `SettingsHome` | `ol-config/src/field_map.rs` | Haxe static → Live/BootToml/ModuleConst/SecretOmit inventory |
| `DOOR_IDS` / `is_door_id` / `AI_IGNORED_FLOOR_IDS` | same | Haxe DoorIds / AiIgnoredFloorIds tables |
| `gameplay_defaults::*` | same | Haxe default values for gameplay batch |
| `apply_live_settings` / `enforce_eternal_winter` | `ol-sim/src/settings_live.rs` | apply onto `SimState` |
| `GameplayKnobs` / `GameplayKnobs::from_live` | same | FoodUse/Heal/Age/Move/Yum/Offspring/… on SimState |
| `haxe_next_season_duration_years` / `haxe_season_hardness` / `haxe_next_season_length_secs` | same | Haxe DoSeason re-seed pure helpers |
| `reseed_season_length_after_roll` / `is_hard_season` | same | post-roll length from `season_duration_base_secs` |
| `SimState::season_duration_base_secs` / `SimState::gameplay` | `ol-sim/src/lib.rs` | SeasonDuration base + live gameplay knobs |
| `step_healing_food_pipe_ex` / `age_step_from_health_ex` / `birth_yum_multiplier_ex` | `food_store_max.rs` | live Healing/Ageing/BirthPrestige |
| `resolve_pop_on_dest_ex` / `compute_*_chance_ex` | `animal_pop.rs` | live ChanceForOffspring/Dying |
| `intent_budget_from_live` | `settings_live.rs` | intent drain from live knobs |
| `SimBootLive` | same | boot package for `run_sim_loop_with_views` |
| `NpcConfig::from_live` | `ol-server/src/npc_ai.rs` | LiveSettings → NPC knobs each ~200 ms wake |
| `run_npc_scheduler(live_share, …)` | same | same-wake hot-reload (no 2 s copy task) |
| build wire | `ol-sim/build_settings_live.rs` | lib.rs + main + npc_ai at cargo build |
| Tests | `ol-config` `force_reload_reports_all_live_keys_*` / `write_default_load_roundtrip_*` / `field_map::*` / `settings_live::apply_live_settings_gameplay_*` | inventory + live gameplay |

## Rust: EMPTY-BOWL / cold_bowl_edge (`TH-MULTI` residual)

| Symbol | File | Role |
|--------|------|------|
| `change_tool_transitions` | `ol-content` `lib_tail.inc.rs` | Haxe `TransitionImporter.changeToolTransitions` — rewrite same-actor `newActor` via `(newActor,-1)` LA/non-LA; skips EMPTY+Cold Bowl / multi-use / 2170 / newActor=0 |
| `pick_tool_last_use_new_actor` | `ol-sim/src/multi_use.rs` | Pure LA→non-LA→target-LA new_actor pick |
| `eat_actor_after_use` / `EatActorUsesOutcome` | same | Haxe eat-path `DoChangeNumberOfUsesOnActorManual` |
| `tool_last_use_new_actor` | `ol-sim/src/use_transition.rs` | ContentDb wrapper for tool last-use chain |
| `resolve_actor_after_use` | same | USE actor uses + tool last-use; **keeps uses=0** after id transform |
| `try_eat_held` | `ol-sim/src/lib.rs` | Eat food then Manual uses (stew 1251→235); clear when no tool row |
| `empty_ground_dismount_transition` | `horse_mount.rs` | Rejects clay TIME `newTargetID==0` (regression) |
| Tests | `multi_use` / `use_transition` / `ol-content` | `eat_bowl_of_stew_*`; `try_eat_stew_*`; `change_tool_transitions_water_bowl_not_cold_bowl` |

## Rust: ScoreEntry prestige queue (S-SCORE / SCORE-ENTRY / score_disk)

| Symbol | File | Role |
|--------|------|------|
| `AccountScoreEntry` / `AccountRecord.score_entries` | `ol-sim/src/accounts.rs` | Haxe `ScoreEntry` queue on account |
| `create_score_entry_if_grave` | `ol-sim/src/score_entry.rs` | Old Grave (89) unburied mali pure |
| `create_score_entry_for_cursed_grave` | same | sharp-stone curse stacks mali pure |
| `create_score_entry_for_dead_relative` / `create_new_score_entry` | same | mother-line ancestor award pure |
| `creator_grave_is_non_bone` | same | Haxe `Lineage.get_grave` creator match |
| `process_score_entry` / `should_process_score_entry` | same | ProcessScoreEntry body + trueAge%5 |
| `should_process_score_entry_on_year_cross` | same | Haxe age-year boundary + %5 gate |
| `format_global_message_text` | same | UPPER + spaces→`_` for GM |
| `save_score_entries` / `load_score_entries` | same | SES1 disk (Haxe TODO implemented) |
| `AccountBook::push_score_entry` / `process_score_entry_for` | same | account queue helpers |
| `push_old_grave_score_entry` / `push_cursed_grave_score_entry` | `ol-sim/src/lib.rs` | `tick_auto_decays` live wire |
| `process_player_score_entries` | same | vitals year-cross → process + GM + no-clamp prestige |
| `apply_dead_relative_score_entry` | `ol-sim/src/death_polish.rs` | death → dead-relative entry |
| `ServerConfig::score_entries_save_path` | `ol-config` | `score_entries_v1.bin` |
| ol-server boot/autosave/shutdown | `ol-server/src/main.rs` | SES1 load after OLA1; save with accounts |
| Tests | `score_entry::*` | create/process/SES1/year-cross/creator-grave |

## Rust: session war / posse persist (`SOCIAL-WAR-PERSIST` / war_posse_disk)

| Symbol | File | Role |
|--------|------|------|
| `WarState` / `declare_war` / `make_peace` / `make_alliance` | `ol-sim/src/war.rs` | undirected pair → Peace/War/Alliance; `pair_key` normalize |
| `WarState::prune_player` / `prune_absent` | same | death / bulk orphan sweep |
| `PosseState` / `add_posse` / `clear` | `ol-sim/src/posse.rs` | killer → target set; PJ wire |
| `PosseState::prune_player` / `prune_absent` | same | death cleanup (as killer + as target) |
| `WarPosseSnapshot` / `counts` | `ol-sim/src/war_posse_persist.rs` | combined session snapshot |
| `save_war_posse` / `load_war_posse` | same | WPS1 atomic tmp→rename; missing file → empty Ok |
| `capture_war_posse_snapshot` / `apply_war_posse_snapshot` | same | live ↔ snapshot |
| `prune_war_posse_for_player` / `prune_war_posse_absent` | same | pure death / bulk prune |
| `WarPosseShare` / `SimBootLive.war_posse_share` | `settings_live.rs` | Arc share for boot seed + autosave |
| `mirror_war_posse_share` | `ol-sim/src/lib.rs` | sim → share (periodic + dirty tick + disconnect) |
| `SimState.war_posse_dirty` | same | SAY WAR/POSSE/PEACE + death prune → flush next tick |
| `apply_death_inheritance` → prune | same | death drops war/posse edges (not disconnect) |
| `ServerConfig::war_posse_save_path` | `ol-config` | `save_directory/war_posse_v1.bin` |
| ol-server boot/autosave/shutdown | `ol-server/src/main.rs` | WPS1 load/save |
| SAY WAR / PEACE / POSSE / ?WAR / ?POSSE | `lib.rs` apply_intent | live mutate + WR/PJ nearby |
| Tests | `war_posse_persist::*` / `war::*` / `posse::*` / `death_prunes_war_and_posse_edges` | WPS1 roundtrip, prune, death wire |

Haxe: no server WAR/POSSE disk (`WR`/`PJ` protocol client tags only). Residual: session `p_id` keys; sticky identity → Players.bin.

## Rust: ally combat strength (ALLY-STRENGTH / ally_combat)

| Symbol | File | Role |
|--------|------|------|
| `GlobalPlayerInstance.calculateEnemyVsAllyStrengthFactor` | `server/GlobalPlayerInstance.hx` | close friendly vs enemy strength ratio |
| `GlobalPlayerInstance.makeAllCloseAllyAngryAt` | same | set lastPlayerAttackedMe on close allies |
| `GlobalPlayerInstance.DoDamage` allyFactor | same | 0.5 if ally else strength factor cap 1.2 |
| `GlobalPlayerInstance.kill` unarmed ally gate | same | first-hit warn; second-hit exile then damage |
| `ServerSettings.AllyConsideredClose` / `AllyStrenghTooLowForPickup` | `settings/ServerSettings.hx` | radius 5; pickup gate default 0 |
| `TransitionHelper` AllyStrenghTooLowForPickup | `server/TransitionHelper.hx` | refuse non-empty target if factor too low |
| `calculate_enemy_vs_ally_strength_factor` | `ol-sim/src/combat.rs` | pure Haxe factor (base 10, weapon×2 food_max) |
| `resolve_ally_damage_factor` | same | ally → 0.5; else min(factor, 1.2) |
| `close_ally_ids_for_anger` / `combat_strength` / `is_close_for_ally_strength` | same | makeAllCloseAllyAngryAt ids + strength helper |
| `ally_strength_blocks_pickup` | same | pure TransitionHelper pickup gate (default off) |
| `resolve_unarmed_ally_hit_gate` / `unarmed_ally_first_hit_messages` | same | kill first-hit warn / second exile pure |
| `AllyStrengthPlayer` / `UnarmedAllyHitGate` | same | scan snapshot + gate enum |
| HIT wire allyFactor + anger + unarmed gate | `ol-sim/src/lib.rs` SAY HIT | **source-wired** `org_damage *= ally_factor`; anger on connect; first-hit ALLY_WARN; **exile-aware `is_ally`** for gate/strength/anger |
| USE pickup gate | `ol-sim/src/use_transition.rs` | threshold default 0 = off; say "Too many hostile people..." |
| Tests | `combat::ally_*` / `unarmed_ally_*` / `say_hit_ally_*` / multi-hop | pure factor + live HIT |

Residual: `AllyStrenghTooLowForPickup` not yet LiveSettings (const 0; USE path ready when >0). Ally prestige → **PRESTIGE-ALLY-COST**.

## Rust: ally prestige cost (PRESTIGE-ALLY-COST / ally_prestige_cost)

| Symbol | File | Role |
|--------|------|------|
| `is_ally` | `ol-sim/src/relations.rs` | Haxe `isAlly` (exile-aware `get_top_leader`) |
| `is_leadership_ally` | same | follow-graph only (no exile); misc non-HIT paths |
| `PrestigeCostFactors` / `compute_hit_reputation_with_factors` | `ol-sim/src/reputation.rs` | live/test category multipliers |
| `PRESTIGE_COST_PER_DAMAGE_ALLY` / `PrestigeCostCategory::Ally` | same | default 1 + GM phrase `"ally"` |
| `format_prestige_cost_global_message` | same | `Lost N prestige for attacking ally Name!` |
| `GameplayKnobs.prestige_cost_per_damage_for_ally` | `settings_live.rs` | LiveSettings → sim |
| `GameplayKnobs.prestige_cost_per_damage_for_child|elderly|close_relatives|women_without_weapon` | same | **C-SS-MORE** LiveSettings |
| `GameplayKnobs::prestige_cost_factors` | same | bundle all five → PrestigeCostFactors |
| `apply_connecting_hit_reputation` | `lib.rs` | recompute `is_ally` + live PrestigeCost* factors + score prestige + GM |
| HIT gate / strength / anger | `lib.rs` SAY HIT | exile-aware `is_ally` (not `is_leadership_ally`) |
| SAY KILL pre-flag | `lib.rs` | exile-aware `is_ally` (recomputed in apply_connecting) |
| Tests | `is_ally_breaks_*` / `prestige_cost_category_ally_*` / `prestige_cost_live_non_ally_factor_overrides_ceil` / `say_hit_peer_ally_*` / multi-hop | pure + live |

Residual: L4525 recent-exile-ally TODO (open both sides); full `addHealthAndPrestige` yum_multiplier / darkNosaj; CombatReputationRestorePerYear → **C-SS-TAIL-KNOBS DONE**.

## Rust: PrestigeCost non-ally Live (C-SS-MORE / settings_more)

| Symbol | File | Role |
|--------|------|------|
| `prestige_cost_per_damage_for_child` | `ol-config` ServerConfig/LiveSettings + `GameplayKnobs` | Haxe default **5** |
| `prestige_cost_per_damage_for_elderly` | same | Haxe default **1** |
| `prestige_cost_per_damage_for_close_relatives` | same | Haxe default **0.5** |
| `prestige_cost_per_damage_for_women_without_weapon` | same | Haxe default **0.5** |
| `gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_*` | `ol-config/field_map.rs` | Haxe ServerSettings defaults |
| FIELD_MAP PrestigeCostPerDamageForChild/Elderly/CloseRelatives/WomenWithoutWeapon | same | **SettingsHome::Live** |
| `GameplayKnobs::prestige_cost_factors` | `settings_live.rs` | live → PrestigeCostFactors |
| `apply_connecting_hit_reputation` | `lib.rs` | uses `state.gameplay.prestige_cost_factors()` |
| Tests | `prestige_cost_live_non_ally_factor_overrides_ceil` / `apply_live_settings_gameplay_knobs` / field_map live | pure + apply |

Residual: optional HIT integration that hot-reloads child factor mid-session (pure path covered); L4525 TODO.

## Rust: C-SS-TAIL-KNOBS / settings_knobs (five ServerSettings Live)

| Symbol | File | Role |
|--------|------|------|
| `grown_up_food_store_max` | `ol-config` + `GameplayKnobs` | Haxe **20** — capacity base/age bands |
| `min_biome_speed_factor` | same | Haxe **0.2** — ocean/mountain floor |
| `hitpoints_speed_factor` | same | Haxe **3** (0 = disable) |
| `food_reduction_faktor_for_eating_high_quality` | same | Haxe **0.8** (typo Quailit) |
| `combat_reputation_restore_per_year` | same | Haxe **2** — calm restore rate |
| `calculate_food_store_max_ex` / `food_store_max_from_parts_ex` | `food_store_max.rs` | live GrownUp (+ NewBorn/OldAge via knobs) |
| `step_healing_food_pipe_ex(..., FoodStoreMaxKnobs)` | same | vitals capacity |
| `effective_biome_speed_ex` / `vitals_speed_product_ex` | `move_speed.rs` | live MinBiome / Hitpoints / GrownUp |
| `reduce_food_value_chain_ex` | `yum.rs` | live high-quality factor |
| `combat_reputation_restore_delta_ex` | `reputation.rs` | live restore rate |
| `tick_combat_reputation_restore` | `lib.rs` | reads `state.gameplay.combat_reputation_restore_per_year` |
| FIELD_MAP five rows | `field_map.rs` | **SettingsHome::Live** |
| Tests | `apply_live_settings_gameplay_knobs` / pure restore/chain/food_max/speed / field_map live | |

Residual: `floor_road_biome_factor` still clamps with module `MIN_BIOME_SPEED_FACTOR`; move path `vitals_speed_product` default (not `_ex`) at some compose sites. darkNosaj restore gate → **WALLET-COINS DONE**. NewBorn/OldAge → **C-SS-AGE-FOOD**.

## Rust: C-SS-MORE-KNOBS / settings_batch2 (six ServerSettings Live + InheritEaten)

| Symbol | File | Role |
|--------|------|------|
| `exhaustion_healing_factor` | `ol-config` + `GameplayKnobs` | Haxe **1.5** — exhaustion heal drain + delta |
| `wound_damage_factor` | same | Haxe **1** — bleed DPS mult |
| `max_movement_quad_jump_distance_before_force` | same | Haxe **5** — timed MOVE jump gate (squared) |
| `food_restore_factor_while_feeding` | same | Haxe **10** — continuous nurse fill |
| `max_has_eaten_for_next_generation` | same | Haxe **4** — inherit clamp |
| `has_eaten_reduction_for_next_generation` | same | Haxe **1** — inherit subtract when count>0 |
| `step_healing_food_pipe_ex(..., exhaustion_healing_factor)` | `food_store_max.rs` | live ExhaustionHealingFactor |
| `wound_bleed_food_extras_ex` | same | live WoundDamageFactor |
| `breastfeed_tick_ex` | `feed.rs` | live FoodRestoreFactorWhileFeeding |
| `jump_exceeds_force_threshold` / `GameplayKnobs::max_move_quad_jump_before_force` | `move_path` / `settings_live` | live MaxMovementQuadJump |
| `inherit_eaten_food_count(s)` / `inherit_eaten_source_parent_id` | `yum.rs` | pure InheritEatenFoodCounts |
| `inherit_eaten_food_counts_for_birth` / `spawn_child` | `lib.rs` | birth wire |
| FIELD_MAP six rows | `field_map.rs` | **SettingsHome::Live** |
| Tests | `heal_exhaustion_live_factor_override` / `wound_bleed_live_factor_override` / `breastfeed_tick_live_factor_override` / pure inherit / `spawn_child_inherits_eaten_food_counts` / apply_live keys | |

Residual: father-line inherit needs lineage.father_id; full grandparent when not in `players` falls back to mother map. **WoundHealingFactor → C-SS-WOUND-HEAL DONE**.

## Rust: C-SS-WOUND-HEAL / wound_healing (WoundHealingFactor Live)

| Symbol | File | Role |
|--------|------|------|
| `wound_healing_factor` | `ol-config` + `GameplayKnobs` | Haxe **1** — hits heal rate + heal food drain |
| `step_healing_food_pipe_ex(..., wound_healing_factor)` | `food_store_max.rs` | live WoundHealingFactor |
| `wound_heal_drain_rate_ex` | same | ?DRAIN estimate live factor |
| tick_vitals heal pipe | `lib.rs` | `state.gameplay.wound_healing_factor` |
| hits write-back | `lib.rs` | clear `wounded_by` when hits&lt;1 + `format_healed` HE |
| `format_healed` | `ol-protocol` | Haxe ClientTag.HEALED |
| FIELD_MAP WoundHealingFactor | `field_map.rs` | **SettingsHome::Live** |
| Tests | `heal_hits_live_wound_healing_factor_override` / apply_live keys / `healed_shape` | |

Residual: **ExhaustionHealingForMaleFaktor → C-SS-MALE-HEAL / C-SS-MORE-BATCH3 DONE**. **Temp/biome heal extras → C-SS-TEMP-HEAL DONE**.

## Rust: C-SS-MALE-HEAL / male_exhaustion (ExhaustionHealingForMaleFaktor Live)

| Symbol | File | Role |
|--------|------|------|
| `exhaustion_healing_for_male_factor` | `ol-config` + `GameplayKnobs` | Haxe **1.2** — male exhaustion recovery mult only |
| `step_healing_food_pipe_ex(..., exhaustion_healing_for_male_factor)` | `food_store_max.rs` | live male factor; female uses 1.0 |
| tick_vitals heal pipe | `lib.rs` | `state.gameplay.exhaustion_healing_for_male_factor` + skin is_male |
| FIELD_MAP ExhaustionHealingForMaleFaktor | `field_map.rs` | **SettingsHome::Live** |
| Tests | `heal_exhaustion_live_male_factor_override` / apply_live keys | |

Residual: is_male via `display_object_id` 0\|19 vs Haxe `ObjectData.male` (soft). **Temp/biome heal extras → C-SS-TEMP-HEAL DONE**.

## Rust: C-SS-TEMP-HEAL / temp_heal_extra (TemperatureHits/ExhaustionDamageFactor Live)

| Symbol | File | Role |
|--------|------|------|
| `temperature_hits_damage_factor` / `temperature_exhaustion_damage_factor` | `ol-config` + `GameplayKnobs` | Haxe **0.5** / **0.2** |
| `temperature_damage_extras` / `_ex` | `food_store_max.rs` | age>1 super-hot/cold → hits+exh (×2 at heat>0.95/<0.05) |
| `biome_love_exhaustion_heal` | same | love>0 → exh − healing×0.5×min(love,1); not doHealing-gated |
| `biome_love_factor` / `biome_love_factor_for_color` / `is_biome_loved_by_color` | same | Haxe biomeLoveFactor self+parents×0.5 |
| `TempBiomeHealKnobs` | same | pipe param bundle |
| `step_healing_food_pipe_ex(..., temp_biome)` | same | order: exh heal → temp → biome love → hits heal |
| tick_vitals | `lib.rs` | person-color `is_super_*_for_person` heal gate + biome/floor/parent love |
| ?DRAIN | `lib.rs` | same person-color super gate |
| FIELD_MAP TemperatureHits/ExhaustionDamageFactor | `field_map.rs` | **SettingsHome::Live** |
| Tests | `temperature_damage_extras_age_and_extremes` / `biome_love_exhaustion_heal_love_and_floor` / `biome_love_factor_brown_jungle_and_swamp_floor` / `step_healing_applies_temp_damage_and_biome_love` / apply_live keys | |

Residual: parent love only when parents living in `players`; mosquito loves_jungle → **COMBAT-MOSQUITO-KIND DONE** (`player_jungle_biome_love`); wrong-biome exh gain stays off (Haxe commented).

## Rust: C-SS-MORE-BATCH3 / settings_batch3 (six ServerSettings Live)

| Symbol | File | Role |
|--------|------|------|
| `exhaustion_healing_for_male_factor` | `ol-config` + `GameplayKnobs` | Haxe **1.2** (also C-SS-MALE-HEAL) |
| `combat_exhaustion_cost_per_attack` | same | Haxe **0.1** — HIT Wound/Kill attacker exhaustion |
| `min_age_to_eat` | same | Haxe **3** — eat/feed + prestige child threshold |
| `max_child_age_for_breast_feeding` | same | Haxe **6** — nurse continuous ≤ / pickup &lt; |
| `ally_considered_close` | same | Haxe **5** — ally strength + anger radius |
| `min_movement_age_in_sec` | same | Haxe **14** — MOVE TooYoung gate |
| `movement_age_allowed_ex` | `move_path.rs` | live MinMovementAgeInSec |
| `is_close_for_ally_strength_ex` / `calculate_enemy_vs_ally_strength_factor_ex` / `close_ally_ids_for_anger_ex` | `combat.rs` | live AllyConsideredClose |
| `can_nurse_age_ex` / `can_pickup_breastfeed_age_ex` / `can_breastfeed_ex` | `feed.rs` | live MaxChildAge ≤ vs &lt; |
| `feeder_may_eat_or_feed` + live min | `feed_other_yum` / `lib.rs` try_eat + feed_other | live MinAgeToEat |
| `PrestigeCostFactors.min_age_to_eat` | `reputation.rs` | live child prestige age |
| FIELD_MAP six rows | `field_map.rs` | **SettingsHome::Live** |
| Tests | `apply_live_settings_gameplay_knobs` + pure live boundary tests | |

Residual (closed by **C-SS-MIN-AGE-AI**): AI/profession/grave/map_pins → live; GPI clothing MinAge gate stays commented (Haxe).

## Rust: C-SS-MORE-BATCH4 / settings_batch4 (six ServerSettings Live)

| Symbol | File | Role |
|--------|------|------|
| `cursed_receive_damage_factor` | `ol-config` + `GameplayKnobs` | Haxe **1.2** — cursed target takes more damage |
| `cursed_make_damage_factor` | same | Haxe **0.5** — cursed attacker deals less |
| `pickup_baby_max_distance` | same | Haxe **1.9** — euclid doBaby/BABY/HOLD |
| `inherit_coins_factor` | same | Haxe **0.8** — coinsInherited fraction |
| `min_age_fertile` / `max_age_fertile` | same | Haxe **14** / **42** inclusive mother band |
| `cursed_receive_damage_mul` / `cursed_make_damage_mul` | `combat.rs` | pure mul from live factor |
| `can_pickup_baby_distance_ex` | `feed.rs` | live max distance |
| `apply_inherit_coins` + `InheritContext.inherit_coins_factor` | `death_inherit.rs` + `death_polish` | live InheritCoinsFactor |
| `age_fertile_ex` / `is_fertile_ex` / `can_birth_full_ex` / `format_query_sex_ex` | `fertility.rs` | live Min/MaxAgeFertile |
| `mother_fitness_ex` / `is_mother_age_fertile_ex` | `birth_fitness.rs` | live fertile band in fitness |
| `player_is_fertile` | `lib.rs` | live min/max for mother pick / nurse gates |
| BIRTH / GESTATE / ?FERTILE / continuous nurse / twin mother | `lib.rs` + `twin_party_live.inc.rs` | live fertile band |
| FIELD_MAP six rows | `field_map.rs` | **SettingsHome::Live** |
| Tests | `apply_live_settings_gameplay_knobs` + `can_birth_full_ex_live_age_band` + `mother_fitness_ex_live_age_band` + `can_pickup_baby_distance_ex_live_max` + `inherit_coins_factor_live_override` + cursed mul pure | |

Residual: `age_curves` FERTILE_MIN/MAX ModuleConst; father fitness 55 gate. **C-SS-MORE-BATCH5 DONE** — GameplayKnobs weapon CD / jump exh / AI speed + USE hungry-work heat/food/exhaust pipe + animal residual live WeaponCoolDown*.

## Rust: C-SS-MORE-BATCH5 / settings_batch5 (weapon CD / jump / hungry / AI speed)

| Symbol | File | Role |
|--------|------|------|
| `weapon_cooldown_factor` / `_if_wounding` | `ol-config` + `GameplayKnobs` | Haxe **0.5** / **5** |
| `close_enemy_with_weapon_speed_factor` | same | Haxe **0.8** |
| `exhaustion_on_jump` | same | Haxe **0.05** |
| `hungry_work_heat` | same | Haxe **0.002** per food |
| `ai_speed_factor_serf/commoner/noble` | same | Haxe **0.8/0.9/1.0** |
| `weapon_cooldown_knobs` / `vitals_speed_live_knobs` | `settings_live.rs` | live tuples |
| `bloody_weapon_after_strike_ex` / `from_zero_ex` / `weapon_bloody_time_to_change_ex` | `weapons.rs` / `weapon_wound.rs` | live CD on HIT bloody |
| `plan_animal_zero_residual_ex` / `from_content_ex` | `weapon_wound.rs` | live CD animal residual TTC |
| `apply_jump_cost_ex` | `move_path.rs` | live ExhaustionOnJump |
| `close_enemy_speed_factor_ex` / `ai_class_speed_factor_ex` / `apply_calculate_speed_full_live` | `move_speed.rs` | live close-enemy + AI class |
| `evaluate_alternative_outcome` / `alt_outcome_gate_applies` / `is_fortified_hits` / `fortification_of` | `ol-sim/src/alt_outcome.rs` | TH-ALT-OUTCOME pure L1260–1306 |
| `apply_default_alternative_outcome_patches` | `ol-content/alt_outcome_patches.inc.rs` | ServerSettings alt/fort + wall/door push(0) |
| `ContentDb::alternative_outcomes_for` | `ol-content` | transition list > new-target object list |
| `finish_cache_boot` alt patches | `ol-content/binary_cache.rs` | OLC1 path applies same tables |
| `apply_use_at` alt TryAgain/Proceed | `use_transition.rs` | live hits stamp + place_object_by_id + keep/transform + Try again say |
| `note_lock_say` / `take_lock_say` | `locks.rs` | `String` payload (dynamic Hits/Fortification) |
| `resolve_hungry_work_temperature` / `compute_hungry_work_cost` / `evaluate_hungry_work_use` / `plan_hungry_work_use` / `object_hungry_work` | `use_transition.rs` | pure TransitionHelper L1170–1256 |
| `apply_use_at` hungry-work gate | `use_transition.rs` | live heat/food/exhaustion on USE |
| Tests | `apply_live_settings_gameplay_knobs` + `hungry_work*` + `animal_zero_residual_live_*` + jump/speed pure live | |

Residual: `Transition.hungryWorkCost` / `hungryWorkTemperature` content fields (defaults 0/−1); full PatchTransitions cost table; hungry-work emote/FX sendFoodUpdate.

## Rust: C-SS-MIN-AGE-AI / min_age_ai (MinAgeToEat residual live)

| Symbol | File | Role |
|--------|------|------|
| `GameplayKnobs.min_age_to_eat` | `settings_live` / ol-config | Haxe **3** Live |
| `resolve_place_grave_id_with_min_age` / place_grave_for_conn | `death_polish.rs` | baby bone pile live |
| `send_baby_map_pin_to_parent` | `map_location_pins.rs` | BABY pin age gate live |
| `peer_count_for_kind` / `age_job_pending_ex` / `job_sensor_flags_from_sticky_ex` | `profession_scan.rs` | profession peer + age-job live |
| `is_child_and_has_mother_ex` / `sensors_from_ext_ex` / `LiveSensorInput.min_age_to_eat` | `priority_ladder.rs` | AI ladder child/food gates |
| `plan_follow_sticky_clear_ex` / `follow_max_tiles_for_context_ex` / `resolve_auto_follow_acquire_ex` | `ai_follow_walk.rs` + live.inc | AI follow clear/bands/acquire |
| `birth_cross_species_aging_mult_ex` | `food_store_max.rs` | birth aging window live |
| fever PE `UpdateEmotesInput.min_age_to_eat` | `lib.rs` tick | hunger PE gate live |
| Tests | `select_grave_live_*` / `is_child_*_ex` / `age_job_pending_ex_*` / sticky clear_ex / birth_cross_ex | |

Residual: GPI clothing MinAge gate stays commented (Haxe).

## Rust: C-SS-AGE-FOOD / age_food_max (NewBorn + OldAge Live)

| Symbol | File | Role |
|--------|------|------|
| `new_born_food_store_max` | `ol-config` + `GameplayKnobs` | Haxe **4** — youth band floor |
| `old_age_food_store_max` | same | Haxe **10** — old band floor |
| `FoodStoreMaxKnobs` | `food_store_max.rs` | `{grown_up, newborn, old_age}` sanitize + bands |
| `calculate_food_store_max_ex` / `food_store_max_from_parts_ex` | same | live all three knobs |
| `step_healing_food_pipe_ex(..., capacity)` | same | vitals recompute with age bands |
| `GameplayKnobs::food_store_max_knobs` | `settings_live.rs` | live → knobs |
| `apply_spawn_food_capacity` / tick_vitals / superMeh recompute | `lib.rs` | pass knobs |
| FIELD_MAP NewBornFoodStoreMax / OldAgeFoodStoreMax | `field_map.rs` | **SettingsHome::Live** |
| Tests | `live_newborn_band_*` / `live_old_age_band_*` / `live_combined_*` / `spawn_baby_food_max_tracks_live_newborn` / apply_live keys | |

Residual: combat `apply_damage_food_pipe` / animal hit path still module-default age bands (adult combat-dominated); Haxe `calculateNotReducedFoodStoreMax` health TODO port-as-is.

## Rust: AI-TAKEOVER disconnect_ai (S-CONN / S-SAI)

| Symbol | File | Role |
|--------|------|------|
| `attach_ai_takeover` / `release_ai_takeover` / `clear_ai_on_death` | `ol-sim/src/ai_takeover.rs` | pure flag policy (Haxe close / rlogin / doRebirth drop) |
| `player_is_ai` / `player_is_human` / `account_is_permanent_ai` | same | Haxe `isAi` / `isHuman` / account.isAi email heuristic |
| `find_reconnect_body_conn_id` / `body_eligible_for_reconnect` | same | getLastLivingPlayer-style reclaim scan |
| `reconnect_position_snap` | same | rlogin birth-origin zero + world tile keep |
| `Player.ai_controlled` + snapshot `connected`/`ai_controlled` | `ol-sim/src/player.rs` | Haxe `serverAi != null`; `is_ai_body` / `is_human_body` |
| `apply_disconnect_ai_takeover` / `try_reconnect_ai_takeover` | `ol-sim/src/lib.rs` | live wire: Disconnected / !CLOSE attach; Login reclaim rebind |
| `list_ai_takeover_conn_ids` / `on_player_death_clear_ai_takeover` | same | NPC drive list + death helper |
| age/hunger/DIE/combat death | same | `clear_ai_on_death` (no permanent AI rebirth for human email) |
| `npc_ai` takeover loop | `ol-server/src/npc_ai.rs` | thin eat/explore on `ai_controlled` player_views |
| SoulView `is_ai` | `ol-sim/src/soul_live.rs` | `player_is_ai(connected, ai_controlled, email)` |
| Tests | `ai_takeover::*` + `ai_takeover_disconnect_and_reconnect` / `_skips_deleted` / `_close_say_attaches` | pure + integration |

Haxe: `Connection.close` → `new ServerAi` when alive; `isAi` = sock==null; `rloginHelper` reclaim + drop held; `ServerAi.doRebirth` removeAi when !account.isAi. Residual: permanent AI population / CreateNewAiPlayer; full AiBase on replacement; postload ServerAi until human login.

---

## Rust: Prestige class boni (CLASS-BONI / prestige_class_table)

| Symbol | File | Role |
|--------|------|------|
| `PrestigeClass` / `PRESTIGE_CLASS_NAMES` | `ol-sim/src/prestige.rs` | Haxe `Lineage.PrestigeClass` + `PrestigeClasses` name table (4–5 Noble aliases) |
| `prestige_class_name_at_index` / `class_name` | same | Haxe `Lineage.get_className` |
| `calculate_class_boni` | same | Haxe `GlobalPlayerInstance.calculateClassBoni` (same +2, Noble↔Serf −3) |
| `calculate_needed_prestige` | same | Haxe `CalculateNeededPrestige` (1-indexed count ≥ n·percent) |
| `calculate_prestige_class_at_birth` | same | Haxe `calculatePrestigeClass` (0.4 Serf / 0.8 Noble; n&lt;2 Commoner; n&lt;5 no Noble; no King/Emperor) |
| `prestige_class_from_percentile` / `prestige_classes_from_living_scores` | same | **Online** living rank bands (20/50/75/90% incl. King/Emperor) — distinct from birth class |
| `mother_fitness` / `father_fitness` | `ol-sim/src/birth_fitness.rs` | additive class boni in fitness |
| `SimState::child_prestige_class_for_score` / `_for_email` / `living_prestige_samples_for_birth_class` | `ol-sim/src/lib.rs` | birth class from account `total_score` vs living `lineage+4·family` |
| `pick_best_mother_p_id` / `_for_child_class` / `_for_email` | same | live parent class + **child** class into MotherView |
| `pick_best_father_p_id` / `_for_child_class` | same | live father/mother class; child class for soft pens |
| `attach_fitness_mother_lineage(..., child_class)` | same | stamps birth prestigeClass on child lineage |
| Tests | `prestige::*` / `birth_fitness::mother_class_boni_*` / `pick_best_mother_serf_*` / `child_prestige_class_for_score_*` | table + birth 0.4/0.8 + pick rank |



---

## Rust: Craving restore (CRAVING-WIRE / craving_restore)

| Symbol | File | Role |
|--------|------|------|
| `YumState::do_increase_food_value` | `ol-sim/src/yum.rs` | Haxe `GlobalPlayerInstance.doIncreaseFoodValue` |
| `YumState::restore_food_count` / `cravings` | same | Haxe `restoreFoodCount` + craving list |
| `dont_change_craving` | same | Haxe L3135 `playerFrom!=playerTo \|\| !isYum` |
| `NearbyBestFood` / `PendingDisplayFood` | same | nearby pick + `displayFood` LS coords |
| `loved_food_ids_for_person_color` | same | Haxe `Biome.getLovedFoodIds` |
| `format_craving` | `ol-protocol` wire_out | Haxe `ClientTag.CRAVING` / CR |
| `try_eat_held` + USE CR/LS send | `ol-sim/src/lib.rs` | live wire after self-eat |
| `apply_feed_other_craving` + FEED/NURSE | same | feed-other reduce + do_increase(dontChange=true) + CR |
| `send_craving_and_display_after_eat` | same | CR urgent + pending displayFood LS |
| Tests | `yum::do_increase_*` / `dont_change_*` / `try_eat_held_emits_craving_wire` / `say_feed_other_craving_*` / `craving_wire_shape` | pure + live + protocol |


## EVE-BANANA / jungle_spawn

| Symbol | Location | Role |
|--------|----------|------|
| `GlobalPlayerInstance.spawnAsEve` | `server/GlobalPlayerInstance.hx` | Eve/Adam wild birth + last-pair |
| `ClearStartLocations` | same | foodArray filter |
| `getCloseSpecialBiomePersonColor` | same | Jungle→Brown / Desert→Black (+ originalBiome) |
| Rust `classify_eve_food_site` / `select_eve_food_pool` / `eve_location_fitness` | `ol-sim/eve_spawn.rs` | pure food pools + fitness |
| Rust `collect_eve_food_sites` / `find_eve_spawn` / `find_eve_spawn_for_account` | same | world scan + grave-aware live pick |
| Rust `resolve_eve_pair_partner` / `apply_eve_pair_slot_update` / `LastEveSlot` | same | lastAi/Human Eve pairing |
| Rust `pick_eve_race_person_object` / `eve_person_color_prefer_original` | same | race object + original biome color |
| Rust `split_account_graves_for_eve` / `account_has_close_*_grave` | same | bone vs gravestone fitness |
| Wire | `spawn_player_inner` synthetic Eve + sim boot preferred spawn | EVE-BANANA |


## Rust: Noob→noble spawn weights (NOOB-NOBLE-SPAWN / spawn_weights)

| Symbol | File | Role |
|--------|------|------|
| `NOOB_NOBLE_MAX_LIVES` / `NOOB_NOBLE_BIRTH_CHANCE` | `ol-sim/src/prestige.rs` | first 5 lives / 50% (Haxe design L6850) |
| `is_noob_for_spawn` | same | `lives_before_birth < 5` |
| `apply_noob_noble_spawn_weight` | same | promote Serf/Commoner→Noble on hit; never demote |
| `calculate_prestige_class_at_birth_with_noob` | same | score table + noob weight |
| `LineageNode.alive` / `SocialState::set_lineage_alive` / `ensure_lineage_alive` | `ol-sim/src/social.rs` | Haxe `Lineage.alive` (birth-class sample gate) |
| `SimState::living_prestige_samples_for_birth_class` | `ol-sim/src/lib.rs` | Haxe L1060 `lineage.alive && !deleted` |
| `SimState::child_prestige_class_for_account` / `_for_email` / `_for_email_with_roll` | same | live wire; roll inject for tests |
| `spawn_player_with_noob_roll` / `spawn_player_as_child_with_noob_roll` | same | deterministic noob roll; force-mother path skips Eve chance |
| `pick_best_mother_p_id_for_child_class` | same | Noble child → Noble mother via class boni (+2/−3) |
| Death stamp | `apply_death_inheritance` | `set_lineage_alive(p_id, false)` |
| Tests | `prestige::noob_*` / `living_prestige_samples_require_lineage_alive` / `spawn_player_noob_noble_prefers_noble_mother` / `spawn_player_veteran_serf_never_noob_noble` / `child_prestige_class_for_email_with_roll_*` | pure + integration e2e |

Haxe: `GlobalPlayerInstance` TODO L1276 "spawn noobs more likely to and as noble"; design note L6850; `calculatePrestigeClass` L1060 alive filter.


## Haxe/Rust: SearchBestFood (SEARCH-BEST-FOOD)

| Symbol | File | Role |
|--------|------|------|
| `AiHelper.SearchBestFood` / `SearchBestFoodHelperNew` / `processFood` | `auto/AiHelper.hx` | best food by score/distance |
| `process_food` / `pick_best_search_food` | `ol-sim/search_best_food.rs` | pure gates + score |
| `food_factor_for_id` / `food_factor_from_eaten_percentage` | same | WorldMap.getFoodFactor pure |
| `food_factor_for_id_ex` / `food_factor_from_eaten_percentage_ex` / `FoodFactorEatenBands` | same | live ServerSettings.FoodFactorEaten* bands (**C-SS-FULL-TABLE**) |
| `get_food_id` / `resolve_food_from_target` | same | ObjectData.getFoodId / foodFromTarget |
| `count_stock_with_piles` / `food_stock_pile_id` | same | CountCloseObjects piles (uses) |
| `in_search_food_square` | same | half-open search radius |
| `can_feed_to_me_obj_ex` / `PSILOCYBE_MUSHROOM_ID` | `ol-sim/yum.rs` | 837 + yellow fever feed gate |
| `search_best_food_full` / `search_best_food_nearby` | `ol-sim/lib.rs` (+ live.inc) | live world+container scan; cand `get_food_factor_ex` + live bands |
| `DisplayBestFood` / `display_best_food` | GPI / lib.rs | LS label when hungry |
| `IsDangerous` / `is_dangerous_near` | AiHelper / search_best_food | deadly animals + hostile_path tiles |




## Haxe/Rust: World FoodFactor (WORLD-FOOD-FACTOR)

| Symbol | File | Role |
|--------|------|------|
| `WorldMap.addFoodStatistic` / `eatenFoodPercentage` | `server/WorldMap.hx` | accumulate + % (**EATEN-FOOD-PCT** live on add in Rust) |
| `try_horse_eat` world factors + stats + superMeh + yum restore | `use_transition.rs` | Haxe doHorse→doEating L3186–3215 + doIncreaseFoodValue |
| `WorldMap.getFoodFactor` / `getStarvingFoodFactor` | same | band + death ratio |
| Eat fill × factors | `GlobalPlayerInstance.hx` | L3186–3215 |
| `higherQaulityFood` milk chains | `ServerSettings.hx` L1345–1353 | 1463→4081→3593 / 1481→4082→3596 |
| superMeh prestige/hits trade | `GlobalPlayerInstance.hx` L3195–3206 | age/prestige/hits/wound death |
| Rust `WorldFoodStats` | `ol-sim/world_food_stats.rs` | live map + HQ edges + starving counters |
| `get_food_factor_ex` | same | live FoodFactorEaten* bands |
| `super_meh_trade` / `super_meh_food_max_is_deadly` | same | pure side-effects |
| Rust `try_eat_held` | `lib.rs` | self-eat ×world×starving + superMeh + live restore/reduction |
| `EatLiveKnobs` / `YumRestoreKnobs` / `compute_eat_full` / `eat_full` | `ol-sim/yum.rs` | C-SS-FULL-TABLE live eat + craving restore knobs |
| `GameplayKnobs::eat_live_knobs` / `yum_restore_knobs` / `food_factor_eaten_bands` | `settings_live.rs` | hot-reload → pure helpers |
| `feed_fill_with_world_factors` + FEED/NURSE | `lib.rs` | feed-other factors + `add_food_statistic` |
| search cand `food_factor` | live.inc | live `get_food_factor_ex` + bands |

## DO-COMMANDS / say_commands (2026-07-26)

| Symbol | File | Role |
|--------|------|------|
| `GlobalPlayerInstance.doCommands` | `server/GlobalPlayerInstance.hx` | Natural-language SAY: I EXILE/FOLLOW/HIRE/GIVE, ORDER,, OWN THIS, HOME! |
| `processFollowCommand` / `processHireCommand` / `redeem` | same | Follow self/name; hire AI for coins; clear exile edges |
| `getLeaderWhoExiled` / `isFollowerFrom` | same | Exile gates on follow/hire; redeem chain depth |
| `NamingHelper.GetName` / `GetPlayerByName` | `server/NamingHelper.hx` | Third token + closest name |
| Rust `parse_do_command` / `parse_roman_coin_amount` / `compute_hire_cost` / `find_player_by_name` / `do_command_broadcasts_chat` | `ol-sim/speech.rs` | Pure DO-COMMANDS |
| Rust `apply_do_commands_live` / `NameCandidate` / `DoCommandEffects` | `ol-sim/do_commands_wire.rs` | Live SAY wire + hire prestige regain FX |
| Rust `process_follow_command` / `tick_pending_new_followers` / `try_set_follow_player` | `ol-sim/do_commands_wire.rs` | **LEADERSHIP-UX** delayed follow + confirm |
| Rust `FollowHireLiveKnobs` / `from_gameplay` | `ol-sim/do_commands_wire.rs` | **FOLLOW-HIRE-DELAY** live TimeConfirm + HireCost |
| Rust `NameCandidate.person_color` / `from_player_with_person_color` | `ol-sim/do_commands_wire.rs` | Haxe `getColor` / `ObjectData.person` for hire ×2 |
| Rust `try_hire` is_friendly + person_color | `ol-sim/do_commands_wire.rs` | Haxe isFriendly (ally+lastAttack) + foreign color cost |
| Rust `tick_pending_new_followers` host-name say | `ol-sim/do_commands_wire.rs` | Haxe TimeHelper confirm uses host `player.name` |
| Rust `GameplayKnobs.time_confirm_new_follower` / `hire_cost*` | `ol-sim/settings_live.rs` + `ol-config` | LiveSettings hot-reload |
| Rust `get_top_leader` / `get_top_leader_or_self` | `ol-sim/relations.rs` | exile/deleted/circular→None (Haxe getTopLeader) |
| Rust `TIME_CONFIRM_NEW_FOLLOWER` / `format_following_for_player` / `format_following_line_unfollowed` / `following_badge_color` | `ol-sim/social.rs` | FW -1 + top badge + pending UX strings |
| Rust `format_map_location_says_body` FOLLOWER | `ol-sim/leadership.rs` | map pin body on follow request |
| Rust `Player.new_follower_id` / `new_follower_for_id` / `new_follower_time` | `ol-sim/player.rs` | session pending slots |
| Rust `SocialState::redeem` / `leader_who_exiled` / `hired_by` / `set_hired` | `ol-sim/social.rs` | Redeem chain + exile gate + hire map |
| Wire | `apply_say_or_remv` + `tick_vitals` pending confirm | natural-language forms; FW fan-all |
| Tests | `speech::*` / `social::redeem_*` / `do_commands_wire::follow_*` / `get_top_leader_*` / `say_do_commands_*` | pure + live |

Residual: multi-owner `addOwner`; AiBase MAKE/CRAFT hear path; HOME! firePlace side-effect. **FOLLOW-HIRE-DELAY DONE** (live TimeConfirm/HireCost; hire immediate; isFriendly+person-color cost; host-name confirm-say; FW fan-all + hire UX; pending slots PLB1). Haxe TODOs deferred: follower badge color resend; allow other-leader follow; hire leaders path. Leader range → **LEADER-RANGE**.


## HEALTH-AGE-FOOD / health_food_max

| Symbol | File | Role |
|--------|------|------|
| `CalculateHealthFactor` / `CalculateHealthFoodStoreMaxFactor` / `CalculateHealthAgeFactor` | `server/GlobalPlayerInstance.hx` | yum_multiplier vs medianPrestige health curve |
| `calculateFoodStoreMax` | same | capacity × health (adult) + age bands |
| `TimeHelper.updateAge` | `server/TimeHelper.hx` | trueAge wall-clock; age × ageingFactor; birth mults; age_r |
| GPI init `yum_multiplier` / food_store_max | `server/GlobalPlayerInstance.hx` L998–1003 | birth seed + calculateFoodStoreMax |
| Rust `calculate_health_factor` / `calculate_health_food_store_max_factor` / `calculate_health_age_factor` | `ol-sim/food_store_max.rs` | pure |
| Rust `age_step_from_health` / `birth_cross_species_aging_mult` / `birth_yum_multiplier` / `age_r_from_ageing_factor` / `median_prestige_for_health` | same | aging step + birth mult + age_r + birth yum |
| Rust `SimState::player_health_*_factor` / `player_birth_aging_mult` / `seed_birth_yum_prestige` / `apply_spawn_food_capacity` / `recompute_median_prestige` | `ol-sim/lib.rs` | live wire |
| Rust `Player.age_r` | `ol-sim/player.rs` | Haxe age_r |
| Rust tick_vitals / spawn / revive / spawn_child / FX food_change / combat / animal | `ol-sim/lib.rs` | food_max + age deltas + wire |


## MAP-LOCATION-PINS / social_pins (2026-07-28)

| Symbol | File | Role |
|--------|------|------|
| `Connection.sendMapLocation` | `server/Connection.hx` | PS body `text1 *text2 p_id *map rel_x rel_y` (birth transform) |
| Login mother pin | `Connection.hx` L281 | `MOTHER *leader` after map chunk / close players |
| Birth baby pin | `GlobalPlayerInstance.hx` L1013–1027 | `BABY *baby` to human mother + father |
| CountAndDisplayFollower/Ally/Family | `GlobalPlayerInstance.hx` L6728+ | name balloons + closest pin; `isAlly` exile-aware |
| `?ALLY`/`ALLY?`/`?F`/`FOLLOWER?`/`?FAM`/`FAM?` | same doCommands | private vs public count (`toSelf`) |
| `?F` → `sendToMeAllFollowings(true)` | `Connection.hx` | FW refresh before CountAndDisplayFollower |
| `!H` / `!HUMAN` | GPI L5710 | `HUMAN *follower`; `allowShowHuman` |
| Age-10 father re-follow | `TimeHelper.hx` L780–805 | LEADER pin to child + FOLLOWER to father |
| Follow-request FOLLOWER pin | GPI processFollow | birth-relative transformX/Y |
| Rust pure + live | `ol-sim/map_location_pins.rs` | format + CountAndDisplay + login/birth/gestation/SAY/HIT/age-10 |
| Rust wire | `lib.rs` login / BIRTH / gestation / apply_say / tick_vitals; `do_commands_wire` follow pin | |
| Tests | `map_location_pins::*` / `social_map_pins_ally_follower_human` | pure labels + gates + live SAY pins |

Residual: LOCATION_SAYS `markers` (separate); age-10 spoken say/emote pair polish.

## LEADER-RANGE / leader_break (2026-07-26)

| Symbol | File | Role |
|--------|------|------|
| `Connection.sendToMePlayerInfo` topLeader exception | `server/Connection.hx` | Far top leader still gets PU (client break workaround) |
| `Connection.sendLeader` / `sendDirectLeader` / `sendMapLocation` | same | LEAD / !L map pin PS |
| `Server` case LEAD | `server/Server.hx` | LEAD tag → sendLeader |
| `GlobalPlayerInstance` `!L`/`?L`/`!DL` | `server/GlobalPlayerInstance.hx` | Power say + optional map |
| Rust `decide_player_info_range` / `is_close_pu` / `format_leader_map_location_body` / `is_top_leader_of` | `ol-sim/leadership.rs` | Pure range + map body + chain helper |
| Rust `nearby_conn_ids_for_player_update` / `apply_leader_query` / `parse_leader_personal_command` | `ol-sim/leader_range.rs` | Live PU fan-out + LEAD/!L |
| Rust `send_forced_player_update` / `send_action_result_pu_and_frame` | `ol-sim/lib.rs` | Use leader-aware fan-out |
| Rust `format_player_out_of_range` | `ol-protocol` | PO wire |
| Tests | `leadership::*` / `leader_range::*` / `lead_and_bang_l_send_map_pin` | pure + live |

Residual (closed by **PO-FAR-PLAYERS**): full SendToMeAllClosePlayers PO + torus `is_close_pu_wrap`. `get_top_leader` exile/deleted/circular → **LEADERSHIP-UX** DONE.

## PO-FAR-PLAYERS / player_out_of_range (2026-07-28)

| Symbol | File | Role |
|--------|------|------|
| `Connection.SendToMeAllClosePlayers` | `server/Connection.hx` L393 | Viewer-centric roster sweep + FRAME |
| `Connection.sendToMePlayerInfo` | same L414–448 | Far non-leader → PO; top leader exempt; held skip; moving gate |
| `WorldMap.transformX/Y` | `server/WorldMap.hx` | Torus relative before `isClose` |
| `GlobalPlayerInstance.isClose` | `server/GlobalPlayerInstance.hx` | Squared-Euclidean distance gate |
| `TimeHelper` `SendMoveEveryXTicks` | `server/TimeHelper.hx` L132–135 | Periodic refresh `sendMoving=false` |
| Rust `is_close_pu_wrap` / `decide_player_info_range_wrap` | `ol-sim/leadership.rs` | Torus-aware isClose |
| Rust `decide_viewer_subject_wrap` / `collect_far_non_leader_p_ids_wrap` | `ol-sim/leader_range.rs` | Pure PO/PU + wrap |
| Rust `send_to_me_all_close_players` / `_all_viewers` / `should_refresh_close_players` | same | Live wire + LOGIN + tick gate |
| Rust `format_player_out_of_range` | `ol-protocol` | `PO\np_id …\n#` |
| Rust `math_wrap::wrap_delta` | `ol-sim/math_wrap.rs` | Shared torus dx/dy |
| Tests | `leader_range::*` / `leadership::*` / `math_wrap::*` | pure + live torus + send_moving=false |

Residual: product Haxe `MaxDistanceToBeConsideredAsClose` often 2e6 (Rust uses `NEARBY_RANGE`/`broadcast_all`); `SEND_MOVE_EVERY_X_TICKS` const not LiveSettings; NAME body lineage quality.

## PO-MAX-DISTANCE / close_say_range (2026-07-29)

| Symbol | File | Role |
|--------|------|------|
| `ServerSettings.MaxDistanceToBeConsideredAsCloseForSay` | `settings/ServerSettings.hx` L266 | CloseForSay = **20** (not PU MaxDistance) |
| `ServerSettings.MaxDistanceToBeConsideredAsCloseForSayAi` | same L267 | AI hear = 20 |
| `Connection.sendSayToAllClose` | `server/Connection.hx` | isClose + PLAYER_SAYS + FRAME; AI say loop |
| `GlobalPlayerInstance.isClose` / `AiHelper.CalculateDistance` | server | Euclidean² + wrap |
| Rust `ADULT_CHAT_RANGE` / `MAX_DISTANCE_CLOSE_FOR_SAY` | `ol-sim/speech.rs` | **20** ModuleConst |
| Rust `chat_range_for_age` | `speech.rs` + live `lib.rs` | infant 8 / child 16 / adult 20; NaN→adult |
| Rust `NEARBY_RANGE` | `ol-sim/lib.rs` | **24** PU/MX interest only |
| Rust live SAY / `send_chat_ps` / AI LLM chunks / do-commands spoken_says | `lib.rs` | `ADULT_CHAT_RANGE` or age-scaled |
| Rust pending newFollower `spoken_says` | `tick_vitals` + `tick_pending_new_followers` | CloseForSay 20 (was residual NEARBY 24) |
| Rust social-pin public count / coins say / moskitos say | `lib.rs` | CloseForSay 20 |
| Tests | `speech::age_brackets` / `say_adult_close_for_say_range_twenty` / `pending_follower_spoken_says_close_for_say_range` / `chat_range_for_age_nan_matches_speech` / `mumble::mumble_narrower_than_adult` | pure + live |

Residual: Euclidean² vs Chebyshev metric (product-wide); MuteBook/DEAF on `send_chat_ps` (Rust product); young age soft scale (Rust product; Haxe always 20).


## Haxe/Rust: FoodStats disk dump (FOODSTATS-DISK)

| Symbol | File | Role |
|--------|------|------|
| `WorldMap.writeFoodStatistics` | `server/WorldMap.hx` | % recompute + text dump |
| Save call | `WorldMap.write` | `FoodStats{N}.txt` after index |
| `WebServer.generateFoodStatistics` | `server/WebServer.hx` | HTML Food/Eaten/Related table |
| `WorldMap.write` TraceCountObjectsToDisk | `server/WorldMap.hx` | optional `ObjectCounts{N}.txt` |
| Rust `format_stats_line` / `write_food_statistics` | `ol-sim/world_food_stats.rs` | Haxe line shape + disk |
| Rust `format_food_statistics_html` | `ol-sim/world_food_stats.rs` | pure HTML table (**FOODSTATS-WEB**) |
| Rust `haxe_food_stats_slot_filename` | same | Haxe slot name helper (fixed name default) |
| `WorldFoodShare` + sim mirror | `lib.rs` / `SimBootLive` | autosave + web snapshot |
| ol-web `/stats/food` + `WebState.food_view` | `ol-web/lib.rs` | live Food/Eaten/Related HTML |
| ol-config `food_stats_save_path` | `ol-config` | `FoodStats.txt` |
| ol-server autosave/shutdown + food_view wire | `ol-server/main.rs` | dump + web share |
| Rust `format_object_count_line` / `write_object_counts` | `ol-sim/long_term.rs` | ObjectCounts pure dump |
| `WorldMap.countObjects` / `updateObjectCounts` | `server/WorldMap.hx` | full ground+nest census |
| Rust `count_objects_from_world` / `count_parent_id` | `ol-sim/long_term.rs` | pure Haxe countObjects |
| Rust `LongTermState::update_object_counts` / `ensure_counts_for_dump` | `long_term.rs` | current recompute + boot seed |
| Rust `should_update_object_counts` | `long_term.rs` | `(tick+20)%600` gate |
| TimeHelper updateObjectCounts | `server/TimeHelper.hx` L162 | periodic recompute |
| ol-config `object_counts_save_path` | `ol-config` | `ObjectCounts.txt` |
| Rust `ObjectCountsShare` + sim mirror | `object_counts_share.rs` / `SimBootLive` | autosave snapshot |
| ol-server autosave/shutdown | `ol-server` | `write_object_counts` ObjectCounts.txt |
| Tests | `world_food_stats::*` / `long_term::count_objects*` / `format_object_count*` / `object_counts_share::*` | pure + disk |

## Rust: dropHeldObject smart AI (DROP-HELD-AI / DROP-HELD-LIVE)

| Symbol | File | Role |
|--------|------|------|
| `drop_held_object` / `DropHeldInput` / `DropHeldDecision` | `ol-sim/src/drop_held_ai.rs` | pure AiBase.dropHeldObject planner |
| `store_in_quiver` / `QuiverClothing` / `can_add_to_quiver` / `QUIVER_CLOTHING_SLOT` | same | bow/arrow → SELF slot 5; uses/numUses capacity |
| `use_up_dough` / `UseUpDoughInput` | same | Bowl of Dough 252 → Clay Plate keep-last |
| `should_drop_near_fire` / `oven` / `forge` / `well` | same | dropNear*ItemIds tables |
| `pile_blocked` / `must_use_as_drop` / `force_drop_at_feet` | same | dontUsePile / dontUseDrop / peel-at-feet |
| `fill_anchors_from_scan` / `DropHeldAnchors` | same | forge/kiln/well from ScanTile |
| `consider_drop_held_object` / `consider_drop_held_decision` / `_ex` | same | interrupt goto; UseUpDough then 185→85 |
| `resolve_prefer_short_craft` / `to_live_intent` | same | PreferShortCraft → UseAt; Goto walk; SelfClothing live; **BusyMoving→Wait**; PreferShortCraft craft_actor → SeekOrCraft craft_if_needed (**PREFER-SHORT-WAIT**) |
| `drop_held_decision_to_live_intent` | `prefer_short_busy_to_live.inc.rs` | free-function alias of BusyMoving/PreferShortCraft live map |
| `plan_drop_held_live` / `smart_drop_held_to_live_intent` / `smart_drop_held_from_sensors` | same | DROP-HELD-LIVE pure enqueue bridge |
| `drop_held_input_from_sensors` / `DropHeldSensorExtras` | same | scan anchors + knife/bread/quiver fill |
| `self_clothing_raw_payload` | same | `SELF 0 0 slot` payload |
| `count_bread_family_near` / `has_knife_near_scan` | same | UseUpDough scan sensors |
| `smart_drop_held_profession` / `smart_drop_held_profession_ex` / `drop_held_live_intent_actionable` / `live_intent_is_wait` | `short_craft_intent.rs` | parent bridge; `_ex` passes is_moving (BusyMoving→Wait); Wait is actionable + not wire |
| `ShortCraftLiveIntent::Goto` / `SelfClothing` / `Wait` | `short_craft_intent.rs` | dropOnStart walk; quiver SELF; isMoving hold-tick |
| `get_or_craft_result_to_live_intent` BusyMoving | `get_or_craft.rs` | Haxe isMoving return true → Wait (**PREFER-SHORT-WAIT**) |
| `ProfessionScanInput.is_moving` | `profession_scan.rs` | Haxe myPlayer.isMoving for dropHeld |
| `pottery/farm/smith/baker DropHeld` | `profession_scan.rs` | smart_drop_held_profession_ex(..., is_moving) |
| `ladder_profession_scan_tick` Wait terminal | same | hold tick; no makeStuff fallthrough |
| `npc_ai` Goto / SelfClothing / Wait / force_drop_at_feet | `ol-server/npc_ai.rs` | Move + Raw SELF + prof_wait_busy_moving + smart drop peels |
| `selfplay` SMART-DROP path | `ol-server/selfplay.rs` | Drop/Use/SELF/Goto/SMART-DROP-WAIT (no feet-drop on Wait) |
| `closest_free_container` / `closest_with_contains` | `drop_held_ai.rs` | numSlots drop-in; clay basket prefer |
| `should_drop_on_table` / `is_baked_pie` / `is_small_food_to_store` / `allows_drop_in_container` | `drop_held_ai.rs` | DROP-HELD-TABLE / AiHelper L30–40 + L195 free-container gate |
| `container_prefer_factor` / `adjust_container_drop_score` / `closest_preferred_container` / `best_empty_or_container_drop` | same | Table 3371 0.25; box 3065 0.25; basket 292 0.5; other 0.8; same-food ×0.5; joint empty score |
| `quiver_from_clothing_snapshot` / `QuiverClothing::from_clothing_snapshot` / `clothing_ids_snapshot` | same | storeInQuiver clothingObjects scan |
| `Player::clothing_parent_ids` / `clothing_uses_remaining` / `PlayerSnapshot.clothing` / `clothing_uses` | `player.rs` | 6-slot clothingObjects for quiver sensors |
| `npc_ai` `DropHeldSensorExtras.quiver` from snapshot | `ol-server/npc_ai.rs` | force_drop_at_feet smart path fills quiver |
| `DROP_NEAR_FIRE_IDS` / `DONT_USE_PILE_IDS` / `DONT_USE_DROP_FOR_ITEMS` / `SKEWERED_RABBIT` / `OMELETTE` / `TABLE` / `WOODEN_SLOT_BOX` | same | Haxe id tables |
| `ScanTile::{num_slots,num_uses,contains_id,contained_count,is_full_uses,has_free_slot}` | `profession_scan.rs` | dropHeld capacity snapshot |

### AI-HANDLER / S-AIH (`AiHandler.hx` → `ai_handler.rs`)

| Haxe | Rust | Notes |
|------|------|-------|
| `checkRateLimit` / `recordCall` / `cleanOldTimestamps` | `AiCallRateLimit` | 1h window; `AiCallsPerHour` default 500 |
| `ChatResponse` | `chat_response_with` | provider inject; max 2 attempts; network-error retry |
| `isNetworkError` | `is_network_error` | network vs API patterns |
| `buildPrompt` | `build_prompt` / `PromptParts` | soul + relation + memory + command schema |
| `getRelationshipInfo` | `get_relationship_info` / `RelationshipView` | pure flags snapshotted on main thread |
| `checkIfShouldDoCommand` | `check_if_should_do_command` | follower / close relative / reject |
| `getCommandContext` | `get_command_context` | JSON action schema |
| `getEmoteId` | `get_emote_id` | happy…homesick table |
| `parseAiResponse` | `parse_ai_response` → `ParsedAiResponse` | text/emote/actions pure |
| parse side-effects | `plan_apply_parsed_ai_response` → `ApplyAiResponsePlan` | emote+300 / follow / drop / makeItem; live → **AI-LLM-APPLY** |
| `logToFile` | `format_conversation_log_entry` + `append_conversation_log` / `log_conversation_to_file` / `log_conversation_to_file_now` | daily path; live on drain worker |
| `respondToPlayerAsync` | `plan_respond_to_player` + `process_llm_response_for_say` / `plan_speech_llm_complete` | async stages pure + complete plan |
| `sendResponseInChunks` / `splitResponse` | `plan_response_chunks` / `enqueue_llm_say_chunks` / `poll_ready_llm_say` | MaxAIResponseperSay; tick drain |
| AiBase speech gate | `should_invoke_llm_for_speech` / `plan_speech_llm_start` / `speech_llm_gate_from_runtime` | age/!/?/cooldown; oreally+`...` |
| `checkIfYouAreAllied` | `check_if_you_are_allied_speech` | silent vs `I AM NOT YOUR ALLY!`+angry |
| `sendSayToAllClose` AI loop | `collect_ai_speech_hearers` + `fan_out_ai_speech_llm` | dist 20 + ALL/!!/??/name/closest |
| `timeReactedLastCommand` | `Player.llm_speech` / `LlmSpeechRuntime` | cooldown + pending chunks + in_flight |
| job queue | `take_llm_speech_jobs` / `push_llm_speech_result` / `tick_llm_speech_wire` | apply path |
| HTTP drain | `export_llm_speech_jobs_to_share` / `import_llm_speech_results_from_share` / `LlmSpeechIoShare` | sim↔server |
| `run_llm_speech_http_drain` | `ol-server::ai_provider::run_llm_speech_http_drain` | take→`call_ai_async`→`logToFile`→push |
| job→result | `llm_speech_job_to_result` | pure map |
| drain params | `try_drain_params_from_env` → `(CallAiParams, limit, log_base)` | key + `AI_CONVERSATION_LOG_BASE` |
| secrets | `api_key_from_env` / `ol-server::ai_llm_env::LlmEnvConfig` | env only; never server.toml; boot `from_env` + `debug_status` |

### AI-PROVIDER / S-AIP (`AIProvider.hx` → `ai_handler` pure + `ol-server/ai_provider` HTTP)

| Haxe | Rust | Notes |
|------|------|-------|
| `IsLLMActivated` | `is_llm_activated` / `LlmEnvConfig::is_activated` | key ≠ empty / Not Set |
| `callAi` request body | `build_ai_request_body` | system dialog prompt; max_tokens |
| `callAi` URL | `ai_messages_endpoint` | `{AiApiUrl}/v1/messages` |
| `callAi` headers | `ai_request_headers` | Bearer + x-api-key + anthropic-version |
| `callAi` model/url | `resolve_ai_model` / `resolve_ai_api_url` | MiniMax defaults |
| `callAi` HTTP | `ol-server::ai_provider::call_ai` / `call_ai_async` / `CallAiParams` | reqwest; 120s; inject for `chat_response_with` |
| `parseResponse` | `parse_provider_response` | content[] text; choices fallback; type=error |
| secrets | env `AI_API_KEY`/`XAI_API_KEY`/`AI_API_URL`/`AI_DEFAULT_MODEL`/`AI_MAX_TOKENS_FOR_CHAT` | SecretOmit; never server.toml |
| Residual | — | **AI-LLM-WIRE** speech→async SAY; multi-server twins **parked** |


### AI-LLM-WIRE (`AiBase.say`/`sayHelper` → speech_llm)

| Haxe | Rust | Notes |
|------|------|-------|
| `MaxDistanceToBeConsideredAsCloseForSayAi` | `MAX_DISTANCE_SAY_AI` (20) / `ai_within_say_range` | quad-dist |
| attention ALL/!!/??/name/closest | `ai_speech_attention` / `collect_ai_speech_hearers` | pure |
| LLM fallback gate | `plan_speech_llm_start` | human+activated+age>3+!/?/cooldown |
| immediate oreally/`...`/ally stop | live `fan_out_ai_speech_llm` | free-form SAY path |
| async + chunks | jobs/results + `tick_llm_speech_wire` | PE + chat memory + chunk SAY |
| Residual | — | live RelationshipView full; **AI-SAY-HELPER DONE**; **AI-FOLLOW-WALK DONE**; chunk log lines; toSoul other; exile-branch TODO |

### AI-LLM-APPLY (`parseAiResponse` live → `ai_llm_apply.rs`)

| Haxe | Rust | Notes |
|------|------|-------|
| `doEmote(id,300)` | PE via `format_player_emot` (seconds unused in Haxe) | live tick |
| `startFollowingPlayer` | `plan_start_following_player` + `Player.ai_follow_p_id` + `try_ai_follow_path_to` | follows **speaker** (Haxe self-bug fix) + Goto(speaker+1) |
| `doDropCommand` | `plan_do_drop_command` + `ai_ordered_to_drop` + `apply_drop` feet | waiting_time_min=1 |
| `doMakeCraftCommand(s,true)` | `resolve_make_item_id` + `craft_ai.do_make_craft_command` silent | bare id/name + aliases |
| `findObjectByCommand` / `GetObjectByName` | `make_item_search_token` / `get_object_by_name_like` | pure |
| live apply | `apply_sticky_from_plan` in `tick_llm_speech_wire` | **DONE** |
| ally `goto_speaker` | `ally_goto_speaker_xy` + `try_ai_follow_path_to` | **AI-FOLLOW-WALK** |

### AI-FOLLOW-WALK (`AiBase.isMovingToPlayer` continuous_follow)

| Haxe | Rust | Notes |
|------|------|-------|
| `isMovingToPlayer` / distance gates | `decide_follow_walk` / `follow_max_tiles_for_sticky` | ordered 5 vs loose 10 |
| sticky auto-clear 5min + age | `plan_follow_sticky_clear` / `apply_follow_sticky_clear` | `ORDERED_FOLLOW_MAX_SECS` / `AUTO_STOP_FOLLOW_CLEAR_AGE` |
| `gotoAdv` stand-off goal | `follow_goal_xy` / `follow_stand_half_range` / `follow_seed` | deterministic offset |
| ally / startFollowing Goto(speaker+1) | `ally_goto_speaker_xy` + `try_ai_follow_path_to` | path step cap 10 |
| continuous tick | `tick_ai_follow_walk` after `tick_llm_speech_wire` | skip repath while moving |
| NPC hold profession | `npc_ai` follow_walk before ladder scan | uses `PlayerSnapshot.ai_follow_*` |
| Residual | — | AutoFollowPlayer closest-human; child-mother getFollowPlayer; debug say name; baby/child/wounded bands |

### AI-SAY-HELPER (`AiBase.sayHelper` scripted_cmds)

| Haxe | Rust | Notes |
|------|------|-------|
| `sayHelper` HOLA/HELLO/HI | `plan_scripted_say_helper` + `fan_out_ai_say_scripted` | weapon/angry gates + cooldown 4s |
| NAME? / ARE YOU AI / NICE? / JUMP! | same | JUMP live: `apply_player_jump` PU+BW / drop (**JUMP-BW-FULL**) |
| MOVE! / FOLLOW Goto(speaker+1) | `ally_goto_speaker_xy` + `try_ai_follow_path_to` | **AI-FOLLOW-WALK** pathfind |
| FOLLOW/COME / STOP FOLLOW / STOP/WAIT | sticky `ai_follow_*` / `ai_ordered_to_drop` | STOP `waiting_time_set=10` assign |
| DROP / `doDropCommand` | `ordered_to_drop` + `tick_ordered_ai_drop` | deferred next tick (not immediate feet) |
| GO HOME / `isMovingToHome` | `move_to_home` + `go_home_*` helpers + pathfind | debug GOING/CANNOT when `ai_debug_say` |
| HOME! / SearchNewHome / GetCloseFire | `home_oven_biome_allowed` + `get_close_fire` + `ai_fire_place_*` | swamp no-floor skip; local r=80 |
| MAKE/CRAFT | `resolve_make_item_id` + `craft_ai.do_make_craft_command` | ally gate; non-silent say |
| PROF?/PROF ON / profession! | `create_profession_text` / `AI_PROFESSIONS` | assigned_profession |
| checkIfYouAreAllied / checkIfShouldDoCommand | `plan_ally_gate` / `plan_should_do_command` | loud reject + angry PE |
| Residual | — | ~~full JUMP BW~~ **JUMP-BW-FULL DONE**; `this.time`→waiting floor; global oven list |


### FEED-OTHER-YUM / feed_full_eat
| Symbol | Path | Notes |
|--------|------|-------|
| `doEating` (feed-other) | `server/GlobalPlayerInstance.hx` L3041–3247 | playerFrom ≠ playerTo |
| `feed_other_full_eat` | `ol-sim/lib.rs` (inline; `feed_other_yum_live.inc.rs` mirror) | compute_eat + prestige + multi-use + drugs + gates |
| `feed_other_feeder_prestige_delta` | `ol-sim/feed_other_yum.rs` | yum feeder ×0.2 |
| `feeder_may_eat_or_feed` | `ol-sim/feed_other_yum.rs` | MinAgeToEat + yellow fever |
| `apply_drugs_fever_resistance` | `ol-sim/feed_other_yum.rs` | isDrugs yf count + TTC |
| `feed_other_eater_post_emote` | `ol-sim/feed_other_yum.rs` | miam/happy/ill/sad |
| `feed_other_responsible_id` | `ol-sim/feed_other_yum.rs` | self −1 / feeder p_id |
| `can_feed_to_me_obj_ex_yum` | `ol-sim/yum.rs` | meh refuse food>2; 837 fever |
| `eat_actor_after_use` | `ol-sim/multi_use.rs` | FEED multi-use bowl |
| Tests | `feed_other_*` multi_use / too_young / drugs / 837 / yum fill / meh refuse | live + pure |

## LINEAGE-24H / starving_window (2026-07-29)

| Symbol | File | Role |
|--------|------|------|
| `Lineage.GenerateLineageStatistics` / `reasonKilledLastDay` | `server/Lineage.hx` | rebuild last-day death reason map (yearsSinceDeath < 1440) |
| `WorldMap.getStarvingFoodFactor` | `server/WorldMap.hx` | hunger/age last-day ratio → fill mult |
| `WebServer.generateLineageStatistics` | `server/WebServer.hx` | death-reason HTML + starving % |
| `starving_food_factor_from_deaths` / `note_death_reason_at` | `ol-sim/world_food_stats.rs` | pure formula + session death stamps |
| `get_starving_food_factor_at` / `refresh_starving_window` | same | 24h window + 60s throttle |
| `normalize_death_reason_for_stats` / `_ex` / `_with_resolver` | same | kid hunger + optional kill→name |
| `parse_reason_killed_object_id` | same | `reason_killed_<id>` parse |
| `generate_lineage_statistics` / `LineageStatistics` / `LineageStatRow` | same | full reason/age/gen maps (day+hour+all) |
| `death_stamps_from_lineage_rows` / `seed_death_stamps_from_lineage_rows` | same | boot rehydrate from lineage deaths |
| `format_lineage_death_reason_html` / `format_lineage_ages_html` / `format_lineage_statistics_html` | same | pure WebServer death-reason + ages + starving % |
| `LineageNode.death_sim_time` / `stamp_lineage_death` | `ol-sim/social.rs` | deathTime/reason (ensure node) |
| `SocialState.lineage_stat_rows` / `snapshot_at` / `LineageSnapshot::stat_rows` | same | AllLineages → `LineageStatRow` + web sim_time |
| OLN2 death fields | `ol-sim/lineage_persist.rs` | persist death_sim_time/reason/age; load v1+v2 |
| `GET /stats/lineage` | `ol-web/lib.rs` | Haxe WebServer.generateLineageStatistics HTML |
| `GET /stats/players` + `/players` | same | Haxe createCurrentlyPlayingStatistics living table |

## WEB-ACCOUNTS-STATS / accounts_score_table (2026-07-29)

| Symbol | File | Role |
|--------|------|------|
| `generateAccountStatistics` | `server/WebServer.hx` L301–339 | score table ID/Prestige/Female/Male/Coins |
| `PlayerAccount.totalScore` / `femaleScore` / `maleScore` / `coinsInherited` / `isAi` / `scoreName` | `server/PlayerAccount.hx` | leaderboard fields |
| `AccountSummary` + `female_score` / `male_score` / `is_ai` / `coins_inherited` | `ol-sim/accounts.rs` | web snapshot fields |
| `AccountRecord.haxe_total_score` | same | floor((male+female)/2) or stored total_score |
| `account_email_looks_ai` / `account_score_display_id` | same | isAi heuristic + ID column (last_name) |
| `format_account_statistics_html` | same | pure Haxe HTML fragment (filter ≥5, non-AI, sort desc) |
| `GET /stats/accounts` | `ol-web/lib.rs` | real table (not `{:?}` dump) |
| Tests | `accounts::format_account_statistics_html_*` | filter / sort / columns |

## HEALTH-PRESTIGE-FAN / addHealthAndPrestige (2026-07-29)

| Symbol | File | Role |
|--------|------|------|
| `addHealthAndPrestige` | `server/GlobalPlayerInstance.hx` L5997 | yum + coins + clothing parent/leader fan |
| `clothing_prestige_factor` / `prestige_fan_deltas` | `ol-sim/health_prestige.rs` | pure clothing + family/leader shares |
| `apply_eat_health_prestige` | `ol-sim/lib.rs` | live self + feed-other + darkNosaj gate + coins + fan |
| self-eat wire | `try_eat_held` | was missing health_delta → now applies fan |
| Tests | pure clothing/parent/child/leader + `dark_nosaj_blocks_eat_health_prestige` | residual: ObjectData.prestigeFactor map + extraPrestigeFactor crowns |

### MOVE-NEST-SPEED
| `held_nest_speed_product` / `combine_backpack_and_held_nest` | `ol-sim/src/move_speed.rs` (+ `move_nest_speed_inc.rs`) | Haxe held containedObjects +1 nest mult after backpack shoes-√ |
| `backpack_nest_speed_product` / `resolve_backpack_speed_product` | same | Haxe `getPackpack().containedObjects` (clothing[5]); flat `Player.backpack` fallback when no equipped pack |
| `VitalsSpeedInput.held_nest_product` | same + live path-start / `player_move_speed` | live nest product from `Player.held_helper`; clothing pack wired into `apply_calculate_speed_full` |

