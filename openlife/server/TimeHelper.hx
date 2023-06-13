package openlife.server;

import haxe.Exception;
import openlife.auto.AiBase;
import openlife.auto.AiHelper;
import openlife.client.ClientTag;
import openlife.data.Point;
import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;
import openlife.data.transition.TransitionData;
import openlife.data.transition.TransitionImporter;
import openlife.macros.Macro;
import openlife.server.Biome.BiomeTag;
import openlife.server.GlobalPlayerInstance.Emote;
import openlife.server.Lineage.PrestigeClass;
import openlife.settings.ServerSettings;

@:enum abstract Seasons(Int) from Int to Int {
	public var Spring = 0;
	public var Summer = 1;
	public var Autumn = 2;
	public var Winter = 3;
}

class TimeHelper {
	public static var tickTime = 1 / 20;
	public static var tick:Float = 0; // some are skipped if server is too slow

	// public static var allTicks:Float = 0;   // these ticks will not be skipped, but can be slower then expected
	public static var lastTick:Float = 0;
	public static var serverStartingTime:Float;

	// Time Step Stuff
	private static var worldMapTimeStep = 0; // counts the time steps for doing map time stuff, since some ticks may be skiped because of server too slow
	private static var TimeTimeStepsSartedInTicks:Float = 0;
	private static var TimePassedToDoAllTimeSteps:Float = 0;
	private static var WinterDecayChance:Float = 0;
	private static var SpringRegrowChance:Float = 0;

	// Long Time Step Stuff
	private static var LongTimePassedToDoAllTimeSteps:Float = 0;
	private static var LongTimeTimeStepsSartedInTicks:Float = 0;

	// Seaons
	private static var TimeToNextSeasonInYears:Float = ServerSettings.SeasonDuration;
	private static var TimeSeasonStartedInTicks:Float = 0;
	public static var Season:Seasons = Seasons.Spring;
	private static var SeasonNames = ["Spring", "Summer", "Autumn", "Winter"];
	public static var SeasonTemperatureImpact:Float = 0;
	private static var SeasonHardness:Float = 1;
	public static var SeasonText:String = 'DONT KNOW';

	public static var ReadServerSettings:Bool = true;

	public static function CalculateTimeSinceTicksInSec(ticks:Float):Float {
		return (TimeHelper.tick - ticks) * TimeHelper.tickTime;
	}

	public static function CalculateTimeSinceTicksInYears(ticks:Float):Float {
		return CalculateTimeSinceTicksInSec(ticks) / 60;
	}

	public static function DoTimeLoop() {
		serverStartingTime = Sys.time();
		var averageSleepTime:Float = 0.0;
		var skipedTicks = 0;
		var timeSinceStartCountedFromTicks:Float = TimeHelper.tick * TimeHelper.tickTime;
		serverStartingTime -= timeSinceStartCountedFromTicks; // pretend the server was started before to be aligned with ticks

		// trace('Server Startign time: sys.time: ${Sys.time()} serverStartingTime: $serverStartingTime timeSinceStartCountedFromTicks: $timeSinceStartCountedFromTicks');

		DoTest();

		// if(ServerSettings.NumberOfAis > 0) Ai.StartAiThread();
		AiBase.StartAiThread();

		while (true) {
			if (ServerSettings.UseOneGlobalMutex) Server.Acquire();

			TimeHelper.tick = Std.int(TimeHelper.tick + 1);

			var timeSinceStart:Float = Sys.time() - TimeHelper.serverStartingTime;
			timeSinceStartCountedFromTicks = TimeHelper.tick * TimeHelper.tickTime;

			// TODO what to do if server is too slow?
			if (TimeHelper.tick % 10 != 0 && timeSinceStartCountedFromTicks < timeSinceStart) {
				TimeHelper.tick = Std.int(TimeHelper.tick + 1);
				skipedTicks++;
			}
			if (TimeHelper.tick % 200 == 0) {
				averageSleepTime = Math.ceil(averageSleepTime / 200 * 1000) / 1000;
				trace('\nHum: ${Connection.CountHumans()} Time From Ticks: ${timeSinceStartCountedFromTicks} Time: ${Math.ceil(timeSinceStart)} Skiped Ticks: $skipedTicks Average Sleep Time: $averageSleepTime ');
				// trace('Connections: ${Connection.getConnections().length} Tick: ${TimeHelper.tick} Time From Ticks: ${timeSinceStartCountedFromTicks} Time: ${Math.ceil(timeSinceStart)} Skiped Ticks: $skipedTicks Average Sleep Time: $averageSleepTime ');
				averageSleepTime = 0;
				skipedTicks = 0;

				if (ReadServerSettings) ServerSettings.readFromFile(false);

				// if(Server.server.connections.length > 0) Server.server.connections[0].player.doDeath();
			}

			@:privateAccess haxe.MainLoop.tick();

			Macro.exception(TimeHelper.DoTimeStuff());

			timeSinceStart = Sys.time() - TimeHelper.serverStartingTime;

			if (ServerSettings.UseOneGlobalMutex) Server.Release();

			if (timeSinceStartCountedFromTicks > timeSinceStart) {
				var sleepTime = timeSinceStartCountedFromTicks - timeSinceStart;
				averageSleepTime += sleepTime;

				// trace('sleep: ${sleepTime}');
				Sys.sleep(sleepTime);
			}
		}
	}

	public static function DoTimeStuff() {
		var timePassedInSeconds = CalculateTimeSinceTicksInSec(lastTick);

		TimeHelper.lastTick = tick;

		DoSeason(timePassedInSeconds);

		GlobalPlayerInstance.AcquireMutex();

		for (c in Connection.getConnections()) {
			Macro.exception(DoTimeStuffForPlayer(c.player, timePassedInSeconds));

			var sendMoveEveryXTicks = ServerSettings.SendMoveEveryXTicks;

			if (sendMoveEveryXTicks > 0
				&& TimeHelper.tick % sendMoveEveryXTicks == 0) Macro.exception(c.sendToMeAllClosePlayers(false, false));
			if (TimeHelper.tick % 20 == 0) {
				// send still alive PU as workaround to unstuck stuck client
				// if(c.player.isMoving() == false) c.send(PLAYER_UPDATE, [c.player.toData()]);
				// c.send(PLAYER_UPDATE, [c.player.toData()]);
			}
		}

		for (ai in Connection.getAis()) {
			if (ai.player == null) {
				Connection.removeAi(ai);
				continue;
			}
			Macro.exception(DoTimeStuffForPlayer(ai.player, timePassedInSeconds));
		}

		GlobalPlayerInstance.ReleaseMutex(); // TODO??? secure connections? since changing map stuff sends map updates to players

		Macro.exception(DoWorldMapTimeStuff()); // TODO currently it goes through the hole map each sec / this may later not work

		Macro.exception(RespawnObjects());

		Macro.exception(DoWorldLongTermTimeStuff());

		var worldMap = Server.server.map;

		// make sure they are not all at same tick!
		if ((tick + 20) % ServerSettings.TicksBetweenSaving == 0) Macro.exception(worldMap.updateObjectCounts());
		if (ServerSettings.saveToDisk
			&& tick % ServerSettings.TicksBetweenSaving == 0) Macro.exception(Server.server.map.writeToDisk(false));
		if (ServerSettings.saveToDisk
			&& (tick + 60) % ServerSettings.TicksBetweenBackups == Math.ceil(ServerSettings.TicksBetweenBackups / 2))
			Macro.exception(Server.server.map.writeBackup());

		DoTimeTestStuff();
	}

	private static function DoSeason(timePassedInSeconds:Float) {
		var passedSeasonTime = TimeHelper.CalculateTimeSinceTicksInSec(TimeSeasonStartedInTicks);
		var timeToNextSeasonInSec = TimeToNextSeasonInYears * 60;
		var tmpSeasonTemperatureImpact:Float = 0;

		// make eternal ice age
		// Season = Seasons.Winter;
		// SeasonHardness = 2;
		// passedSeasonTime = 0;

		tmpSeasonTemperatureImpact = ServerSettings.AverageSeasonTemperatureImpact * SeasonHardness;

		if (Season == Seasons.Spring || Season == Seasons.Autumn) tmpSeasonTemperatureImpact *= 0.25;
		if (Season == Seasons.Winter || Season == Seasons.Autumn) tmpSeasonTemperatureImpact *= -1;

		var factor = TimeToNextSeasonInYears * 15 * (1 / timePassedInSeconds);

		SeasonTemperatureImpact = (SeasonTemperatureImpact * factor + tmpSeasonTemperatureImpact) / (factor + 1);

		// if(tick % 20 == 0) trace('SEASON: ${SeasonHardness} TemperatureImpact: $SeasonTemperatureImpact tmp: $tmpSeasonTemperatureImpact');

		if (passedSeasonTime > timeToNextSeasonInSec) {
			var tmpSeasonHardness = SeasonHardness;

			TimeSeasonStartedInTicks = tick;
			TimeToNextSeasonInYears = ServerSettings.SeasonDuration / 2 + WorldMap.calculateRandomFloat() * ServerSettings.SeasonDuration;
			Season = (Season + 1) % 4;
			SeasonHardness = WorldMap.calculateRandomFloat() + 0.5;

			var seasonName = SeasonNames[Season];
			var message = 'SEASON: ${seasonName} is there! hardness: ${Math.round(SeasonHardness * 10) / 10} years: ${Math.round(passedSeasonTime / 60)} timeToNextSeasonInSec: $timeToNextSeasonInSec';

			trace(message);

			var hardSeason = (Season == Seasons.Winter || Season == Seasons.Summer) && SeasonHardness > 1.25;
			var hardText = hardSeason ? 'A hard ' : '';
			if (hardSeason && SeasonHardness > 1.4) {
				SeasonHardness += 0.1; // make it even harder
				hardText = 'A very hard ';
			}

			if (hardSeason) SeasonHardness = Math.pow(SeasonHardness, 2);

			// use same hardness for Spring as for winter. bad winter ==> good spring
			if (Season == Seasons.Spring) SeasonHardness = tmpSeasonHardness;

			TimeToNextSeasonInYears *= SeasonHardness;

			Connection.SendGlobalMessageToAll('$hardText ${seasonName} is comming!');

			TimeHelper.SeasonText = '$hardText ${seasonName}';
		}
	}

	private static function DoTimeStuffForPlayer(player:GlobalPlayerInstance, timePassedInSeconds:Float):Bool {
		if (player == null) return false;
		if (player.deleted) return false; // maybe remove?

		Macro.exception(player.connection.doTime(timePassedInSeconds));

		Macro.exception(UpdatePlayerStats(player, timePassedInSeconds));

		Macro.exception(updateAge(player, timePassedInSeconds));

		Macro.exception(updateFoodAndDoHealing(player, timePassedInSeconds));

		Macro.exception(MoveHelper.updateMovement(player));

		Macro.exception(DoTimeOnPlayerObjects(player, timePassedInSeconds));

		if (TimeHelper.tick % 20 == 0) Macro.exception(player.updateTemperature());

		if (TimeHelper.tick % 30 == 0) Macro.exception(UpdateEmotes(player));

		if (TimeHelper.tick % 40 == 0) Macro.exception(DoLeadership(player));

		if (TimeHelper.tick % 50 == 0) Macro.exception(DisplayStuff(player));

		player.connection.send(FRAME, null, false, true);

		return true;
	}

	private static function DisplayStuff(player:GlobalPlayerInstance) {
		if (player.isHuman() == false) return;
		player.connection.sendMapChunkIfNeeded(); // to update seasonal biomes
		player.connection.sendToMeAllFollowings(); // TODO check why following is not updated right seems to work after relogin

		if (player.account.displayClosePlayers) DisplayClosePlayers(player);
		if (player.age < 3) return;

		GlobalPlayerInstance.DisplayBestFood(player);

		if (player.hits > 1) AiHelper.DisplayCloseDeadlyAnimals(player, 10);

		// display seasons
		var nearDeath = player.food_store_max < 2;
		var timeSinceLastHint = TimeHelper.CalculateTimeSinceTicksInSec(player.timeLastTemperatureHint);
		var maxTimeSinceLastHint = nearDeath ? 10 : ServerSettings.DisplayTemperatureHintsPerMinute * 60;
		player.displaySeason = timeSinceLastHint > maxTimeSinceLastHint;

		if (player.displaySeason && player.isSuperHot() && nearDeath) {
			player.say('Need to drink water!', true);
			return;
		}

		if (player.displaySeason && player.isSuperCold() && nearDeath) {
			player.say('Need a fire!', true);
			return;
		}

		if (player.displaySeason && player.isSuperHot() && player.hits > 3) {
			player.timeLastTemperatureHint = TimeHelper.tick;
			// if(player.isIll() == false){
			// if(Season == Seasons.Summer && ) player.say('too hot ${SeasonNames[Season]}', true);
			// player.say('too hot need cooling!', true);
			// }

			var season = Season == Seasons.Summer ? ' ${SeasonNames[Season]}' : '';
			var rand = WorldMap.calculateRandomInt(2);

			if (rand == 0) player.say('too hot${season} a river could help!',
				true); else if (rand == 1) player.say('too hot${season} could drink some water!',
				true); else if (rand == 2) player.say('too hot${season} some snow would be nice!', true);
			// else if(rand == 2) player.say('too hot${season} a jungle could help', true);
			// else player.say('too hot${season} a desert would be warm!', true);
		} else if (player.displaySeason && player.isSuperCold() && player.hits > 3) {
			player.timeLastTemperatureHint = TimeHelper.tick;
			// if(Season == Seasons.Winter) player.say('its ${SeasonNames[Season]} i need to get warmer', true);

			var season = Season == Seasons.Winter ? ' ${SeasonNames[Season]}' : '';
			var rand = WorldMap.calculateRandomInt(3);

			if (rand == 0) player.say('too cold${season} need a fire!',
				true); else if (rand == 1) player.say('too cold${season} need more clothing!',
				true); else if (rand == 2) player.say('too cold${season} a jungle could help', true); else
				player.say('too cold${season} a desert would be warm!', true);
		}

		// if(player.isSuperHot() || player.isSuperCold()) player.displaySeason = false;
		// else player.displaySeason = true;
	}

	private static function DisplayClosePlayers(player:GlobalPlayerInstance) {
		if (ServerSettings.DisplayPlayerNamesDistance < 1) return;

		for (point in player.locationSaysPositions) {
			player.connection.send(ClientTag.LOCATION_SAYS, ['${point.x} ${point.y} ']);
		}

		player.locationSaysPositions = new Array<Point>();

		var count = 0;
		var maxDistance = ServerSettings.DisplayPlayerNamesDistance * ServerSettings.DisplayPlayerNamesDistance;
		for (p in GlobalPlayerInstance.AllPlayers) {
			var quadDist = AiHelper.CalculateDistanceToPlayer(player, p);

			if (quadDist < 64) continue;
			if (quadDist > maxDistance) continue;

			var name = player.mother == p ? 'MOTHER' : p.name;
			if (player.partner == p) name = 'PARTNER';
			if (player.father == p) name = 'FATHER';
			var rx = WorldMap.world.transformX(player, p.tx);
			var ry = WorldMap.world.transformY(player, p.ty);
			var dist = Math.round(Math.sqrt(quadDist));
			if (ServerSettings.DisplayPlayerNamesShowDistance && dist > 9) name += '_${dist}M';

			// player.connection.send(PLAYER_UPDATE, [p.toRelativeData(player)], false);
			// player.connection.send(PLAYER_SAYS, ['${p.id}/$0 $name']);

			player.connection.send(ClientTag.LOCATION_SAYS, ['${rx} ${ry} ${name}']);
			player.locationSaysPositions.push(new Point(rx, ry));

			count++;
			if (count >= ServerSettings.DisplayPlayerNamesMaxPlayer) break;
		}
	}

	private static function UpdatePlayerStats(player:GlobalPlayerInstance, timePassedInSeconds:Float) {
		if (player.jumpedTiles > 0) player.jumpedTiles -= timePassedInSeconds * ServerSettings.MaxJumpsPerTenSec * 0.1;
		if (player.lastSayInSec > 0) player.lastSayInSec -= timePassedInSeconds;

		if (player.isBlocked(player.tx, player.ty)) MoveHelper.JumpToNonBlocked(player);

		if (player.connection.sock != null && player.connection.serverAi != null && ServerSettings.AutoFollowAi == false) {
			player.connection.serverAi = null;
			trace('WARNING ${player.name + player.id} has socket and serverAi! serverAi set null!');
		}

		if (player.heldPlayer != null && player.heldPlayer.deleted) {
			player.heldPlayer = null;
			player.o_id = player.heldObject.toArray();
			trace('WARNING ${player.name + player.id} held player is dead! o_id set to: ${player.heldObject.name}');
		}
		if (player.heldByPlayer != null && player.heldByPlayer.deleted) {
			player.heldByPlayer = null;
			trace('WARNING ${player.name + player.id} heldByPlayer is dead!');
		}
		if (player.o_id[0] < 0 && player.heldPlayer == null) {
			player.o_id = player.heldObject.toArray();
			trace('WARNING ${player.name + player.id} ${player.o_id[0]} < 0 but no player held! o_id set to: ${player.heldObject.name}');
		}

		// if(player.angryTime < 0 && player.angryTime > -1) player.angryTime = 0;

		// var moreAngry = player.isHoldingWeapon() || (player.lastPlayerAttackedMe != null && player.lastPlayerAttackedMe.isHoldingWeapon());
		var moreAngry = player.killMode || (player.lastPlayerAttackedMe != null && player.lastPlayerAttackedMe.isHoldingWeapon());
		var minAngryTime = ServerSettings.CombatAngryTimeMinimum;

		if (moreAngry) {
			if (player.angryTime > minAngryTime) player.angryTime -= timePassedInSeconds; else
				player.angryTime = minAngryTime;
		} else {
			var biomeId = WorldMap.world.getBiomeId(player.tx, player.ty);
			var biomeFactor:Float = biomeId == PASSABLERIVER ? 2 : 1;
			biomeFactor = biomeId == DESERT ? 0.5 : biomeFactor;

			if (player.angryTime < ServerSettings.CombatAngryTimeBeforeAttack) player.angryTime += timePassedInSeconds * biomeFactor;
		}

		// if (player.lostCombatPrestige != 0) trace('${player.name + player.id} lostCombatPrestige: ${player.lostCombatPrestige}');

		if (player.angryTime >= 0 && player.lostCombatPrestige > 0 && player.darkNosaj < 1) {
			player.lostCombatPrestige -= (ServerSettings.CombatReputationRestorePerYear * timePassedInSeconds) / 60;
		}

		// if last attacker is far away set null
		if (player.lastPlayerAttackedMe != null) {
			var quadDist = AiHelper.CalculateDistanceToPlayer(player, player.lastPlayerAttackedMe);
			if (quadDist > 100) player.lastPlayerAttackedMe = null;
		}

		// add new follower
		if (player.newFollowerTime > 0) player.newFollowerTime -= timePassedInSeconds; else {
			if (player.newFollower != null) {
				var exileLeader = player.newFollower.getLeaderWhoExiled(player);
				var notExiled = exileLeader == null;

				if (notExiled && player.newFollower.followPlayer != player.newFollowerFor) {
					// player.newFollowerFor since also top leader got informed so player might be top leader
					var done = player.newFollower.setFollowPlayer(player.newFollowerFor);

					if (done) {
						Connection.SendFollowingToAll(player.newFollower);

						player.newFollower.say('I follow now ${player.name} ${player.familyName}', true);
						player.newFollower.connection.sendGlobalMessage('You follow now ${player.name} ${player.familyName}');

						// player.newFollower.say('now I follow ${player.newFollowerFor.name}');

						// player.say('He follows now ${player.newFollowerFor.name}');
					}
				}

				player.newFollowerFor.newFollower = null;
				player.newFollowerFor.newFollowerFor = null;
				player.newFollower = null;
				player.newFollowerFor = null;
			}
		}

		if (player.hasYellowFever()) {
			var isHeldFaktor = player.heldByPlayer != null ? 0.2 : 1; // if taken care its much less hard!

			player.food_store -= timePassedInSeconds * ServerSettings.ExhaustionYellowFeverPerSec * 2;
			// player.exhaustion += timePassedInSeconds * ServerSettings.ExhaustionYellowFeverPerSec * isHeldFaktor;
			// player.hits += timePassedInSeconds * ServerSettings.ExhaustionYellowFeverPerSec * 0.05 * isHeldFaktor;

			player.heat += timePassedInSeconds * 0.02 * isHeldFaktor;
			if (player.heat > 1) player.heat = 1;

			player.food_store_max = player.calculateFoodStoreMax();

			if (TimeHelper.tick % 20 == 0) {
				player.sendFoodUpdate(false);
				player.connection.send(FRAME, null, false);
			}
		}
	}

	private static function DoLeadership(player:GlobalPlayerInstance) {
		if (player.followPlayer == null) return;
		var leader = player.followPlayer;

		passExileToLeader(player, leader);
		passExileToLeader(player, player.getTopLeader());

		var myPower = player.countLeadershipPower();
		var leaderPower = leader.countLeadershipPower();
		var isMyFamily = leader.lineage.eve == player.lineage.eve;
		var factor = isMyFamily ? 1.2 : 2;

		if (player.age > 6 && myPower > leaderPower * factor) {
			trace('Leader change: ${player.name} myPower: ${myPower} ${leader.name} leaderPower: ${leaderPower} isMyFamily: ${isMyFamily}');

			player.followPlayer = leader.followPlayer;
			leader.followPlayer = player;

			Connection.SendFollowingToAll(player);
			Connection.SendFollowingToAll(leader);

			player.say('I am more powerful then ${leader.name} ${leader.familyName}!', true);

			// var text = isKing ? 'King' : 'Leader';
			var text = 'Leader';

			for (p in GlobalPlayerInstance.AllPlayers) {
				if (p == player) continue;
				if (p.getTopLeader(player) != player) continue;

				p.say('My new $text is ${player.name} ${player.familyName}!', true);
				p.connection.sendGlobalMessage('Long live the new $text ${player.name} ${player.familyName}!');
				// p.connection.sendMapLocation(bestLeader, "LEADER", "leader");
			}

			// player.connection.sendGlobalMessage('You follow now ${player.name} ${player.familyName}');
		}
	}

	private static function passExileToLeader(player:GlobalPlayerInstance, leader:GlobalPlayerInstance) {
		if (leader == null) return;

		var dist = player.calculateExactQuadDistanceToPlayer(leader);
		// if (dist < ServerSettings.MaxDistanceToAutoExileAttacker) {
		if (dist < 5) {
			for (p in GlobalPlayerInstance.AllPlayers) {
				if (p.isExiledBy(player) == false) continue;
				if (p.isExiledBy(leader)) continue;

				trace('EXILE: ${player.name} power: ${player.power} ${p.name} power: ${p.power}');
				// Only exile ally if player pwoer is heigher
				if (p.isAlly(leader) && p.power * 2 > player.power) continue;

				trace('EXILE: leader: ${leader.name} ${player.name} power: ${player.power} ${p.name} power: ${p.power}');

				leader.say('I EXILE ${p.name} BECAUSE I TRUST ${player.name}');
				break;
			}
		}
	}

	private static function DoTimeOnPlayerObjects(player:GlobalPlayerInstance, timePassedInSeconds:Float) {
		var obj = player.heldObject;

		// if(player.o_id[0] < 1) return;
		// if(obj.timeToChange <= 0) return;
		// obj.timeToChange -= timePassedInSeconds;

		// 2101 Cast Fishing Pole
		// if (obj.parentId == 2101) trace('Cast Fishing Pole: timeToChange: ${obj.timeToChange}');

		if (obj.timeToChange > 0 && obj.isTimeToChangeReached()) {
			var transition = TransitionImporter.GetTransition(-1, obj.parentId, false, false);

			if (transition != null) {
				// var desc = obj.objectData.description;
				// use alternative outcome for example for wound on player vs on ground
				var alternativeTimeOutcome = obj.objectData.alternativeTimeOutcome;
				obj.id = alternativeTimeOutcome >= 0 ? alternativeTimeOutcome : TransitionHelper.TransformTarget(transition.newTargetID);

				// trace('TIME: ${desc} --> ${obj.objectData.description} transition: ${transition.newTargetID} alternative: ${obj.objectData.alternativeTimeOutcome} neededTime: ${obj.timeToChange}');

				obj.creationTimeInTicks = TimeHelper.tick;

				player.setHeldObject(obj);

				player.setHeldObjectOriginNotValid(); // no animation

				TransitionHelper.DoChangeNumberOfUsesOnActor(player, transition);

				Connection.SendUpdateToAllClosePlayers(player, false, true);
			}
		}

		if (player.hiddenWound != null) {
			if (player.hiddenWound.isTimeToChangeReached()) {
				player.hiddenWound = null;
				player.doEmote(Emote.happy);
			} else {
				if (player.heldObject.id == 0) {
					player.setHeldObject(player.hiddenWound);
					player.setHeldObjectOriginNotValid(); // no animation
					Connection.SendUpdateToAllClosePlayers(player, false);
				}
			}

			if (player.hiddenWound != null && player.hiddenWound.id == 0) player.hiddenWound = null;
		}

		if (player.fever != null) {
			if (player.fever.isTimeToChangeReached()) {
				player.fever = null;
				player.doEmote(Emote.happy);
				player.connection.sendGlobalMessage('You survived yellow fever! Next time you will be more resistant...');
			}
		}

		// TODO contained objects
		// clothing decay / contained objects --> like in backpack
		for (i in 0...player.clothingObjects.length) {
			var obj = player.clothingObjects[i];

			var timeTransition = TransitionImporter.GetTransition(-1, obj.parentId, false, false);
			if (timeTransition == null) continue;

			if (obj.timeToChange <= 0) obj.timeToChange = timeTransition.calculateTimeToChange();

			if (obj.timeToChange > 0 && obj.isTimeToChangeReached()) {
				var name = obj.name;
				obj.id = timeTransition.newTargetID;

				trace('TIME: clothing ${name} --> ${obj.name}');
				obj.creationTimeInTicks = TimeHelper.tick;
				player.setInClothingSet(i);
			}
		}
	}

	private static function UpdateEmotes(player:GlobalPlayerInstance) {
		if (player.isWounded()) {
			Connection.SendEmoteToAll(player, Emote.shock);
			return;
		}

		// if(player.isHoldingWeapon() && player.angryTime < ServerSettings.CombatAngryTimeBeforeAttack / 2 )
		if (player.angryTime < 2) {
			if (player.isHoldingWeapon()) player.doEmote(Emote.murderFace); else {
				var lastPlayerAttackedMe = player.lastPlayerAttackedMe;
				if (lastPlayerAttackedMe != null
					&& lastPlayerAttackedMe.lastAttackedPlayer == player
					&& lastPlayerAttackedMe.isHoldingWeapon()) player.doEmote(Emote.terrified); else
					player.doEmote(Emote.angry);
			}

			return;
		}

		if (player.hasYellowFever()) {
			if (player.isSuperHot()) player.doEmote(Emote.heatStroke); else
				player.doEmote(Emote.yellowFever);
			return;
		}

		if (player.food_store < 0 && player.age >= ServerSettings.MinAgeToEat) {
			player.doEmote(Emote.starving);
			return;
		}

		if (player.angryTime < ServerSettings.CombatAngryTimeBeforeAttack) {
			if (player.isHoldingWeapon()) player.doEmote(Emote.angry); else {
				if (player.lastPlayerAttackedMe != null && player.lastPlayerAttackedMe.isHoldingWeapon()) player.doEmote(Emote.shock); else
					player.doEmote(Emote.angry);
			}
		}

		// if (player.isHoldingChildInBreastFeedingAgeAndCanFeed() && player.isSuperHot() == false && player.isSuperCold() == false) {
		if (player.isHoldingChildInBreastFeedingAgeAndCanFeed()) {
			// player.heldPlayer.doEmote(Emote.happy);
			// return;
		}

		var passedTime = CalculateTimeSinceTicksInSec(player.lastTimeEmoteSend);
		if (passedTime < 9) return;
		player.lastTimeEmoteSend = TimeHelper.tick;

		if (player.isSuperHot()) player.doEmote(Emote.heatStroke); //-2
		if (player.isSuperCold()) player.doEmote(Emote.pneumonia); //-2
		// else if(playerHeat > 0.6) player.doEmote(Emote.dehydration);

		if (player.mother != null) {
			// if(this.isAi() == false) this.connection.sendMapLocation(this.mother,'MOTHER', 'mother');
			// if(player.isAi() == false) player.connection.sendMapLocation(player.mother,'MOTHER', 'leader');
			// if(player.mother.isAi() == false) player.mother.connection.sendMapLocation(player,'BABY', 'baby');
		}
	}

	private static function updateAge(player:GlobalPlayerInstance, timePassedInSeconds:Float) {
		var tmpAge = player.age;
		var healthFactor = player.CalculateHealthAgeFactor();
		var ageingFactor:Float = 1;

		// trace('aging: ${aging}');
		// trace('player.age_r: ${player.age_r}');
		// trace('healthFactor: ${healthFactor}');

		if (player.age < ServerSettings.GrownUpAge) {
			// ageingFactor = healthFactor;
		} else {
			ageingFactor = 1 / healthFactor;
		}

		if (player.isHuman() && player.mother != null && player.mother.isAi() && player.age < ServerSettings.MinAgeToEat) {
			ageingFactor *= ServerSettings.AgingFactorHumanBornToAi;
			// if(TimeHelper.tick % 20 == 0) trace('ageing: human born to ai: $ageingFactor');
		} else if (player.isAi() && player.mother != null && player.mother.isHuman() && player.age < ServerSettings.MinAgeToEat) {
			ageingFactor *= ServerSettings.AgingFactorAiBornToHuman;
			// if(TimeHelper.tick % 20 == 0) trace('ageing: human born to ai: $ageingFactor');
		}

		if (player.food_store < 0) {
			if (player.age < ServerSettings.GrownUpAge) {
				ageingFactor *= ServerSettings.AgingFactorWhileStarvingToDeath;
			} else {
				ageingFactor *= 1 / ServerSettings.AgingFactorWhileStarvingToDeath;
			}
		}

		player.age_r = ServerSettings.AgeingSecondsPerYear / ageingFactor;
		var ageing = timePassedInSeconds / ServerSettings.AgeingSecondsPerYear;

		player.trueAge += ageing;

		ageing *= ageingFactor;

		player.age += ageing;

		// trace('player.age: ${player.age}');

		if (Std.int(tmpAge) != Std.int(player.age)) {
			if (ServerSettings.DebugPlayer)
				trace('Player: ${player.p_id} Old Age: $tmpAge New Age: ${player.age} TrueAge: ${player.trueAge} agingFactor: $ageingFactor healthFactor: $healthFactor');

			// player.yum_multiplier -= ServerSettings.MinHealthPerYear; // each year some health is lost
			player.food_store_max = player.calculateFoodStoreMax();

			// decay some coins per year
			if (Std.int(player.trueAge) % 10 == 0 && player.coins > 10) {
				var decayedCoins:Float = Std.int(player.coins / 10); // 1% per year
				var maxPrestigeFromCoins = player.prestige / 5;
				player.coins -= decayedCoins;

				maxPrestigeFromCoins = Math.max(maxPrestigeFromCoins, ServerSettings.MinPrestiegeFromCoinDecayPerYear * 10);
				decayedCoins = Math.min(decayedCoins, maxPrestigeFromCoins);
				player.addPrestige(decayedCoins);
				player.prestigeFromWealth += decayedCoins;
			}

			if (player.age > ServerSettings.MaxAge) player.doDeath('reason_age');

			// trace('update age: ${player.age} food_store_max: ${player.food_store_max}');
			player.sendFoodUpdate(false);

			// if (player.isMoving() == false) Connection.SendUpdateToAllClosePlayers(player, false);

			if (Std.int(player.trueAge) % 10 == 0) {
				var coins:Float = Std.int(player.coins);
				var text = coins >= 10 ? 'You have ${coins} coins! You can use: I give you IXC' : '';
				player.connection.sendGlobalMessage(text);
			}

			if (Std.int(player.trueAge) == 1) {
				var coins:Float = Std.int(player.coins);
				var text = 'Bad Temperature (bottom right) can kill you';
				var text2 = 'The max food (left corner) is also your health!';
				var text3 = 'Life is hard. Try to stay alife!';
				var text4 = 'Good luck!';
				player.connection.sendGlobalMessage(text);
				player.connection.sendGlobalMessage(text2);
				player.connection.sendGlobalMessage(text3);
				player.connection.sendGlobalMessage(text4);
			}

			if (Std.int(player.trueAge) == 5) {
				var father = player.father;
				if (player.followPlayer == player.mother && father != null && father.isDeleted() == false) {
					var rand = WorldMap.world.randomFloat();
					var chance = player.isMale() ? 0.4 : 0.8;
					if (rand > chance) {
						var done = player.setFollowPlayer(player.father);

						if (done) {
							var text = player.isMale() ? 'SON' : 'DAUGHTER';

							Connection.SendFollowingToAll(player);

							player.say('I FOLLOW MY FATHER!');
							father.say('MY $text ${player.name} FOLLOWS ME NOW!', true);

							player.connection.sendMapLocation(father, 'LEADER', 'leader');
							father.connection.sendMapLocation(player, 'FOLLOWER', 'follower');
							father.doEmote(Emote.hubba);
							player.doEmote(Emote.happy);
						}
					}
				}
			}

			if (Std.int(player.age) == 58) {
				// trace('Player: ${player.name + player.p_id} death is near!');

				var factor = ServerSettings.DisplayScoreFactor;
				var totalPrestige = Math.floor(player.yum_multiplier);
				var prestigeFromChildren = Math.floor(player.prestigeFromChildren);
				var prestigeFromGrandkids = Math.floor(player.prestigeFromGrandkids);
				var prestigeFromFollowers = Math.floor(player.prestigeFromFollowers);
				var prestigeFromEating = Math.floor(player.prestigeFromEating);
				var prestigeFromParents = Math.floor(player.prestigeFromParents);
				var prestigeFromSiblings = Math.floor(player.prestigeFromSiblings);

				var textFromChildren = prestigeFromChildren > 5 ? 'You have gained in total ${prestigeFromChildren * factor} prestige from children!' : '';
				var textFromGrandkids = prestigeFromGrandkids > 5 ? 'You have gained in total ${prestigeFromGrandkids * factor} prestige from grandkids!' : '';
				var textFromFollowers = prestigeFromFollowers > 5 ? 'You have gained in total ${prestigeFromFollowers * factor} prestige from followers!' : '';
				var textFromEating = prestigeFromEating > 5 ? 'You have gained in total ${prestigeFromEating * factor} prestige from YUMMY food!' : '';
				var textFromWealth = player.prestigeFromWealth > 5 ? 'You have gained ${player.prestigeFromWealth * factor} prestige from your wealth!' : '';
				var textFromParents = prestigeFromParents > 5 ? 'You have gained ${prestigeFromParents * factor} prestige from parents!' : '';
				var textFromSiblings = prestigeFromSiblings > 5 ? 'You have gained ${prestigeFromSiblings * factor} prestige from siblings!' : '';

				// trace('New Age: $message');
				player.connection.sendGlobalMessage('Your life nears the end. You earned ${totalPrestige} prestige!');

				if (ServerSettings.DisplayScoreOn) {
					player.connection.sendGlobalMessage(textFromChildren);
					player.connection.sendGlobalMessage(textFromGrandkids);
					player.connection.sendGlobalMessage(textFromFollowers);
					player.connection.sendGlobalMessage(textFromEating);
					player.connection.sendGlobalMessage(textFromWealth);
					player.connection.sendGlobalMessage(textFromParents);
					player.connection.sendGlobalMessage(textFromSiblings);
				}
			}

			ScoreEntry.ProcessScoreEntry(player);
		}
	}

	private static function updateFoodAndDoHealing(player:GlobalPlayerInstance, timePassedInSeconds:Float) {
		// trace('food_store: ${connection.player.food_store}');

		var tmpFood = Math.ceil(player.food_store);
		var tmpExtraFood = Math.ceil(player.yum_bonus);
		var tmpFoodStoreMax = Math.ceil(player.food_store_max);
		var originalFoodDecay = timePassedInSeconds * player.foodUsePerSecond; // depends on temperature
		var playerIsStarvingOrHasBadHeat = player.food_store < 0 || player.isSuperCold() || player.isSuperHot();
		var doHealing = playerIsStarvingOrHasBadHeat == false
			&& player.isWounded() == false
			&& player.hasYellowFever() == false
			&& player.angryTime > 0;
		var foodDecay = originalFoodDecay;
		var healing = timePassedInSeconds * ServerSettings.HealingPerSecond;
		// var healing = 1.5 * timePassedInSeconds * ServerSettings.FoodUsePerSecond - originalFoodDecay;

		// healing is between 0.5 and 2 of food decay depending on temperature
		// if (healing < timePassedInSeconds * ServerSettings.FoodUsePerSecond / 2) healing = timePassedInSeconds * ServerSettings.FoodUsePerSecond / 2;
		// if(tick % 20 == 0) trace('${player.id} heat: ${player.heat} faktor: ${healing / originalFoodDecay} healing: $healing foodDecay: $originalFoodDecay');

		if (player.age < ServerSettings.GrownUpAge && player.food_store > 0) foodDecay *= ServerSettings.FoodUseChildFaktor;

		if (player.isAi()) {
			if (player.lineage.prestigeClass == PrestigeClass.Serf) foodDecay *= ServerSettings.AIFoodUseFactorSerf; else
				if (player.lineage.prestigeClass == PrestigeClass.Commoner) foodDecay *= ServerSettings.AIFoodUseFactorCommoner; else
					if (player.lineage.prestigeClass == PrestigeClass.Noble) foodDecay *= ServerSettings.AIFoodUseFactorNoble;
		}

		// do damage if wound
		if (player.isWounded() || player.hiddenWound != null) {
			var wound = player.isWounded() ? player.heldObject : player.hiddenWound;
			var bleedingDamage = timePassedInSeconds * wound.objectData.damage * ServerSettings.WoundDamageFactor;
			player.hits += bleedingDamage;
			foodDecay += 2 * bleedingDamage;
			// player.exhaustion += bleedingDamage;
		}

		// do damage while starving
		if (player.food_store < 0) {
			player.hits += originalFoodDecay * 0.5;
		}

		// take care of exhaustion
		var exhaustionFoodNeed = 0.0;
		// if(healing > 0 && player.exhaustion > -player.food_store_max && player.food_store > 0)
		if (doHealing && player.exhaustion > -player.food_store_max) {
			var healingFaktor = player.isMale() ? ServerSettings.ExhaustionHealingForMaleFaktor : 1;
			// var exhaustionFaktor = player.exhaustion > player.food_store_max / 2 ? 2 : 1;
			var exhaustionFaktor:Float = 1;
			exhaustionFoodNeed = originalFoodDecay * ServerSettings.ExhaustionHealingFactor * exhaustionFaktor;

			player.exhaustion -= healing * ServerSettings.ExhaustionHealingFactor * healingFaktor * exhaustionFaktor;
			foodDecay += exhaustionFoodNeed;
		}

		// take damage if temperature is too hot or cold
		var damage:Float = 0;

		// give some time for BB to survive first year FIX: BB instand death in hard winter born outside
		if (player.age > 1) {
			if (player.isSuperHot()) damage = player.heat > 0.95 ? 2 * originalFoodDecay : originalFoodDecay; else if (player.isSuperCold())
				damage = player.heat < 0.05 ? 2 * originalFoodDecay : originalFoodDecay;

			player.hits += damage * ServerSettings.TemperatureHitsDamageFactor;
			player.exhaustion += damage * ServerSettings.TemperatureExhaustionDamageFactor;
		}

		// do Biome exhaustion
		// var tmpexhaustion = player.exhaustion;
		var biomeLoveFactor = player.biomeLoveFactor();
		if (biomeLoveFactor > 1) biomeLoveFactor = 1;
		// if(biomeLoveFactor < 0) player.exhaustion -= originalFoodDecay * biomeLoveFactor / 2; // gain exhaustion in wrong biome
		if (biomeLoveFactor > 0 && player.exhaustion > -player.food_store_max) player.exhaustion -= healing * 0.5 * biomeLoveFactor;
		// if (player.exhaustion > -player.food_store_max)
		//	trace('${player.name} Exhaustion: $tmpexhaustion ==> ${player.exhaustion} pID: ${player.p_id} biomeLoveFactor: $biomeLoveFactor');

		// do healing but increase food use

		// if (healing > 0 && player.hits > 0 && playerIsStarvingOrHasBadHeat == false && player.isWounded() == false) {
		if (doHealing && healing > 0 && player.hits > 0) {
			// var healingFaktor = doHealing ? 1.0 : 0.0;
			// var foodDecayFaktor = doHealing ? 2 : 1;

			player.hits -= healing * ServerSettings.WoundHealingFactor; // * healingFaktor;

			foodDecay += originalFoodDecay * ServerSettings.WoundHealingFactor; // * foodDecayFaktor;

			if (player.hits < 0) player.hits = 0;

			if (player.woundedBy != 0 && player.hits < 1) {
				player.woundedBy = 0;
				if (player.connection != null) player.connection.send(ClientTag.HEALED, ['${player.p_id}']);
			}
		}

		// if starving to death and there is some health left, reduce food need and health
		if (player.food_store < 0 && player.yum_multiplier > 1) {
			player.yum_multiplier -= foodDecay;
			foodDecay /= 2;
		}

		// do breast feeding
		var heldPlayer = player.heldPlayer;

		if (player.isHoldingChildInBreastFeedingAgeAndCanFeed()) {
			// trace('feeding:');

			if (heldPlayer.food_store < heldPlayer.getMaxChildFeeding()) {
				var food = ServerSettings.FoodRestoreFactorWhileFeeding * timePassedInSeconds * ServerSettings.FoodUsePerSecond;
				var tmpFood = Math.ceil(heldPlayer.food_store);

				heldPlayer.food_store += food;
				foodDecay += food / 2;

				// if (heldPlayer.hits > 0) heldPlayer.hits -= timePassedInSeconds * 0.2;

				var hasChanged = tmpFood != Math.ceil(heldPlayer.food_store);
				if (hasChanged) {
					heldPlayer.sendFoodUpdate(false);
					heldPlayer.connection.send(FRAME, null, false);
				}

				// trace('feeding: $food foodDecay: $foodDecay');
			}

			if (heldPlayer.hits > 0) {
				heldPlayer.hits -= timePassedInSeconds * 0.2;
			}
		}

		foodDecay *= player.isEveOrAdam() && player.isWounded() == false ? ServerSettings.EveFoodUseFactor : 1;

		if (player.yum_bonus > 0) {
			player.yum_bonus -= foodDecay;
		} else {
			player.food_store -= foodDecay;
		}

		// if (TimeHelper.tick % 40 == 0) trace('${player.name + player.id} FoodDecay: ${Math.round(foodDecay / timePassedInSeconds * 100) / 100} org: ${Math.round(originalFoodDecay / timePassedInSeconds * 100) / 100)} fromexh: ${Math.round(exhaustionFoodNeed / timePassedInSeconds * 100) / 100}');
		if (ServerSettings.DebugPlayer && TimeHelper.tick % 40 == 0)
			trace('${player.name + player.id} FoodDecay: ${Math.round(foodDecay / timePassedInSeconds * 100) / 100} org: ${Math.round(originalFoodDecay / timePassedInSeconds * 100) / 100)} fromexh: ${Math.round(exhaustionFoodNeed / timePassedInSeconds * 100) / 100}');

		player.food_store_max = player.calculateFoodStoreMax();

		var hasChanged = tmpFood != Math.ceil(player.food_store) || tmpExtraFood != Math.ceil(player.yum_bonus);
		hasChanged = hasChanged || tmpFoodStoreMax != Math.ceil(player.food_store_max);

		if (hasChanged) {
			player.sendFoodUpdate(false);
			player.connection.send(FRAME, null, false);
		}

		if (player.food_store_max < ServerSettings.DeathWithFoodStoreMax) {
			var reason = player.woundedBy == 0 ? 'reason_hunger' : 'reason_killed_${player.woundedBy}';

			player.doDeath(reason);

			Connection.SendUpdateToAllClosePlayers(player, false);
		}
	}

	public static function ClearCursedGraves(stuff:Map<Int, ObjectHelper>) {
		var oldStuff = [for (obj in stuff) obj];
		var newStuff = new Map<Int, ObjectHelper>();

		for (obj in oldStuff) {
			var objData = WorldMap.world.getObjectDataAtPosition(obj.tx, obj.ty);

			if (objData.isBoneGrave()) {
				var index = WorldMap.world.index(obj.tx, obj.ty);
				newStuff[index] = obj;
			}
		}

		return newStuff;
	}

	public static function DoWorldMapTimeStuff() {
		// devide in X steps
		var timeParts = ServerSettings.WorldTimeParts;
		var worldMap = Server.server.map;

		var partSizeY = Std.int(worldMap.height / timeParts);
		var startY = (worldMapTimeStep % timeParts) * partSizeY;
		var endY = startY + partSizeY;

		// trace('startY: $startY endY: $endY worldMap.height: ${worldMap.height}');

		if (tick % 2000 == 0) {
			WorldMap.world.cursedGraves = ClearCursedGraves(WorldMap.world.cursedGraves);

			// clear ovens, so that old ones go away
			var ovens = [for (obj in WorldMap.world.ovens) obj];
			var newovens = new Map<Int, ObjectHelper>();

			for (oven in ovens) {
				var objData = WorldMap.world.getObjectDataAtPosition(oven.tx, oven.ty);
				// 237 Adobe Oven // 753 Adobe Rubble
				if (ObjectData.IsOven(objData.id)) { // || objData.id == 753){
					var index = WorldMap.world.index(oven.tx, oven.ty);
					newovens[index] = oven;
				}
			}

			WorldMap.world.ovens = newovens;
		}

		if (worldMapTimeStep % timeParts == 0) {
			if (TimeTimeStepsSartedInTicks > 0) TimePassedToDoAllTimeSteps = TimeHelper.CalculateTimeSinceTicksInSec(TimeTimeStepsSartedInTicks);

			// trace('DOTIME: started: $TimeTimeStepsSartedInTicks passed: $TimePassedToDoAllTimeSteps');

			WinterDecayChance = TimePassedToDoAllTimeSteps * ServerSettings.WinterWildFoodDecayChance / (TimeToNextSeasonInYears * 60);
			SpringRegrowChance = TimePassedToDoAllTimeSteps * ServerSettings.SpringWildFoodRegrowChance / (TimeToNextSeasonInYears * 60);

			WinterDecayChance *= SeasonHardness;
			SpringRegrowChance *= SeasonHardness;

			TimeTimeStepsSartedInTicks = tick;

			// trace('DOTIME: winterDecayChance: $winterDecayChance springRegrowChance: $springRegrowChance');
		}

		worldMapTimeStep++;

		for (y in startY...endY) {
			for (x in 0...worldMap.width) {
				if (Season == Seasons.Spring) {
					var hiddenObj = worldMap.getHiddenObjectId(x, y);
					if (hiddenObj[0] != 0) RespawnOrDecayPlant(hiddenObj, x, y, true);

					var originalObj = worldMap.getOriginalObjectId(x, y);
					if (originalObj[0] != 0) RespawnOrDecayPlant(originalObj, x, y, false, true);
				}

				var obj = worldMap.getObjectId(x, y);

				if (obj[0] == 0) continue;

				DoSecondTimeOutcome(x, y, obj[0], TimePassedToDoAllTimeSteps);

				DoItemInWaterMovement(x, y, obj[0], TimePassedToDoAllTimeSteps);

				var biome = worldMap.getBiomeId(x, y);

				// TODO move in long time processing
				// add possible spawn localtions
				if (obj[0] == 30) WorldMap.world.berryBushes[WorldMap.world.index(x,
					y)] = worldMap.getObjectHelper(x, y); // Wild Gooseberry Bush ==> possible spawn location
				if (obj[0] == 2142) WorldMap.world.bananaPlants[WorldMap.world.index(x,
					y)] = worldMap.getObjectHelper(x, y); // Banana Plant ==> possible spawn location
				if (obj[0] == 36 && biome == BiomeTag.SNOW) WorldMap.world.wildCarrots[WorldMap.world.index(x,
					y)] = worldMap.getObjectHelper(x, y); // Seeding Wild Carrot ==> possible spawn location
				if (obj[0] == 761 && biome == BiomeTag.DESERT) WorldMap.world.cactuses[WorldMap.world.index(x,
					y)] = worldMap.getObjectHelper(x, y); // Barrel Cactus ==> possible spawn location
				if (obj[0] == 4251 && biome == BiomeTag.GREY) WorldMap.world.wildGarlics[WorldMap.world.index(x,
					y)] = worldMap.getObjectHelper(x, y); // Wild Garlic ==> possible spawn location

				// get possible teleport locations
				// 237 Adobe Oven // 753 Adobe Rubble
				if (ObjectData.IsOven(obj[0])) WorldMap.world.ovens[WorldMap.world.index(x, y)] = worldMap.getObjectHelper(x, y);

				if (ObjectData.IsBoneGrave(obj[0])) WorldMap.world.cursedGraves[WorldMap.world.index(x, y)] = worldMap.getObjectHelper(x, y);

				var floorId = worldMap.getFloorId(x, y);
				// 1596 = Stone Road
				if (floorId == 1596) WorldMap.world.roads[WorldMap.world.index(x, y)] = worldMap.getObjectHelper(x, y);

				RespawnOrDecayPlant(obj, x, y);

				var helper = worldMap.getObjectHelper(x, y, true);

				/*if (helper == null && obj.length > 1) {
					trace('TIME: In Container!: ${obj}');
					// TODO only id needed for time stuff
					helper = worldMap.getObjectHelper(x, y);
				}*/

				if (helper != null) {
					if (helper.timeToChange == 0) // maybe timeToChange was forgotten to be set
					{
						var timeTransition = TransitionImporter.GetTransition(-1, helper.id, false, false);

						if (timeTransition != null) {
							trace('WARNING: found helper without time transition: ${helper.description}: ' + timeTransition.getDescription());
							helper.timeToChange = timeTransition.calculateTimeToChange();
						}
					}

					// clear up not needed ObjectHelpers to save space
					if (worldMap.deleteObjectHelperIfUseless(helper)) continue; // uses worlmap mutex

					if (helper.timeToChange > 0) TimeHelper.doTimeTransition(helper);

					// do time transition for contained objects
					// TODO time in contained objects in contained objects
					// TODO use mutex in case something changes???
					var changed = false;
					for (obj in helper.containedObjects) {
						changed = changed || TimeHelper.doTimeForObject(obj);
					}

					if (changed) {
						// clear up decayed objects
						var containedObjects = helper.containedObjects;
						var newContainedObjects = [];
						for (obj in containedObjects) {
							if (obj.id < 1) continue;
							newContainedObjects.push(obj);
						}
						helper.containedObjects = newContainedObjects;
						WorldMap.world.setObjectHelper(x, y, helper);
						Connection.SendMapUpdateToAllClosePlayers(x, y);
					}
					continue;
				}

				var timeTransition = TransitionImporter.GetTransition(-1, obj[0], false, false);
				if (timeTransition == null) continue;

				helper = worldMap.getObjectHelper(x, y);
				helper.timeToChange = timeTransition.calculateTimeToChange();

				worldMap.setObjectHelper(x, y, helper);

				// trace('TIME: ${helper.objectData.description} neededTime: ${timeToChange}');

				// var testObj = getObjectId(helper.tx, helper.ty);

				// trace('testObj: $testObj obj: $obj ${helper.tx},${helper.ty} i:$i index:${index(helper.tx, helper.ty)}');
			}
		}
	}

	private static function DoItemInWaterMovement(tx:Int, ty:Int, objId, timepassed:Float) {
		var world = WorldMap.world;
		var objData = ObjectData.getObjectData(objId);
		if (objData.dummyParent != null) objData = objData.dummyParent;

		if (objData.isPermanent()) return;
		var biomeId = world.getBiomeId(tx, ty);

		if (biomeId != PASSABLERIVER && biomeId != OCEAN && biomeId != RIVER) return;

		var floorId = world.getFloorId(tx, ty);
		if (floorId > 0) return;

		var rand = world.randomFloat();
		timepassed *= Math.pow(objData.speedMult, 3);
		if (biomeId == PASSABLERIVER) timepassed *= 0.2;
		if (biomeId == OCEAN) timepassed *= 0.5;
		if (biomeId == RIVER) timepassed *= 1.5;
		// trace('Water:1 ${objData.name} biomeId: ${biomeId} floorId: ${floorId} rand: ${rand} timepassed: ${timepassed}');

		if (rand > timepassed) return;

		var obj = world.getObjectHelper(tx, ty);
		var ttx = tx + world.randomInt(2) - 1;
		var tty = ty + world.randomInt(2) - 1;

		// trace('Water:2 ${objData.name} biomeId: ${biomeId} floorId: ${floorId} rand: ${rand} --> ${world.getObjectHelper(ttx, tty).name}');

		var objId = world.getObjectId(ttx, tty);
		if (objId[0] > 0) return;

		world.setObjectHelper(ttx, tty, obj);
		world.setObjectHelper(tx, ty, obj.groundObject);

		var ground = obj.groundObject == null ? [0] : obj.groundObject.toArray();
		obj.groundObject = null;

		Connection.SendAnimalMoveUpdateToAllClosePlayers(tx, ty, ttx, tty, ground, obj.toArray(), 1);

		// trace('Water:3 ${objData.name} biomeId: ${biomeId} floorId: ${floorId} rand: ${rand} --> ${world.getObjectHelper(ttx, tty).name}');
	}

	private static function DoSecondTimeOutcome(tx:Int, ty:Int, objId, timepassed:Float) {
		var objData = ObjectData.getObjectData(objId);
		if (objData.secondTimeOutcome < 1 || objData.secondTimeOutcomeTimeToChange < 1) return;

		if (timepassed / objData.secondTimeOutcomeTimeToChange < WorldMap.calculateRandomFloat()) return;

		var obj = WorldMap.world.getObjectHelper(tx, ty);

		/*if(obj == null)
			{
				obj = re
				var ids = [objData.secondTimeOutcome];
				WorldMap.world.setObjectId(tx,ty,ids);
				Connection.SendMapUpdateToAllClosePlayers(tx,ty, ids);
				return;
		}*/

		obj.id = objData.secondTimeOutcome;
		WorldMap.world.setObjectHelper(tx, ty, obj);
		Connection.SendMapUpdateToAllClosePlayers(tx, ty);
	}

	private static function RespawnOrDecayPlant(objIDs:Array<Int>, x:Int, y:Int, hidden:Bool = false, fromOriginals:Bool = false) {
		var objID = objIDs[0];
		var objData = ObjectData.getObjectData(objID);

		// Season = Seasons.Spring;

		if (Season == Seasons.Winter && hidden == false && objData.winterDecayFactor > 0) {
			// reduce uses if it is for example a berry bush
			if (objData.numUses > 1) {
				var objHelper = WorldMap.world.getObjectHelper(x, y);

				if (WinterDecayChance * objData.winterDecayFactor * objHelper.numberOfUses < WorldMap.calculateRandomFloat()) return;

				WorldMap.world.mutex.acquire(); // TODO try catch // TODO object helper may have changed

				objHelper.numberOfUses -= 1;
				objHelper.TransformToDummy();
				WorldMap.world.setObjectHelper(x, y, objHelper);

				Connection.SendMapUpdateToAllClosePlayers(x, y);

				WorldMap.world.mutex.release();

				return;
			}

			if (WinterDecayChance * objData.winterDecayFactor < WorldMap.calculateRandomFloat()) return;

			var random = WorldMap.calculateRandomFloat();

			WorldMap.world.mutex.acquire(); // TODO try catch

			WorldMap.world.setObjectId(x, y, [0]);
			if (objData.springRegrowFactor > random) WorldMap.world.setHiddenObjectId(x, y, objIDs); // TODO hide also object helper for advanced objects???

			Connection.SendMapUpdateToAllClosePlayers(x, y);

			WorldMap.world.currentObjectsCount[objID]--;

			WorldMap.world.mutex.release(); // TODO try catch

			if (ServerSettings.DebugSeason) {
				var mod = WorldMap.world.currentObjectsCount[objID] < 1000 ? 100 : 1000;
				mod = WorldMap.world.currentObjectsCount[objID] < 100 ? 10 : mod;
				mod = WorldMap.world.currentObjectsCount[objID] < 10 ? 1 : mod;
				if (WorldMap.world.currentObjectsCount[objID] % mod == 0)
					trace('SEASON DECAY: ${objData.description} ${WorldMap.world.currentObjectsCount[objID]} original: ${WorldMap.world.originalObjectsCount[objID]}');
			}
		} else if (Season == Seasons.Spring && objData.springRegrowFactor > 0) {
			// TODO regrow also from originalObjects?

			// increase uses if it is for example a berry bush
			if (objData.numUses > 1 || objData.undoLastUseObject != 0) {
				var objHelper = WorldMap.world.getObjectHelper(x, y);
				if (objHelper.numberOfUses >= objData.numUses && objData.undoLastUseObject == 0) return;

				var factor = objData.numUses - objHelper.numberOfUses;
				if (factor < 1) factor = 1;

				if (SpringRegrowChance * objData.springRegrowFactor * factor < WorldMap.calculateRandomFloat()) return;

				WorldMap.world.mutex.acquire();
				Macro.exception(IncreaseNumberOfUses(objHelper));

				Connection.SendMapUpdateToAllClosePlayers(x, y);

				WorldMap.world.mutex.release();

				return;
			}

			// hidden objects should come to surface
			// GrowNewPlantsFromExistingFactor ==> offsprings per season per plant
			// GrowBackOriginalPlantsFactor ==> regrow from original
			var factor = hidden ? 2 : ServerSettings.GrowNewPlantsFromExistingFactor;
			if (fromOriginals) factor = ServerSettings.GrowBackOriginalPlantsFactor;

			var spawnAs = objData.countsOrGrowsAs > 0 ? objData.countsOrGrowsAs : objData.parentId;
			var currentCount = WorldMap.world.currentObjectsCount[spawnAs];
			var originalCount = WorldMap.world.originalObjectsCount[spawnAs];

			// Dont make new offsprings if too high population, except from original to spread at possible empty regions
			if (fromOriginals == false && currentCount >= originalCount) return;
			// make more offsprings if population is low
			if (currentCount < originalCount / 2) factor *= ServerSettings.GrowBackPlantsIncreaseIfLowPopulation;

			if (SpringRegrowChance * objData.springRegrowFactor * factor < WorldMap.calculateRandomFloat()) return;

			WorldMap.world.mutex.acquire();
			Macro.exception(RegrowObj(x, y, spawnAs, hidden));
			WorldMap.world.mutex.release();

			currentCount += 1;
			WorldMap.world.currentObjectsCount[spawnAs] = currentCount;

			if (ServerSettings.DebugSeason) {
				var mod = currentCount < 1000 ? 100 : 1000;
				mod = currentCount < 100 ? 10 : mod;
				mod = currentCount < 10 ? 1 : mod;
				if (currentCount % mod == 0) trace('SEASON REGROW: ${objData.name} ${currentCount} original: ${originalCount} spawnAs: $spawnAs');
			}
		}
	}

	private static function IncreaseNumberOfUses(objHelper:ObjectHelper) {
		objHelper.numberOfUses += 1;
		objHelper.TransformToDummy();
		WorldMap.world.setObjectHelper(objHelper.tx, objHelper.ty, objHelper);
	}

	private static function RegrowObj(tx:Int, ty:Int, spawnAs:Int, hidden:Bool):Bool {
		var floorId = WorldMap.world.getFloorId(tx, ty);
		if (floorId > 0) return false;
		var done = SpawnObject(tx, ty, spawnAs);
		if (hidden && done) WorldMap.world.setHiddenObjectId(tx, ty, [0]); // What was hidden comes back
		return done;
	}

	public static function RespawnObjects() {
		var timeParts = ServerSettings.WorldTimeParts * 10;
		var worldMap = Server.server.map;
		var partSizeY = Std.int(worldMap.height / timeParts);
		var startY = (worldMapTimeStep % timeParts) * partSizeY;
		var endY = startY + partSizeY;

		// trace('startY: $startY endY: $endY worldMap.height: ${worldMap.height}');

		for (y in startY...endY) {
			for (x in 0...worldMap.width) {
				var objID = worldMap.getOriginalObjectId(x, y)[0];

				if (objID == 0) continue;

				if (ServerSettings.CanObjectRespawn(objID) == false) continue;

				if (ServerSettings.ObjRespawnChance < worldMap.randomFloat()) continue;

				WorldMap.world.mutex.acquire();

				Macro.exception(SpawnObject(x, y, objID));

				WorldMap.world.mutex.release();

				// trace('respawn object: ${objData.description} $obj');
			}
		}
	}

	public static function SpawnObject(x:Int, y:Int, objID:Int, dist:Int = 6, tries:Int = 3):Bool {
		var world = WorldMap.world;
		var objData = ObjectData.getObjectData(objID);
		var spawnAs = objData.countsOrGrowsAs > 0 ? objData.countsOrGrowsAs : objData.parentId;
		var currentCount = WorldMap.world.currentObjectsCount[spawnAs];
		var originalCount = WorldMap.world.originalObjectsCount[spawnAs];

		if (currentCount >= originalCount) return false;
		if (currentCount / (originalCount + 1) > world.randomFloat()) return false;

		for (ii in 0...tries) {
			var tmpX = world.randomInt(2 * dist) - dist + x;
			var tmpY = world.randomInt(2 * dist) - dist + y;

			if (world.getObjectId(tmpX, tmpY)[0] != 0) continue;
			if (world.getObjectId(tmpX, tmpY - 1)[0] != 0) continue; // make sure that obj does not spawn above one tile of existing obj
			if (world.getFloorId(tmpX, tmpY) != 0) continue; // dont spawn on floors

			var biomeId = world.getBiomeId(tmpX, tmpY);
			var objData = ObjectData.getObjectData(objID);

			if (objData.isSpawningIn(biomeId) == false) continue;

			world.setObjectId(tmpX, tmpY, [objID]);

			world.currentObjectsCount[spawnAs]++;

			Connection.SendMapUpdateToAllClosePlayers(tmpX, tmpY);

			return true;
		}

		return false;
	}

	public static function DoWorldLongTermTimeStuff() {
		// TODO dont decay if some on is close?
		// TODO decay stuff in containers
		// TODO decay stuff with number of uses > 1
		// TODO set custom objDecayTo
		// TODO add decay object so that decay is visible ==> 618 Filled Small Trash Pit

		var timeParts = ServerSettings.WorldTimeParts * 10;
		var worldMap = Server.server.map;
		var partSizeY = Std.int(worldMap.height / timeParts);
		var startY = (worldMapTimeStep % timeParts) * partSizeY;
		var endY = startY + partSizeY;

		// trace('DOLONGTIME: $worldMapTimeStep from $timeParts');

		if (worldMapTimeStep % timeParts == 0) {
			if (LongTimeTimeStepsSartedInTicks > 0) LongTimePassedToDoAllTimeSteps = TimeHelper.CalculateTimeSinceTicksInSec(LongTimeTimeStepsSartedInTicks);

			trace('DOLONGTIME: $tick started: $LongTimeTimeStepsSartedInTicks passed: ${LongTimePassedToDoAllTimeSteps}');

			LongTimeTimeStepsSartedInTicks = tick;
		}

		// var timePassedInSec = LongTimeTimeStepsSartedInTicks;
		var season = TimeHelper.Season;
		var timePassedInYears = LongTimePassedToDoAllTimeSteps / 60;

		// trace('startY: $startY endY: $endY worldMap.height: ${worldMap.height}');

		for (y in startY...endY) {
			for (x in 0...worldMap.width) {
				var objId = worldMap.getObjectId(x, y)[0];
				var floorId = worldMap.getFloorId(x, y);

				DoSeasonalBiomeChanges(x, y, timePassedInYears);

				if (objId == 0 && floorId == 0 && season == Spring) DoRespawnFromOriginal(x, y, timePassedInYears);

				if (season == Spring) DoSpringStuff(x, y, timePassedInYears);

				if (floorId != 0) DecayFloor(x, y, timePassedInYears);

				if (objId != 0) DecayObject(x, y, timePassedInYears);

				if (objId != 0) AlignWalls(x, y);

				if (objId != 0) ClearHeldObjectOnground(x, y, objId);
			}
		}
	}

	// Horse-Drawn Cart 778 // Horse-Drawn Tire Cart 3158
	// Shorn Sheep on Rope 3934 // Domestic Sheep on Rope 3926
	private static var objectIdsToUnstuck = [778, 3158, 3934, 3926, 3926];

	private static function ClearHeldObjectOnground(tx:Int, ty:Int, objId:Int) {
		var world = WorldMap.world;

		// transform placed object back to a not held one in case its a held one like a horse cart
		var trans = TransitionImporter.GetTransition(objId, -1);
		if (trans == null) return false;

		// Rope 59 --> like domestic sheep on rope
		if (objectIdsToUnstuck.contains(objId) == false && trans.newActorID != 59) return false;

		var obj = world.getObjectHelper(tx, ty);
		obj.id = trans.newTargetID;
		world.setObjectHelper(tx, ty, obj);

		if (trans.newActorID > 0) WorldMap.PlaceObject(tx, ty, new ObjectHelper(null, trans.newActorID));

		trace('WARNING: ClearHeldObjectOnground ${obj.name} ${obj.parentId}');
		return true;
	}

	private static function AlignWalls(tx:Int, ty:Int) {
		var world = WorldMap.world;
		var objData = world.getObjectDataAtPosition(tx, ty);
		// if (objData.rValue > 0) trace('AlignWalls1: ${objData.name} rValue: ${objData.rValue}');
		// if (objData.name.indexOf('WALL') != -1) trace('AlignWalls1: ${objData.name} rValue: ${objData.rValue} cloth: ${objData.isClothing()}');

		if (objData.isWall() == false) return;
		// trace('AlignWalls2: ${objData.name} cloth: ${objData.clothing}');

		// 885 Stone Wall (corner)
		// 886 Stone Wall (vertical)
		// 887 Stone Wall (horizontal)

		// 895 Ancient Stone Wall (corner)
		// 897 Ancient Stone Wall (vertical)
		// 896 Ancient Stone Wall (horizontal)

		// 154 Adobe Wall (corner)
		// 156 Adobe Wall (vertical)
		// 155 Adobe Wall (horizontal)

		// 1883 Plaster Wall (corner)
		// 1884 Plaster Wall (vertical)
		// 1885 Plaster Wall (horizontal)

		// 111 Pine Wall (corner)
		// 113 Pine Wall (vertical)
		// 112 Pine Wall (horizontal)

		// 3266 Snow Wall (corner)
		// 3267 Snow Wall (vertical)
		// 3268 Snow Wall (horizontal)

		// 551 Fence (corner)
		// 549 Fence (vertical)
		// 550 Fence (horizontal)

		AlignWall(tx, ty, objData, [154, 156, 155]); // Adobe Wall
		AlignWall(tx, ty, objData, [1883, 1884, 1885]); // Plaster Wall
		AlignWall(tx, ty, objData, [111, 113, 112]); // Pine Wall
		AlignWall(tx, ty, objData, [3266, 3267, 3268]); // Snow Wall
		AlignWall(tx, ty, objData, [885, 886, 887]); // Stone Wall
		AlignWall(tx, ty, objData, [895, 897, 896]); // Ancient Stone Wall
		AlignWall(tx, ty, objData, [551, 549, 550]); // Fence

		// TODO different colors // walls with containers
	}

	private static function AlignWall(tx:Int, ty:Int, objData:ObjectData, walls:Array<Int>) {
		if (walls.length < 3) return;

		if (walls.contains(objData.parentId) == false) return;

		var world = WorldMap.world;
		var objDataLeft = world.getObjectDataAtPosition(tx - 1, ty);
		var objDataRight = world.getObjectDataAtPosition(tx + 1, ty);
		var objDataNorth = world.getObjectDataAtPosition(tx, ty + 1);
		var objDataSouth = world.getObjectDataAtPosition(tx, ty - 1);
		var isHorizontal = objDataLeft.isWall() && objDataRight.isWall() && objDataNorth.isWall() == false && objDataSouth.isWall() == false;
		var isVertical = objDataLeft.isWall() == false && objDataRight.isWall() == false && objDataNorth.isWall() && objDataSouth.isWall();
		var isCorner = isHorizontal == false && isVertical == false;

		if (isCorner && objData.parentId != walls[0]) {
			var obj = world.getObjectHelper(tx, ty);
			obj.id = walls[0];
			world.setObjectHelper(tx, ty, obj);
			trace('WALL: ${objData.description} ${objData.parentId} --> Corner');
		} else if (isVertical && objData.parentId != walls[1]) {
			var obj = world.getObjectHelper(tx, ty);
			obj.id = walls[1];
			world.setObjectHelper(tx, ty, obj);
			trace('WALL: ${objData.description} ${objData.parentId} --> Vertical');
		} else if (isHorizontal && objData.parentId != walls[2]) {
			var obj = world.getObjectHelper(tx, ty);
			obj.id = walls[2];
			world.setObjectHelper(tx, ty, obj);
			trace('WALL: ${objData.description} ${objData.parentId} --> Horizontal');
		}
	}

	private static function DecayFloor(x:Int, y:Int, passedTimeInYears:Float) {
		var world = WorldMap.world;
		var objId = world.getObjectId(x, y)[0];
		var floorId = world.getFloorId(x, y);

		if (floorId == 0) return;
		// var objData = world.getObjectDataAtPosition(x,y);
		// if(objData.isPermanent()) return; // TODO allow floor decay if objId is permanent / maybe decay neighbor floor first?

		var biomeId = world.getBiomeId(x, y);
		var biomeDecayFactor:Float = Biome.getBiomeDecayFactor(biomeId);

		var objData = ObjectData.getObjectData(floorId);
		var decayChance = ServerSettings.FloorDecayChance * objData.decayFactor;
		decayChance *= biomeDecayFactor;

		var wallStrength = ObjectHelper.CalculateSurroundingWallStrength(x, y);
		var floorStrength = ObjectHelper.CalculateSurroundingFloorStrength(x, y);
		var totalStrength = wallStrength + floorStrength;
		if (totalStrength < 1) totalStrength = 1;
		// 20% for a floor souranded by 4 floors
		// 11% for a floor souranded by 4 Walls (0%)
		// 50% for a floor souranded by 1 floor
		// 33% for a floor souranded by 1 Wall

		// 14% for a floor souranded by 4 Floor and 1 Wall (0%)
		// TODO allow some floor decay inside
		var strengthFactor = totalStrength > 5 ? 0 : 1 / totalStrength;
		decayChance *= strengthFactor;

		// trace('Floor ${objData.name} try decayed chance: ${decayChance * 10000} strength: $totalStrength w: $wallStrength  f: $floorStrength strengthFactor: $strengthFactor');

		if (world.randomFloat() > decayChance) return;

		var decaysToObj = objData.decaysToObj == 0 ? 618 : objData.decaysToObj; // 618 Filled Small Trash Pit
		var decaysToObjData = ObjectData.getObjectData(decaysToObj);

		if (decaysToObjData.floor) world.setFloorId(x, y, decaysToObj); else {
			world.setFloorId(x, y, 0);
			if (objId == 0) world.setObjectId(x, y, [decaysToObj]);
		}

		Connection.SendMapUpdateToAllClosePlayers(x, y);

		trace('Floor ${objData.name} decayed to: ${decaysToObjData.name} chance: $decayChance strength: $totalStrength w: $wallStrength  f: $floorStrength strengthFactor: $strengthFactor');
	}

	private static function DecayObject(x:Int, y:Int, passedTimeInYears:Float) {
		var world = WorldMap.world;
		var objId = world.getObjectId(x, y)[0];

		if (objId == 0) return;

		var floorId = world.getFloorId(x, y);
		var biomeId = world.getBiomeId(x, y);
		var biomeDecayFactor:Float = Biome.getBiomeDecayFactor(biomeId);
		if (floorId != 0) biomeDecayFactor = 1; // normal decay on floor

		if (ServerSettings.CanObjectRespawn(objId) == false) return;
		var objData = world.getObjectDataAtPosition(x, y);

		if (objData.decayFactor <= 0) return;

		var decayChance = ServerSettings.ObjDecayChance * objData.decayFactor;
		var countAs = objData.countsOrGrowsAs > 0 ? objData.countsOrGrowsAs : objData.parentId;

		// for example a knife with 58 steps with a tech level of 20 holds round about 4 times longer
		var techLevel = ServerSettings.ObjDecayFactorPerTechLevel;
		var techFactor = techLevel / (techLevel + objData.carftingSteps);

		decayChance *= techFactor;

		// if(objData.carftingSteps > 1) trace('${objData.name} steps: ${objData.carftingSteps} techFactor: ${Math.round(techFactor*100)}% decayFactor: ${Math.round(objData.decayFactor*100)}%');
		if (world.currentObjectsCount[countAs] < world.originalObjectsCount[countAs] * 0.8) return; // dont decay natural stuff if there are too few

		var objectHelper = world.getObjectHelper(x, y, true);
		var containsSomething = objectHelper != null && objectHelper.containedObjects.length > 0;
		var floorDecayFactor = floorId > 0 ? 0.0 : 1.0;

		// Pine Floor 3290
		if (floorId == 3290) floorDecayFactor = 0.5;
		// Stone Road 1596
		if (floorId == 1596) floorDecayFactor = 0.5;

		// if (containsSomething && (floorId > 0 || objId != 292)) return; // TODO 292 Basket ==> Allow all containers
		// Dont decay stuff on Floor except walls
		if (floorDecayFactor < 0.01 && objData.isWall() == false) return;
		if (floorDecayFactor < 0.01 && containsSomething && objData.decaysToObj < 1) return;
		// if (floorId > 0 && (objData.isWall() == false || (containsSomething && objData.decaysToObj < 1))) return; // TODO Decay Allow containers in colored walls
		// TODO Check all container decay / Decay Allow containers in colored walls
		if (objData.isWall()) decayChance *= ServerSettings.ObjDecayFactorForWalls;

		// only allow object with time transition to decay if there is no custom decay set // 161 Rabbit to unstuck them from the corner
		// if (objectHelper != null && objectHelper.timeToChange > 0 && objectHelper.timeToChange < 3600 && countAs != 161) return;
		if (objectHelper != null && objectHelper.timeToChange > 0 && objData.decaysToObj < 1 && countAs != 161) return;

		// var objData = ObjectData.getObjectData(objId);

		if (objData.isWall() == false) decayChance *= floorDecayFactor;
		decayChance *= biomeDecayFactor;

		if (objData.isNoBoneGrave()) decayChance *= 0.01;

		// if(containsSomething) decayChance *= 10000;

		if (objData.foodValue > 0) decayChance *= ServerSettings.ObjDecayFactorForFood;

		if (objData.isClothing()) decayChance *= ServerSettings.ObjDecayFactorForClothing;

		// if (floorId != 0) decayChance *= ServerSettings.ObjDecayFactorOnFloor;

		if (objData.isPermanent()) decayChance *= ServerSettings.ObjDecayFactorForPermanentObjs;
		// if (objData.permanent <= 0) trace('decay not permanent: ${objData.description}');

		if (world.randomFloat() > decayChance) return;

		// if(objData.isSpawningIn(biomeId) == false) continue;
		if (objectHelper == null) objectHelper = world.getObjectHelper(x, y);
		var decaysToObj = objData.decaysToObj;
		var decaysToObjectData = ObjectData.getObjectData(decaysToObj);
		var decaysToObjSlots = decaysToObjectData.numSlots;
		if (decaysToObj == 0 && objData.permanent > 0) decaysToObj = 618; // 618 Filled Small Trash Pit

		while (objectHelper.containedObjects.length > decaysToObjSlots) {
			var containedObject = objectHelper.containedObjects.pop();
			WorldMap.PlaceObject(x, y, containedObject);
			trace('Decay: ${objectHelper.name} place contained: ${containedObject.name} newSlots: ${decaysToObjSlots} use Slots: ${objectHelper.containedObjects.length - 1}');
		}

		objectHelper.id = decaysToObj;
		objectHelper.TransformToDummy();
		// world.setObjectId(x, y, [decaysToObj]);
		world.setObjectHelper(x, y, objectHelper);

		world.currentObjectsCount[countAs]--;

		Connection.SendMapUpdateToAllClosePlayers(x, y);

		trace('decay object: ${objData.name} $objId');
	}

	// TODO find a better way to respawn this stuff???
	public static function DoRespawnFromOriginal(tx:Int, ty:Int, passedTimeInYears:Float) {
		var world = WorldMap.world;
		var objData = world.getObjectDataAtPosition(tx, ty);
		var origObj = world.getOriginalObjectId(tx, ty);

		// remember that true needed time is 4x since it regrows only in spring

		if (origObj[0] == 50) // Milkweed
		{
			if (world.randomFloat() < passedTimeInYears / 60) {
				world.setObjectId(tx, ty, [50]);
			}
		} else if (origObj[0] == 136) // Sapling
		{
			if (world.randomFloat() < passedTimeInYears / 60) {
				world.setObjectId(tx, ty, [136]);
			}
		} else if (origObj[0] == 1261) // 1261 Canada Goose Pond with Egg
		{
			if (world.randomFloat() < passedTimeInYears / (60 * 24)) {
				world.setObjectId(tx, ty, [1261]);
			}
		} else if (origObj[0] == 211) // 211 Fertile Soil Deposit // TODO remove if AI can handle compost
		{
			if (world.randomFloat() < passedTimeInYears / (60 * 24 * 2)) {
				world.setObjectId(tx, ty, [211]);
			}
		}

		if (objData.parentId == 511) // Pond --> Canada Goose Pond
		{
			if (world.randomFloat() < passedTimeInYears / 60) {
				var obj = world.getObjectHelper(tx, ty);
				var numUses = obj.numberOfUses;
				obj.id = 141;
				obj.numberOfUses = numUses; // not sure if needed
				obj.TransformToDummy();
				world.setObjectHelper(tx, ty, obj);
			}
		}
	}

	public static function DoSpringStuff(tx:Int, ty:Int, passedTimeInYears:Float) {
		var world = WorldMap.world;
		var objData = world.getObjectDataAtPosition(tx, ty);
		// var origObj = world.getOriginalObjectId(tx, ty);
		// trace('Hungry Grizzly Bear: ${world.currentObjectsCount[631]} Bear Cave: ${world.originalObjectsCount[630]}');
		// trace('Hungry Grizzly Bear!');
		/*if (objData.parentId == 631) {
			var obj = world.getObjectHelper(tx, ty);
			obj.id = 630;
			world.setObjectHelper(tx, ty, obj);
		}*/
		// if (passedTimeInYears < 0.01) passedTimeInYears = 0.1;

		// Bear Cave 630 --> Bear Cave awake 648
		if (objData.parentId == 630) {
			// trace('Hungry Grizzly Bear:1 ${world.currentObjectsCount[631]} Bear Cave: ${world.originalObjectsCount[630]}');
			// trace('Hungry Grizzly Bear: ${world.currentObjectsCount[631]} Bear Cave: ${world.originalObjectsCount[630]}');

			// Hungry Grizzly Bear 631
			// var countAs = objData.countsOrGrowsAs > 0 ? objData.countsOrGrowsAs : objData.parentId;
			if (world.currentObjectsCount[631] < world.originalObjectsCount[630] * 0.1) {
				// trace('Hungry Grizzly Bear: rand: ${world.randomFloat()} years: ${passedTimeInYears * 5}');

				if (world.randomFloat() < passedTimeInYears / 2) {
					world.currentObjectsCount[631] += 1;

					trace('Hungry Grizzly Bear: NEW! ${world.currentObjectsCount[631]} Bear Cave: ${world.originalObjectsCount[630]}');

					var obj = world.getObjectHelper(tx, ty);
					obj.id = 648;
					world.setObjectHelper(tx, ty, obj);
				}
			}
		}
	}

	public static function DoSeasonalBiomeChanges(tx:Int, ty:Int, timePassedInYears:Float) {
		// Season = Seasons.Winter;

		if (Season == Seasons.Winter) SpreadSnow(tx, ty, timePassedInYears);
		if (Season == Seasons.Summer || Season == Seasons.Spring) RemoveSnow(tx, ty, timePassedInYears);
	}

	public static function SpreadSnow(tx:Int, ty:Int, timePassedInYears:Float) {
		var world = WorldMap.world;
		var biomeId = world.getBiomeId(tx, ty);

		if (biomeId != SNOW && biomeId != BiomeTag.SNOWINGREY) return;

		var chance = timePassedInYears * ServerSettings.SeasonBiomeChangeChancePerYear * 4; // 4 because 4 directions
		if (world.randomFloat() > chance) return;

		var rand = world.randomInt(3);
		var randX = tx;
		var randY = ty;

		if (rand == 0) randX = tx + 1; else if (rand == 1) randX = tx - 1; else if (rand == 2) randY = ty + 1; else if (rand == 3) randY = ty - 1;

		if (IsProtected(randX, randY) == false) world.setBiomeId(randX, randY, SNOW);

		// also diagonal
		var xOff = 1 - world.randomInt(2);
		var yOff = 1 - world.randomInt(2);

		// dont spread snow over corners to allow buildings with open corners like vic build them
		var objData = world.getObjectDataAtPosition(tx + xOff, ty);
		if (objData.isWall()) xOff = 0;

		var objData = world.getObjectDataAtPosition(tx, ty + yOff);
		if (objData.isWall()) yOff = 0;

		var randX = tx + xOff;
		var randY = ty + yOff;

		if (IsProtected(randX, randY) == false) {
			world.setBiomeId(randX, randY, SNOW);

			// create and destroy stones
			// TODO stone piles
			var floorId = world.getFloorId(randX, randY);

			if (floorId < 1) {
				var fromObjData = world.getObjectDataAtPosition(tx, ty);
				var objData = world.getObjectDataAtPosition(randX, randY);
				var originalObjId = world.getOriginalObjectId(randX, randY);

				// 133 Flint
				if (objData.parentId == 0 && originalObjId[0] == 133) {
					var rand = world.randomFloat();

					if (rand < 0.05 && WorldMap.world.currentObjectsCount[133] < WorldMap.world.originalObjectsCount[133]) {
						world.setObjectId(randX, randY, [133]);
						Connection.SendMapUpdateToAllClosePlayers(randX, randY);
						WorldMap.world.currentObjectsCount[133]++;
						if (ServerSettings.DebugSeason)
							trace('SEASON SNOW: NEW FLINT ${WorldMap.world.currentObjectsCount[133]} original: ${WorldMap.world.originalObjectsCount[133]}');
					}
				}

				// 33 Stone // 34 Sharp Stone // 135 Flint Chip // 850 Stone Hoe // 848 Hardened Row
				if ((objData.parentId == 33 || objData.parentId == 34 || objData.parentId == 135 || objData.parentId == 850 || objData.parentId == 848)) {
					var rand = world.randomFloat();
					if (WorldMap.world.currentObjectsCount[objData.parentId] < WorldMap.world.originalObjectsCount[objData.parentId] * 0.8) rand = 1;

					if (rand < 0.05) {
						world.setObjectId(randX, randY, [objData.decaysToObj]);
						Connection.SendMapUpdateToAllClosePlayers(randX, randY);
						WorldMap.world.currentObjectsCount[objData.parentId]--;
						if (ServerSettings.DebugSeason)
							trace('SEASON DECAY ${objData.name}: ${WorldMap.world.currentObjectsCount[objData.parentId]} original: ${WorldMap.world.originalObjectsCount[objData.parentId]}');
					}
				}

				// 32 Big Hard Rock
				if (objData.parentId == 0 && fromObjData.parentId == 32) {
					var rand = world.randomFloat();
					if (rand < 0.05) {
						world.setObjectId(randX, randY, [33]);
						Connection.SendMapUpdateToAllClosePlayers(randX, randY);
						WorldMap.world.currentObjectsCount[33]++;
						if (ServerSettings.DebugSeason)
							trace('SEASON NEW STONE: ${WorldMap.world.currentObjectsCount[33]} original: ${WorldMap.world.originalObjectsCount[33]}');
					}
				}

				// Move Stones: // 33 Stone // 34 Sharp Stone
				if (objData.parentId == 0 && (fromObjData.parentId == 33 || fromObjData.parentId == 34)) {
					var rand = world.randomFloat();
					if (rand < 0.2) {
						world.setObjectId(tx, ty, [0]);
						world.setObjectId(randX, randY, [33]);
						Connection.SendAnimalMoveUpdateToAllClosePlayers(tx, ty, randX, randY, [0], [fromObjData.parentId], 1);
						// if(ServerSettings.DebugSeason) trace('SEASON MOVE STONE');
					}
				}
			}
		}
		// trace('DoSeasonalBiomeChanges: $randX $randY');
	}

	public static function IsProtected(tx:Int, ty:Int):Bool {
		var world = WorldMap.world;
		var biomeId = world.getBiomeId(tx, ty);

		if (biomeId != GREY && biomeId != YELLOW && biomeId != GREEN && biomeId != SWAMP && biomeId != PASSABLERIVER) return true;

		var objData = world.getObjectDataAtPosition(tx, ty);
		var insulation = objData.isClothing() ? 0 : objData.getInsulation();

		if (insulation > world.randomFloat()) return true;
		if (insulation > world.randomFloat()) return true; // let walls protect twice

		// trace('DoSeasonalBiomeChanges: ${objData.name} WallInsulation: $insulation ==> snow');

		var floorId = world.getFloorId(tx, ty);
		var floorObjData = ObjectData.getObjectData(floorId);
		var floorInsulation = floorObjData.getInsulation();

		if (floorInsulation > world.randomFloat()) return true;

		// trace('DoSeasonalBiomeChanges: ${floorObjData.name} FloorInsulation: $floorInsulation ==> snow');

		return false;
	}

	private static function RemoveSnow(tx:Int, ty:Int, timePassedInYears:Float) {
		var world = WorldMap.world;
		var biomeId = world.getBiomeId(tx, ty);

		if (biomeId != SNOW) return;

		var chance = timePassedInYears * ServerSettings.SeasonBiomeChangeChancePerYear * 4; // 4 because 4 directions
		chance *= ServerSettings.SeasonBiomeRestoreFactor;
		if (world.randomFloat() > chance) return;

		var rand = world.randomInt(3);
		var randX = tx;
		var randY = ty;

		if (rand == 0) randX = tx + 1; else if (rand == 1) randX = tx - 1; else if (rand == 2) randY = ty + 1; else if (rand == 3) randY = ty - 1;

		var biomeId = world.getBiomeId(randX, randY);
		var doChange = true;

		if (biomeId == SNOW || biomeId == SNOWINGREY) doChange = false;

		var originalBiome = world.getOriginalBiomeId(tx, ty);

		if (doChange) world.setBiomeId(tx, ty, originalBiome);

		// also diagonal
		var randX = tx + 1 - world.randomInt(2);
		var randY = ty + 1 - world.randomInt(2);

		var biomeId = world.getBiomeId(randX, randY);
		var doChange = true;

		if (biomeId == SNOW || biomeId == SNOWINGREY) doChange = false;

		var originalBiome = world.getOriginalBiomeId(tx, ty);

		if (doChange) world.setBiomeId(tx, ty, originalBiome);

		// trace('DoSeasonalBiomeChanges: $randX $randY originalBiome: $originalBiome');
	}

	// public static function DecayObjects() {}

	public static function doTimeTransition(helper:ObjectHelper):Bool {
		if (helper.isTimeToChangeReached() == false) return false;
		// trace('TIME: ${helper.objectData.description} passedTime: $passedTime neededTime: ${timeToChange}');

		// TODO test time transition for maxUseTaget like Goose Pond:
		// -1 + 142 = 0 + 142
		// -1 + 142 = 0 + 141

		WorldMap.world.mutex.acquire();

		// just to be sure, that no other thread changed object meanwhile

		if (helper != WorldMap.world.getObjectHelper(helper.tx, helper.ty)) {
			WorldMap.world.mutex.release();

			trace("TIME: some one changed helper meanwhile");

			return false;
		}

		var sendUpdate = false;

		Macro.exception(sendUpdate = doTimeTransitionHelper(helper));

		if (sendUpdate == false) {
			WorldMap.world.mutex.release();
			return false;
		}

		Connection.SendMapUpdateToAllClosePlayers(helper.tx, helper.ty);

		WorldMap.world.mutex.release();

		return true;
	}

	public static function doTimeTransitionHelper(helper:ObjectHelper):Bool {
		var tx = helper.tx;
		var ty = helper.ty;

		var tileObject = Server.server.map.getObjectId(tx, ty);
		// var objData = ObjectData.getObjectData(tileObject[0]);
		// trace('Time: tileObject: $tileObject ${objData.name}');

		var transition = TransitionImporter.GetTransition(-1, tileObject[0], false, false);

		if (transition == null) {
			helper.timeToChange = 0;
			WorldMap.world.setObjectHelperNull(tx, ty);
			// FIX 4773 rabbit bones
			trace('WARNING: Time: no transtion found! Maybe object was moved? tile: $tileObject helper: ${helper.id} ${helper.description}');
			return false;
		}

		var newObjectData = ObjectData.getObjectData(transition.newTargetID);

		// for example if a grave with objects decays
		if (helper.containedObjects.length > newObjectData.numSlots) {
			// check in another 20 sec
			helper.timeToChange += 20;

			var containedObject = helper.containedObjects.pop();

			// For each Sharp Stone a grave needs much longer to decay / This can be used to let cursed graves exist much longer
			if (containedObject.id == 34) {
				helper.timeToChange += ServerSettings.CursedGraveTime * 60 * 60;
				ScoreEntry.CreateScoreEntryForCursedGrave(helper);
			}

			WorldMap.PlaceObject(tx, ty, containedObject);

			trace('time: placed object ${containedObject.description} from ${helper.description}');

			// trace('time: could not decay newTarget cannot store contained objects! ${helper.description}');

			return false;
		}

		if (transition.move > 0) {
			Macro.exception(doAnimalMovement(helper, transition));
			return false;
		}

		ScoreEntry.CreateScoreEntryIfGrave(helper);

		if (helper.isLastUse()) {
			var tmpTransition = TransitionImporter.GetTransition(-1, helper.parentId, false, true);
			if (tmpTransition != null) transition = tmpTransition; else {
				var objData = ObjectData.getObjectData(tileObject[0]);
				var name = objData == null ? '' : objData.name;
				trace('WARNING: TIME: $tileObject ${name} isLastUse transition not found!');
			}
		}

		var isMaxUse = false;
		// if it is a reverse transition, check if it would exceed max numberOfUses
		if (transition.reverseUseTarget && helper.numberOfUses >= newObjectData.numUses) {
			// if (ServerSettings.DebugTransitionHelper)
			//	trace('TRANS: ${player.name + player.id} Target: numberOfUses >= newTargetObjectData.numUses: ${this.target.numberOfUses} ${newTargetObjectData.numUses} try use maxUseTransition');
			transition = TransitionImporter.GetTransition(-1, helper.parentId, false, false, true);

			if (transition == null) {
				// trace('TIME: Maxuse: Cannot do reverse transition for taget: ${helper.name} numberOfUses: ${helper.numberOfUses} newObjectData.numUses: ${newObjectData.numUses}');
				helper.creationTimeInTicks = TimeHelper.tick;
				return false;
			}

			// trace('TIME: Maxuse: ${transition.getDesciption()}');
			isMaxUse = true;
		}

		// can have different random outcomes like Blooming Squash Plant
		helper.id = TransitionHelper.TransformTarget(transition.newTargetID);
		helper.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(helper);
		helper.creationTimeInTicks = TimeHelper.tick;

		if (isMaxUse) helper.numberOfUses = helper.objectData.numUses; else
			TransitionHelper.DoChangeNumberOfUsesOnTarget(helper, transition, null, false);

		WorldMap.world.setObjectHelper(tx, ty, helper);

		return true;
	}

	private static function doTimeForObject(obj:ObjectHelper):Bool {
		// 330 Hot Steel Ingot on Flat Rock
		// if(obj.parentId == 330) trace('TIME: In Container: ${obj.name} timeToChange: ${obj.timeToChange}');
		// trace('TIME: In Container: ${obj.name} --> ${obj.timeToChange}');

		var timeTransition = TransitionImporter.GetTransition(-1, obj.parentId, false, false);
		if (timeTransition == null) return false;

		if (obj.timeToChange <= 0) obj.timeToChange = timeTransition.calculateTimeToChange();

		if (obj.isTimeToChangeReached() == false) return false;

		var transition = TransitionImporter.GetTransition(-1, obj.parentId, false, false);

		if (transition == null) {
			obj.timeToChange = 0;
			return false;
		}

		var newObjectData = ObjectData.getObjectData(transition.newTargetID);

		// for example if a grave with objects decays
		if (obj.containedObjects.length > newObjectData.numSlots) {
			// TODO
			return false;
		}

		if (obj.isLastUse()) {
			var tmpTransition = TransitionImporter.GetTransition(-1, obj.id, false, true);
			if (tmpTransition != null) transition = tmpTransition;
		}

		trace('TIME: In Container: ${obj.name} --> ${newObjectData.name}');

		// can have different random outcomes like Blooming Squash Plant
		obj.id = TransitionHelper.TransformTarget(transition.newTargetID);
		obj.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(obj);
		obj.creationTimeInTicks = TimeHelper.tick;

		TransitionHelper.DoChangeNumberOfUsesOnTarget(obj, transition, null, false);

		return true;
	}

	private static function GetClosestBoneGrave(from:ObjectHelper) {
		var graves = [for (obj in WorldMap.world.cursedGraves) obj];
		var bestGrave = null;
		var bestQuadDistance = -1.0;

		for (grave in graves) {
			if (grave.isBoneGrave() == false) continue;

			var quadDistance = AiHelper.CalculateDistance(from.tx, from.ty, grave.tx, grave.ty);

			if (bestGrave == null) {
				bestGrave = grave;
				bestQuadDistance = quadDistance;
				continue;
			}
			if (quadDistance > bestQuadDistance) continue;

			bestGrave = grave;
			bestQuadDistance = quadDistance;
		}

		return bestGrave;
	}

	/*
		MX
		x y new_floor_id new_id p_id old_x old_y speed

		Optionally, a line can contain old_x, old_y, and speed.
		This indicates that the object came from the old coordinates and is moving
		with a given speed.
	 */
	private static function doAnimalMovement(animal:ObjectHelper, timeTransition:TransitionData):Bool {
		// TODO fleeing
		// TODO Offspring only if with child
		// TODO eating meat / killing sheep

		var moveDist = timeTransition.move;
		if (moveDist <= 0) return false;
		if (moveDist > 3) moveDist = 3;
		timeTransition.move = 3;

		if (moveDist < 3) moveDist += 1; // movement distance is plus 4 in vanilla if walking over objects
		animal.objectData.moves = moveDist; // TODO better set in settings

		if (animal.hits > 0) animal.hits -= 0.005; // reduce hits the animal got
		if (animal.hits < 0) animal.hits = 0;

		var worldmap = Server.server.map;
		var objectData = animal.objectData.dummyParent == null ? animal.objectData : animal.objectData.dummyParent;
		var fromTx = animal.tx;
		var fromTy = animal.ty;
		var currentbiome = worldmap.getBiomeId(animal.tx, animal.ty);
		var currentOriginalbiome = worldmap.getOriginalBiomeId(animal.tx, animal.ty);
		var lovesCurrentBiome = objectData.isSpawningIn(currentbiome);
		var lovesCurrentOriginalBiome = objectData.isSpawningIn(currentOriginalbiome);

		if (lovesCurrentOriginalBiome) {
			// trace('set loved tx,ty for animal: ${animal.description}');
			animal.lovedTx = animal.tx;
			animal.lovedTy = animal.ty;
		}

		// choose targets
		var gotoTarget = TimeHelper.Season == Winter || currentbiome == BiomeTag.SNOW;

		// 418 Wolf // 631 Hungry Grizzly Bear
		if (gotoTarget && (animal.parentId == 418 || animal.parentId == 631)) {
			if (animal.target == null) animal.target = GetClosestBoneGrave(animal); else {
				var objData = WorldMap.world.getObjectDataAtPosition(animal.target.tx, animal.target.ty);
				if (objData.isBoneGrave() == false) animal.target = GetClosestBoneGrave(animal);
			}
		}

		if (animal.isDeadlyAnimal()) {
			// Shot Wolf 420 // Shot Bison 1438 // Shot Grizzly Bear 632
			var chasingAnimals = [420, 1438, 632, 635, 637];
			// trace('Deadly: ${animal.name}');

			// Rattle Snake 764
			var chaseDistance = animal.parentId == 764 ? 5 : 20;
			var lovedSeason = animal.parentId == 764 ? Summer : Winter;
			var rightSeason = TimeHelper.Season == lovedSeason;

			// Wild Boar 1323 // Wild Boar with Piglet 1328 // Bison 1435 // Bison with Calf 1436
			var animalsDontChase = [1323, 1328, 1435, 1436];
			if (animalsDontChase.contains(animal.parentId)) rightSeason = false;

			if (animal.hits > 0 || rightSeason || chasingAnimals.contains(animal.parentId)) {
				var player = GlobalPlayerInstance.GetClosestPlayerAt(animal.tx, animal.ty, chaseDistance);
				if (player != null) {
					// trace('Deadly: ${animal.name} target: ${player.name}');
					animal.target = worldmap.getObjectHelper(player.tx, player.ty);
					gotoTarget = true;

					// Alert close animal
					var closeAnimal = AiHelper.GetClosestObjectToPosition(animal.tx, animal.ty, animal.parentId, 20, animal);
					if (closeAnimal != null && closeAnimal.hits <= 0) closeAnimal.hits = 0.1;
					// if (closeAnimal != null) trace('Deadly: Alert: ${closeAnimal.name} hits: ${closeAnimal.hits}');
				}
			}
		}

		var maxIterations = 20;
		var besttarget = null;
		var bestQuadDist = 999999999999999999999999999999999.9;

		for (i in 0...maxIterations) {
			var toTx = animal.tx - moveDist + worldmap.randomInt(moveDist * 2);
			var toTy = animal.ty - moveDist + worldmap.randomInt(moveDist * 2);

			var target = worldmap.getObjectHelper(toTx, toTy);

			if (CanAnimalEndUpHere(animal, target) == false) continue;

			// make sure that target is not the old tile
			if (toTx == animal.tx && toTy == animal.ty) continue;

			// if (target.id != 0) trace('MOVE target: ${target.name}');

			var targetBiome = worldmap.getBiomeId(toTx, toTy);

			if (WorldMap.isBiomeBlocking(toTx, toTy)) continue;

			var isPreferredBiome = objectData.isSpawningIn(targetBiome);

			// if(animal.objectData.dummyParent != null) trace('Animal Move: ${objectData.description} $isPreferredBiome');

			// lower the chances even more if on river
			// var isHardbiome = targetBiome == BiomeTag.RIVER || (targetBiome == BiomeTag.GREY) || (targetBiome == BiomeTag.SNOW) || (targetBiome == BiomeTag.DESERT);
			var isNotHardbiome = isPreferredBiome || targetBiome == BiomeTag.GREEN || targetBiome == BiomeTag.YELLOW;

			var chancePreferredBiome = isNotHardbiome ? ServerSettings.chancePreferredBiome : (ServerSettings.chancePreferredBiome + 4) / 5;

			// trace('chance: $chancePreferredBiome isNotHardbiome: $isNotHardbiome biome: $targetBiome');

			// skip with chancePreferredBiome if this biome is not preferred
			if (isPreferredBiome == false && i < 5 && worldmap.randomFloat() <= chancePreferredBiome) continue;

			// limit movement if blocked
			target = calculateNonBlockedTarget(animal, fromTx, fromTy, target);
			if (target == null) continue; // movement was fully bocked, search another target
			if (target.id != 0 && i < maxIterations / 2) continue; // prefer to go on empty tiles

			// if (target.id != 0) trace('MOVE target: ${target.name}');

			var gotoLovedBiome = gotoTarget == false
				&& lovesCurrentBiome == false
				&& isPreferredBiome == false
				&& (animal.lovedTx != 0 || animal.lovedTy != 0);
			var targetTx = gotoLovedBiome || animal.target == null ? animal.lovedTx : animal.target.tx;
			var targetTy = gotoLovedBiome || animal.target == null ? animal.lovedTy : animal.target.ty;

			// if(animal.target != null) trace('animal: ${animal.name} ${animal.tx} ${animal.ty} ==> ${animal.target.tx} ${animal.target.ty}');

			// try to go closer to loved biome
			if (gotoTarget || gotoLovedBiome) {
				var quadDist = AiHelper.CalculateDistance(targetTx, targetTy, target.tx, target.ty);
				if (quadDist < bestQuadDist) {
					bestQuadDist = quadDist;
					besttarget = target;
					maxIterations = i + 6; // try to find better

					// trace('i: $i set better target for animal: ${animal.description} quadDist: $quadDist');
				}

				if (i < maxIterations - 2) continue; // try to find better

				if (besttarget != null) target = besttarget;

				// Connection.SendLocationToAllClose(animal.lovedTx, animal.lovedTy, animal.name);

				// toTx = toTx > animal.lovedTx ? toTx - 1 : toTx + 1;
			}

			toTx = target.tx;
			toTy = target.ty;

			// 2710 + -1 = 767 + 769 // Wild Horse with Lasso + TIME  -->  Lasso# tool + Wild Horse
			// TODO merge with timeAlternaiveTransition? // TODO shouldnt the alternative target be set?
			var transition = TransitionImporter.GetTransition(animal.parentId, -1, false, false);
			var tmpGroundObject = animal.groundObject;

			if (transition != null) {
				animal.groundObject = new ObjectHelper(null, transition.newActorID);
				animal.groundObject.groundObject = tmpGroundObject;

				// trace('animal movement: found -1 transition: ${animal.description} --> ${animal.groundObject.description}');
			}

			var isLastMove = animal.isLastUse();
			if (isLastMove) timeTransition = TransitionImporter.GetTransition(-1, animal.parentId, false, true);

			// FIX: 544 Domestic Mouflon with Lamb + -1  ==> 545 Domestic Lamb + 541 Domestic Mouflon
			var timeAlternaiveTransition = isLastMove ? TransitionImporter.GetTransition(animal.parentId, -1, true, false) : null;

			// FIX: Allow Shorn Domestic Sheep 577 --> Fleece 578 + Shorn Domestic Sheep 576
			if (timeAlternaiveTransition == null && timeTransition.newActorID > 0) timeAlternaiveTransition = timeTransition;

			animal.id = timeAlternaiveTransition != null ? timeAlternaiveTransition.newTargetID : timeTransition.newTargetID;
			if (isLastMove) {
				animal.numberOfUses = animal.objectData.numUses;
				animal.TransformToDummy();
			}

			var rabbitInWrongPlace = false;
			// 3568 Fleeing Rabbit dest# groundOnly // 3566 Fleeing Rabbit
			if (animal.parentId == 3568 && targetBiome != YELLOW && targetBiome != GREEN) {
				animal.id = 3566; // dont go in the ground
				rabbitInWrongPlace = true;
			}

			if (animal.parentId == 3568 && (currentOriginalbiome == YELLOW || currentOriginalbiome == GREEN)) {
				rabbitInWrongPlace = false;
			}

			TransitionHelper.DoChangeNumberOfUsesOnTarget(animal, timeTransition, false);

			// save what was on the ground, so that we can move on this tile and later restore it
			var oldTileObject = animal.groundObject == null ? [0] : animal.groundObject.toArray();
			var newTileObject = animal.toArray();

			// var des = animal.groundObject == null ? "NONE": 'GROUND: ${animal.groundObject.name}';
			// if(animal.groundObject != null && animal.groundObject.id != 0) trace('MOVE: oldTile: $des $oldTileObject newTile: ${animal.name} $newTileObject');

			if (timeAlternaiveTransition == null) {
				worldmap.setObjectHelper(fromTx, fromTy, animal.groundObject);
			} else {
				// FIX: 544 Domestic Mouflon with Lamb + -1  ==> 545 Domestic Lamb + 541 Domestic Mouflon

				oldTileObject = [timeAlternaiveTransition.newActorID];
				worldmap.setObjectId(fromTx, fromTy, oldTileObject);

				var newAnimal = worldmap.getObjectHelper(fromTx, fromTy);
				newAnimal.groundObject = tmpGroundObject;
				newAnimal.numberOfUses = newAnimal.objectData.numUses;
				newAnimal.TransformToDummy();
				worldmap.setObjectHelper(fromTx, fromTy, newAnimal);

				trace('timeAlternaiveTransition: ${timeAlternaiveTransition.getDescription()}');
			}

			var tmpGroundObject = animal.groundObject;
			animal.groundObject = target;

			// var des = animal.groundObject == null ? "NONE": 'GROUND: ${animal.groundObject.name}';
			// if(animal.groundObject != null && animal.groundObject.id != 0) trace('MOVE: oldTile: $des $oldTileObject newTile: ${animal.name} $newTileObject');

			worldmap.setObjectHelper(toTx, toTy, animal); // set so that object has the right position before doing damage
			// helper.tx = toTx;
			// helper.ty = toTy;
			var damage = DoAnimalDamage(fromTx, fromTy, animal);
			// TODO only change after movement is finished
			if (damage <= 0) animal.timeToChange = timeTransition.calculateTimeToChange();
			animal.creationTimeInTicks = TimeHelper.tick;

			worldmap.setObjectHelper(toTx, toTy, animal); // set again since animal might be killed

			// var chanceForOffspring = isPreferredBiome ? ServerSettings.ChanceForOffspring : ServerSettings.ChanceForOffspring * Math.pow((1
			//	- chancePreferredBiome), 2);
			var chanceForOffspring = isPreferredBiome ? ServerSettings.ChanceForOffspring : ServerSettings.ChanceForOffspring / 100;
			var chanceForAnimalDying = isPreferredBiome ? ServerSettings.ChanceForAnimalDying * ServerSettings.ChanceForAnimalDyingFactorIfInLovedBiome : ServerSettings.ChanceForAnimalDying;

			// TODO let domestic animals multiply if there is enough green biome
			// TODO let domestic animals eat up green biome
			// TODO dont consider lovesCurrentOriginalBiome once domestic animals muliply
			if (animal.isDomesticAnimal() && (lovesCurrentBiome || lovesCurrentOriginalBiome)) chanceForAnimalDying /= 1000;
			if (rabbitInWrongPlace) chanceForAnimalDying *= 2; // 10

			var canDieIfPopulationIsAbove = rabbitInWrongPlace ? 0.4 : 0.8; // 0.2 0.8

			// var currentPop = worldmap.currentObjectsCount[newTileObject[0]];
			// var originalPop = worldmap.originalObjectsCount[newTileObject[0]];
			var countAs = animal.objectData.countsOrGrowsAs < 1 ? animal.parentId : animal.objectData.countsOrGrowsAs;
			var currentPop = worldmap.currentObjectsCount[countAs];
			var originalPop = worldmap.originalObjectsCount[countAs];

			// give extra birth chance bonus if population is very low
			if (currentPop < originalPop * ServerSettings.OffspringFactorLowAnimalPopulationBelow)
				chanceForOffspring *= ServerSettings.OffspringFactorIfAnimalPopIsLow;
			chanceForAnimalDying *= currentPop > originalPop ? 100 : 1;

			// if(rabbitInWrongPlace) trace('Animal DEAD? RABBIT: ${animal.name} $newTileObject Count: ${currentPop} Original Count: ${originalPop} chance: $chanceForAnimalDying biome: $targetBiome');

			if (currentPop < originalPop * ServerSettings.MaxOffspringFactor && worldmap.randomFloat() < chanceForOffspring) {
				var closeAnimal = AiHelper.GetClosestObjectToPosition(animal.tx, animal.ty, animal.parentId, 2, animal);

				if (closeAnimal != null) {
					// NO Offspring since too close to same animal
					// TODO does not yet consider count as
					trace('Animal Offspring: CLOSE ${animal.name} id: ${newTileObject} chance: ${chanceForOffspring} curPop: ${currentPop]} original: ${originalPop}');
				} else {
					worldmap.currentObjectsCount[countAs] += 1;

					trace('Animal Offspring: ${animal.name} id: ${newTileObject} chance: ${chanceForOffspring} curPop: ${currentPop]} original: ${originalPop}');

					// if(chanceForOffspring < worldmap.chanceForOffspring) trace('NEW: $newTileObject ${helper.description()}: ${worldmap.currentPopulation[newTileObject[0]]} ${worldmap.initialPopulation[newTileObject[0]]} chance: $chanceForOffspring biome: $targetBiome');

					oldTileObject = newTileObject;

					var newAnimal = ObjectHelper.readObjectHelper(null, newTileObject);
					newAnimal.timeToChange = timeTransition.calculateTimeToChange();
					newAnimal.groundObject = tmpGroundObject;
					worldmap.setObjectHelper(fromTx, fromTy, newAnimal);
				}
			} else if (currentPop > originalPop * ServerSettings.MaxOffspringFactor * canDieIfPopulationIsAbove
				&& originalPop > 0
				&& worldmap.randomFloat() < chanceForAnimalDying) {
				// decay animal only if it is a original one
				// TODO decay all animals and cosider if they cointain items like a horse wagon
				trace('Animal DEAD: ${animal.name} $newTileObject Count: ${currentPop} Original Count: ${originalPop} chance: $chanceForAnimalDying biome: $targetBiome');

				worldmap.currentObjectsCount[countAs] -= 1;
				animal.id = 0;
				newTileObject = animal.groundObject.toArray();
				worldmap.setObjectHelper(toTx, toTy, animal.groundObject);
			}

			animal.failedMoves = 0;
			var speed = ServerSettings.InitialPlayerMoveSpeed * objectData.speedMult;
			Connection.SendAnimalMoveUpdateToAllClosePlayers(fromTx, fromTy, toTx, toTy, oldTileObject, newTileObject, speed);

			// if(tmpGroundObject == null) tmpGroundObject = new ObjectHelper(null, 0);
			// if(tmpGroundObject.id != 0) trace('ANIMALMOVE: $oldTileObject ${tmpGroundObject.name} ${tmpGroundObject.id} ==> $newTileObject ${animal.name} ${animal.parentId}');

			// trace('ANIMALMOVE: true $i');

			return true;
		}

		animal.creationTimeInTicks = TimeHelper.tick;
		animal.failedMoves += WorldMap.calculateRandomFloat();
		// trace('ANIMALMOVE: false failedMoves: ${animal.failedMoves} ${animal.name}');

		// kill animal if it cannot move for some time
		if (animal.failedMoves > 20) {
			trace('ANIMALMOVE: dead failedMoves: ${animal.failedMoves} ${animal.name}');
			animal.failedMoves = 0;
			if (animal.groundObject != null && animal.groundObject.id > 0) WorldMap.world.setObjectHelper(animal.tx, animal.ty, animal.groundObject); else {
				animal.id = animal.objectData.decaysToObj;
				WorldMap.world.setObjectHelper(animal.tx, animal.ty, animal);
			}
			Connection.SendMapUpdateToAllClosePlayers(animal.tx, animal.ty);
		}

		return false;
	}

	public static function TryAnimaEscape(attacker:GlobalPlayerInstance, target:ObjectHelper):Bool {
		var weapon = attacker.heldObject;
		var isMeleeWeapon = weapon.objectData.isMeleeWeapon();
		var usingBowAndArrow = weapon.id == 152; // Bow and Arrow
		var animalEscapeFactor = weapon.objectData.animalEscapeFactor - target.hits * 0.25;
		var random = WorldMap.calculateRandomFloat();
		var weaponDamage = weapon.objectData.damage;
		var damage = weaponDamage / 2 + weaponDamage * WorldMap.world.randomFloat();

		if (target.isDomesticAnimal() && attacker.isHoldingWeapon() == false) return false;

		// attacker.say('escape att', true);

		target.hits += 1;

		// 3948 Arrow Quiver
		// 874 Empty Arrow Quiver
		if (usingBowAndArrow) {
			for (obj in attacker.clothingObjects) {
				if (obj.parentId == 3948 || obj.parentId == 874) animalEscapeFactor /= 2;
				// if(obj.parentId == 3948 || obj.parentId == 874) trace('TryAnimaEscape: Used Quiver $animalEscapeFactor');
			}
		}
		// else target.hits += damage / 50;

		if (ServerSettings.debug) trace('TryAnimaEscape: ${target.hits} damage: ${damage} random: $random > escape factor: $animalEscapeFactor');

		// attacker.say('Hits ${Math.round(target.hits)}', true);

		// if(attacker.makeWeaponBloodyIfNeeded()) return true; // TODO allow animal to be fully killed with knife

		if (random > animalEscapeFactor) {
			// attacker.makeWeaponBloodyIfNeeded(target);
			// target.hits += damage / 10;
			// if(isMeleeWeapon && target.isDeadlyAnimal()) attacker.say('Hits ${Math.round(target.hits)}', true);

			return false;
		}

		target.timeToChange /= 5;
		var tmpTimeToChange = target.timeToChange;
		doTimeTransition(target);

		var escaped = tmpTimeToChange != target.timeToChange;
		if (ServerSettings.debug) trace('TryAnimaEscape: $escaped');
		if (escaped == false) return false;

		attacker.say('Hits ${Math.round(target.hits)}', true);

		attacker.makeWeaponBloodyIfNeeded(target);

		if (usingBowAndArrow) // Bow and Arrow
		{
			weapon.id = 749; // 151 Bow // 749 Bloody Yew Bow
			attacker.setHeldObject(weapon);
			attacker.setHeldObjectOriginNotValid(); // no object move animation
			attacker.o_transition_source_id = -1;
			attacker.action = 0;
			weapon.timeToChange = 2;
			var done = false;
			Macro.exception(done = WorldMap.PlaceObject(target.tx, target.ty, new ObjectHelper(attacker, 798), true)); // Place Arrow Wound
			if (done == false) trace('WARNING: TryAnimaEscape: FAILING TO PLACE ARROW ON GROUND! Held: ${attacker.heldObject.name}');
			if (done == false) attacker.say('Placing Arrow failed! ${attacker.heldObject.name}', true);
		}

		return true;
	}

	public static function MakeAnimalsRunAway(player:GlobalPlayerInstance, searchDistance:Int = 1) {
		// AiHelper.GetClosestObject
		var world = WorldMap.world;
		var baseX = player.tx;
		var baseY = player.ty;

		for (ty in baseY - searchDistance...baseY + searchDistance) {
			for (tx in baseX - searchDistance...baseX + searchDistance) {
				var obj = world.getObjectHelper(tx, ty, true);
				if (obj == null) continue;
				if (obj.objectData.moves == 0) continue;
				if (obj.isDomesticAnimal() && player.isHoldingWeapon() == false) continue;
				// player.say('escape 2 domestic: ${obj.isDomesticAnimal()} weapon: ${player.isHoldingWeapon()}', true);

				var tmpTimeToChange = obj.timeToChange;
				obj.timeToChange /= 5;

				// release GlobalPlayermutex before trying to aquire world mutex, since doTimeTransition needs the world mutex. Always aquire world mutex first!
				if (ServerSettings.UseOneSingleMutex == false) GlobalPlayerInstance.ReleaseMutex();
				Macro.exception(doTimeTransition(obj));
				if (ServerSettings.UseOneSingleMutex == false) GlobalPlayerInstance.AcquireMutex();

				// trace('RUN: $tmpTimeToChange --> ${obj.timeToChange} ' + obj.description);
				// obj.timeToChange = tmpTimeToChange;
			}
		}
	}

	private static function calculateNonBlockedTarget(animal:ObjectHelper, fromX:Int, fromY:Int, toTarget:ObjectHelper):ObjectHelper {
		var tx = toTarget.tx;
		var ty = toTarget.ty;
		var tmpX = fromX;
		var tmpY = fromY;
		var tmpTarget = null;

		for (ii in 0...10) {
			if (tmpX == tx && tmpY == ty) break;

			if (tx > tmpX) tmpX += 1; else if (tx < tmpX) tmpX -= 1;

			if (ty > tmpY) tmpY += 1; else if (ty < tmpY) tmpY -= 1;

			// trace('movement: $tmpX,$tmpY');

			var movementTileObj = WorldMap.worldGetObjectHelper(tmpX, tmpY);
			// var movementBiome = WorldMap.worldGetBiomeId(tmpX , tmpY);

			// var cannotMoveInBiome = movementBiome == BiomeTag.OCEAN ||  movementBiome == BiomeTag.SNOWINGREY;

			var isBiomeBlocking = WorldMap.isBiomeBlocking(tmpX, tmpY);

			if (isBiomeBlocking
				&& ServerSettings.ChanceThatAnimalsCanPassBlockingBiome > 0)
				isBiomeBlocking = WorldMap.calculateRandomFloat() > ServerSettings.ChanceThatAnimalsCanPassBlockingBiome;

			// TODO better patch in the objects, i dont see any reason why a rabit or a tree should block movement
			if (isBiomeBlocking || (movementTileObj.blocksWalking() // )) {
				&& movementTileObj.description.indexOf("Tarry Spot") == -1
				&& movementTileObj.description.indexOf("Tree") == -1
				&& movementTileObj.description.indexOf("Rabbit") == -1
				&& movementTileObj.description.indexOf("Spring") == -1
				&& movementTileObj.description.indexOf("Sugarcane") == -1
				&& movementTileObj.description.indexOf("Pond") == -1
				&& movementTileObj.description.indexOf("Palm") == -1
				&& movementTileObj.description.indexOf("Plant") == -1
				&& movementTileObj.description.indexOf("Iron") == -1)) {
				// trace('movement blocked ${movementTile.description()} ${movementBiome}');

				break;
			}

			/*
				if(animal.objectData.isDomesticAnimal() && movementTileObj.id != 0){
					trace('Animal: ${animal.name} --> ${movementTileObj.name} not blocked!');
				} 

				if(animal.objectData.isDomesticAnimal() && movementTileObj.objectData.blocksDomesticAnimal){
					trace('Animal: ${animal.name} --> ${movementTileObj.name} domestic animal blocked!');
				} 
			 */

			if (animal.isDomesticAnimal() && movementTileObj.objectData.blocksDomesticAnimal) break;
			if (movementTileObj.objectData.blocksAnimal) break;

			// TODO allow move on non empty ground
			// if (movementTileObj.id == 0) tmpTarget = movementTileObj;
			if (CanAnimalEndUpHere(animal, movementTileObj)) tmpTarget = movementTileObj;
		}

		return tmpTarget;
	}

	private static function CanAnimalEndUpHere(animal:ObjectHelper, target:ObjectHelper):Bool {
		// if (target.id != 0) continue; // TODO test if walked on object are restored right

		if (target.blocksWalking()) return false;

		if (target.canMove()) return false; // dont move move on top of other moving stuff

		if (target.groundObject != null && target.groundObject.id != 0) return false;

		if (animal.isDomesticAnimal() && target.objectData.blocksDomesticAnimal) return false;

		if (target.objectData.blocksAnimal) return false;

		if (animal.parentId == 3566) { // 3566 Fleeing Rabbit
			if (target.id != 0) return false;

			var floorId = WorldMap.world.getFloorId(target.tx, target.ty);
			if (floorId != 0) return false;
		}

		return true;
	}

	private static function DoAnimalDamage(fromX:Int, fromY:Int, animal:ObjectHelper):Float {
		var damage = 0.0;
		GlobalPlayerInstance.AcquireMutex();
		Macro.exception(damage = DoAnimalDamageHelper(fromX, fromY, animal));
		GlobalPlayerInstance.ReleaseMutex();

		return damage;
	}

	private static function DoAnimalDamageHelper(fromX:Int, fromY:Int, animal:ObjectHelper):Float {
		var objData = animal.objectData;

		if (objData.deadlyDistance <= 0) return 0;
		if (objData.damage <= 0) return 0;

		// trace('${objData.description} deadlyDistance: ${objData.deadlyDistance} damage: ${objData.damage}');
		var damage = 0.0;
		var tx = animal.tx;
		var ty = animal.ty;
		var tmpX = fromX;
		var tmpY = fromY;

		for (ii in 0...10) {
			// if (ii > 0 && tmpX == tx && tmpY == ty) break;

			for (p in GlobalPlayerInstance.AllPlayers) {
				if (p.deleted) continue;
				if (p.heldByPlayer != null) continue;
				if (p.isCloseUseExact(tmpX, tmpY, objData.deadlyDistance) == false) continue;

				// trace('Do damage to: ${p.name}');
				damage += p.doDamage(animal);
				return damage;
			}

			if (tmpX == tx && tmpY == ty) break;
			if (tx > tmpX) tmpX += 1; else if (tx < tmpX) tmpX -= 1;
			if (ty > tmpY) tmpY += 1; else if (ty < tmpY) tmpY -= 1;
		}

		return damage;
	}

	// Called before time. Do tests here!
	private static function DoTest() {
		// var objData = ObjectData.getObjectData(2143); // Banana
		// var id = objData.getPileObjId();
		// trace('Pile: $id');

		// SerializeHelper.createReadWriteFile();

		/*var array = [];
			for (i in 0...1000000) {
				array[WorldMap.calculateRandomInt(1)] += 1;
				// array[Math.floor(WorldMap.calculateRandomFloat() * 3)] += 1;
				// array[Math.round(Math.floor(Math.random() * 2))] += 1;
				// if (WorldMap.calculateRandomFloat() > 0.5) array[0] += 1; else
				//	array[1] += 1;
			}

			for (i in 0...11)
				trace('rand: $i: ' + array[i]);
		 */

		// +biomeReq6 +biomeReq4 +biomeBlock4
		/*for(obj in ObjectData.importedObjectData)
			{
				if(StringTools.contains(obj.description, '+')) trace(obj.description);
		}*/

		return; // remove if testting
		var trans = TransitionImporter.GetTransition(418, 0, false, false);
		trace('TRANS4: $trans false, false');
		var trans = TransitionImporter.GetTransition(418, 0, true, false);
		trace('TRANS4: $trans true, false');
		// var trans = TransitionImporter.GetTransition(418, 0, true, false);
		// trace('TRANS: $trans');
	}

	// static var personIndex = 0;
	// static var colorIndex = 0;
	private static function DoTimeTestStuff() {
		if (tick % 200 == 0) {
			if (Connection.getConnections().length > 0) {
				/*
					var c = Connection.getConnections()[0];
					//FW follower_id leader_id leader_color_index
					c.send(ClientTag.FOLLOWING, ['${c.player.p_id} 2 $colorIndex']);
					//p_id emot_index
					c.send(ClientTag.PLAYER_EMOT, ['${c.player.p_id} $colorIndex']);
					//c.send(ClientTag.PLAYER_SAYS, ['0 0 $colorIndex']);
					//c.player.say('color $colorIndex');
					c.send(ClientTag.LOCATION_SAYS, ['100 0 /LEADER']);

					trace('FOLLOW '+ '${c.player.p_id} 2 $colorIndex');

					colorIndex++;

				 */
				/*
					c.player.po_id = obj.id;

					Connection.SendUpdateToAllClosePlayers(c.player);
					if(tick % 200 == 0) c.send(ClientTag.DYING, ['${c.player.p_id}']);
					else c.send(ClientTag.HEALED, ['${c.player.p_id}']);

					c.sendGlobalMessage('Id ${obj.parentId} P${obj.person} ${obj.description}');

					personIndex++;
					//  418 + 0 = 427 + 1363 / @ Deadly Wolf + Empty  -->  Attacking Wolf + Bite Wound 
				 */
			}
		}
	}
}
