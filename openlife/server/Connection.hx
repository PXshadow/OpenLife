package openlife.server;

import haxe.Exception;
import openlife.data.map.MapData;
import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;
import openlife.macros.Macro;
import openlife.server.Biome.BiomeTag;
import openlife.settings.ServerSettings;
#if (target.threaded)
import haxe.io.Bytes;
import openlife.client.ClientTag;
import sys.net.Socket;

class Connection {
	public static var debugText = '';
	private static var connections:Array<Connection> = [];
	private static var ais:Array<ServerAi> = [];

	public var player:GlobalPlayerInstance;
	public var playerAccount:PlayerAccount;
	public var serverAi:ServerAi; // null if connected to a client

	public var running:Bool = true;

	public var sock:Socket;
	public var sendFrame = false;
	var server:Server;
	var tag:ServerTag;

	private var messageQueue = new Array<String>();
	private var timeToWaitBeforeNextMessageSend:Float = 0;

	// if it is an AI sock = null
	public function new(sock:Socket, server:Server) {
		this.sock = sock;
		this.server = server;
		var challenge = "dsdjsiojdiasjiodsjiosd";
		var version = ObjectData.dataVersionNumber;

		// if it is an AI there is no sock
		if (sock != null) send(SERVER_INFO, ["0/0", challenge, '$version']);
	}

	public function isAi():Bool {
		// return this.connection.playerAccount.isAi;
		return this.sock == null;
	}

	/**
		LOGIN client_tag email password_hash account_key_hash tutorial_number twin_code_hash twin_count#
		NOTE:  The command LOGIN can be replaced with RLOGIN if the client is
		reconnnecting to an existing life and doesn't want to start a new life
		if the previous life is over.
	**/
	public function login(client_tag:String, email:String, password_hash:String, account_key_hash:String) {
		// A normal login is treated same as a reconnect
		// TODO twins

		trace('login: ${account_key_hash}');

		GlobalPlayerInstance.AllPlayerMutex.acquire();

		this.playerAccount = PlayerAccount.GetOrCreatePlayerAccount(email, account_key_hash);
		this.player = GlobalPlayerInstance.CreateNewHumanPlayer(this);

		Macro.exception(initConnection(this.player, this.playerAccount));

		trace('New Born Score: ${this.playerAccount.totalScore} Prestige: ${this.player.yum_multiplier}');

		if (this.player.trueAge < 5)
			this.sendGlobalMessage('YOUR PRESTIGE FROM LAST LIFE IS ${Math.ceil(this.player.yum_multiplier * ServerSettings.DisplayScoreFactor)}');
		this.sendGlobalMessage('EATING YUMMY FOOD AND HAVING MANY KIDS WILL INCREASE YOUR PRESTIGE!');

		GlobalPlayerInstance.AllPlayerMutex.release();
	}

	public function rlogin(client_tag:String, email:String, password_hash:String, account_key_hash:String) {
		GlobalPlayerInstance.AllPlayerMutex.acquire();

		this.playerAccount = PlayerAccount.GetOrCreatePlayerAccount(email, account_key_hash);
		var lastLivingPlayer = playerAccount.getLastLivingPlayer();

		if (lastLivingPlayer != null) {
			var lastConnection = lastLivingPlayer.connection;
			// deactivate AI
			ais.remove(lastConnection.serverAi);
			lastConnection.serverAi = null;
			var tx = lastLivingPlayer.tx;
			var ty = lastLivingPlayer.ty;
			lastLivingPlayer.gx = 0;
			lastLivingPlayer.gy = 0;
			lastLivingPlayer.x = tx;
			lastLivingPlayer.y = ty;
			lastLivingPlayer.moveHelper.exactTx = tx;
			lastLivingPlayer.moveHelper.exactTy = ty;

			if(lastLivingPlayer.heldPlayer != null) lastLivingPlayer.dropPlayer(0,0);

			Macro.exception(initConnection(lastLivingPlayer, this.playerAccount));

			trace('reconnect to ${player.p_id} ${player.name}');

			GlobalPlayerInstance.AllPlayerMutex.release();

			return;
		}

		GlobalPlayerInstance.AllPlayerMutex.release();

		login(client_tag, email, password_hash, account_key_hash);
	}

	private function initConnection(connectedPlayer:GlobalPlayerInstance, connectedPlayerAccount:PlayerAccount) {
		send(ACCEPTED);

		this.player = connectedPlayer;
		this.player.connection = this;
		this.playerAccount = connectedPlayerAccount;

		connectedPlayerAccount.lastSeenInTicks = TimeHelper.tick;

		// this.player.gx += this.player.x;
		// this.player.gy += this.player.y;

		// this.player.x = 0;
		// this.player.y = 0;

		addToConnections();
		sendMapChunk(player.x, player.y);
		// var id = player.p_id;
		// send(LINEAGE,['$id eve=$id']);
		send(TOOL_SLOTS, ["0 1000"]);

		// send PU and FRAME also to the connection --> therefore make sure that addToConnections is called first
		SendToMeAllClosePlayers(player, true);
		sendToMeAllPlayerNames();
		sendToMeAllLineages();
		sendToMeAllFollowings();
		sendToMeAllExiles();

		player.sendFoodUpdate();

		// this.send(ClientTag.LOCATION_SAYS,['0 100 ! 30']);

		if (player.mother != null) this.sendMapLocation(player.mother, 'MOTHER', 'leader');

		send(FRAME, null, true);

		// bID DISPLAY_NAME
		var badbiomes = '';
		badbiomes += '${BiomeTag.SNOWINGREY} MOUNTAIN\n';
		badbiomes += '${BiomeTag.RIVER} RIVER\n';
		badbiomes += '${BiomeTag.OCEAN} OCEAN\n';
		badbiomes += '${BiomeTag.SWAMP} SWAMP\n';
		
		send(BAD_BIOMES, [badbiomes]);
	}

	public static function getConnections():Array<Connection> {
		return connections;
	}

	public static function getAis():Array<ServerAi> {
		return ais;
	}

	public static function addAi(ai:ServerAi) {
		ais.push(ai);
	}

	public static function removeAi(ai:ServerAi) {
		ais.remove(ai);
	}

	public static function getLivingHumans():Array<GlobalPlayerInstance> {
		var livingHumans = new Array<GlobalPlayerInstance>();

		for (c in connections) {
			if (c.player.deleted) continue;
			livingHumans.push(c.player);
		}

		return livingHumans;
	}

	public static function getLivingAis():Array<ServerAi> {
		var livingAis = new Array<ServerAi>();

		for (ai in ais) {
			if (ai.player.deleted) continue;
			livingAis.push(ai);
		}

		return livingAis;
	}

	public static function SendLocationToAllClose(tx:Int, ty:Int, text:String) {
		for (c in connections) {
			// since player has relative coordinates, transform them for player
			var player = c.player;
			var targetX = WorldMap.world.transformX(player, tx);
			var targetY = WorldMap.world.transformY(player, ty);

			if (c.player.isClose(targetX, targetY, 20) == false) continue;
			c.send(ClientTag.LOCATION_SAYS, ['${targetX} ${targetY} $text']);
		}
	}

	public static function SendUpdateToAllClosePlayers(playerToSend:GlobalPlayerInstance, isPlayerAction:Bool = true, sendFrame:Bool = true) {
		try {
			playerToSend.MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed(); // TODO better change, since it can mess with other threads

			for (c in connections) {
				// since player has relative coordinates, transform them for player
				//var targetX = player.tx - c.player.gx;
				//var targetY = player.ty - c.player.gy;
				var player = c.player;
				var targetX = WorldMap.world.transformX(player, playerToSend.tx);
				var targetY = WorldMap.world.transformY(player, playerToSend.ty);

				// update only close players except if player is deleted (death)
				if (playerToSend.deleted == false
					&& c.player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsClose) == false) continue;

				c.send(PLAYER_UPDATE, [playerToSend.toRelativeData(c.player)], isPlayerAction);
				if (sendFrame) c.send(FRAME, null, isPlayerAction);
			}

			for (ai in ais) {
				ai.ai.playerUpdate(playerToSend);
			}
		} catch (ex)
			trace(ex);
	}

	public function sendToMeAllClosePlayers(sendMovingPlayer:Bool = false, isPlayerAction:Bool = true) {
		return SendToMeAllClosePlayers(this.player, sendMovingPlayer, isPlayerAction);
	}

	public static function SendToMeAllClosePlayers(player:GlobalPlayerInstance, sendMovingPlayer:Bool = false, isPlayerAction:Bool = true) {
		if (player.isDeleted()) return;

		try {
			var connection = player.connection;

			// player.MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed(); // TODO better change, since it can mess with other threads. A good idea maybe to use a global player mutex for all players

			for (c in connections) {
				connection.sendToMePlayerInfo(c.player, sendMovingPlayer, isPlayerAction);
			}

			for (ai in ais) {
				connection.sendToMePlayerInfo(ai.player, sendMovingPlayer, isPlayerAction);
			}

			player.connection.send(FRAME, null, isPlayerAction);
		} catch (ex)
			trace(ex);
	}

	private function sendToMePlayerInfo(playerToSend:GlobalPlayerInstance, sendMovingPlayer:Bool = true, isPlayerAction:Bool = true) {
		if (playerToSend.deleted) return;
		if (playerToSend.isHeld()) return;

		// since player has relative coordinates, transform them for player
		//var targetX = playerToSend.tx - player.gx;
		//var targetY = playerToSend.ty - player.gy;
		var player = this.player;
		var targetX = WorldMap.world.transformX(player, playerToSend.tx);
		var targetY = WorldMap.world.transformY(player, playerToSend.ty);

		// update only close players
		// TODO vanilla client breaks if /LEADER is used and leader is out of range so dont send out of range for now
		/*if (player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsClose) == false) {
			player.connection.send(PLAYER_OUT_OF_RANGE, ['${playerToSend.p_id}'], isPlayerAction);
			return;
		}*/

		if (playerToSend.isMoving()) {
			if (sendMovingPlayer) {
				player.connection.send(PLAYER_UPDATE, [playerToSend.toRelativeData(player)], isPlayerAction);
				var moveString = playerToSend.moveHelper.generateRelativeMoveUpdateString(player);
				player.connection.send(PLAYER_MOVES_START, [moveString]);
			}

			// sending moving player again creates otherwise a display bug
		} else {
			player.connection.send(PLAYER_UPDATE, [playerToSend.toRelativeData(player)], isPlayerAction);
		}

		player.connection.send(ClientTag.NAME, ['${playerToSend.p_id} ${playerToSend.name} ${playerToSend.lineage.getFullName(true, true)}']);
	}

	public function sendNameToAll() {
		var player = this.player;

		//trace('TEST Naming sendNameToAll ${player.p_id} ${player.name} ${player.lineage.getFullName(true, true)}');

		for (c in Connection.getConnections()) {
			c.send(ClientTag.NAME, ['${player.p_id} ${player.name} ${player.lineage.getFullName(true, true)}']);
		}
	}

	public function sendToMeAllPlayerNames() {
		for (c in Connection.getConnections()) {
			var player = c.player;

			this.send(ClientTag.NAME, ['${player.p_id} ${player.name} ${player.lineage.getFullName(true, true)}']);
			// this.send(ClientTag.NAME,['${player.p_id} ${player.name} ${player.familyName}']);
		}

		for (c in ais) {
			var player = c.player;
			this.send(ClientTag.NAME, ['${player.p_id} ${player.name} ${player.lineage.getFullName(true, true)}']);
			// this.send(ClientTag.NAME,['${player.p_id} ${player.name} ${player.familyName}']);
		}
	}

	// fathers are not supported by client
	public function sendToMeAllLineages() {
		for (c in Connection.getConnections()) {
			var lineageString = c.player.lineage.createLineageString();
			this.send(ClientTag.LINEAGE, [lineageString]);
		}

		for (c in ais) {
			var lineageString = c.player.lineage.createLineageString();
			this.send(ClientTag.LINEAGE, [lineageString]);
		}
	}

	/* FOLLOWING (FW): follower_id leader_id leader_color_index
		Provides list of people following other people.
		If leader is -1, that person follows no one
		Leader color index specifies leader's badge color from a fixed color list */
	public function sendFollowing(player:GlobalPlayerInstance) {
		var leader = player.getTopLeader();
		// var leaderId = leader == null ? -1 : leader.p_id; // TODO not sure if client wants top leader or next leader
		var leaderId = player.followPlayer == null ? -1 : player.followPlayer.p_id;
		var leaderBadgeColor = leader == null ? player.leaderBadgeColor : leader.leaderBadgeColor;

		send(FOLLOWING, ['${player.p_id} $leaderId $leaderBadgeColor']);
	}

	public static function SendFollowingToAll(player:GlobalPlayerInstance) {
		for (c in Connection.getConnections()) {
			c.sendFollowing(player);
		}
	}

	public function sendToMeAllFollowings() {
		for (c in Connection.getConnections()) {
			sendFollowing(c.player);
		}

		for (c in ais) {
			sendFollowing(c.player);
		}
	}

	public function sendToMeAllExiles() {
		for (p in GlobalPlayerInstance.AllPlayers) {
			sendFullExileListToMe(p);
		}
	}

	/**
		EX
		exile_target_id exiler_id
		exile_target_id exiler_id
		exile_target_id exiler_id
		...
		exile_target_id exiler_id
		#

		Provides list of people exiled by another person.

		If someone's exile list has changed (who's exiling them), then their whole
		exile list is sent.

		Each target's list is prefaced by this line:

		exile_target_id -1

		This indicates that the target's complete list is coming, and the client-side
		list should be cleared in preparation for this.


		If a target is newly exiled by no one, they show up as:

		exile_target_id -1
	**/
	public static function SendExileToAll(exiler:GlobalPlayerInstance, target:GlobalPlayerInstance) {
		for (c in Connection.getConnections()) {
			c.send(EXILED, ['${target.p_id} ${exiler.p_id}']);
		}
	}

	public function sendFullExileListToMe(target:GlobalPlayerInstance) {
		var list = CreateFullExileList(target);

		if (StringTools.contains(list, '\n') == false) return;

		send(EXILED, [list]);
	}

	public static function SendFullExileListToAll(target:GlobalPlayerInstance) {
		var list = CreateFullExileList(target);

		trace('EXILE LIST: $list');

		for (c in Connection.getConnections()) {
			c.send(EXILED, [list]);
		}
	}

	private static function CreateFullExileList(target:GlobalPlayerInstance):String {
		var list = '${target.p_id} -1';

		for (p in target.exiledByPlayers) {
			var tmp = '\n${target.p_id} ${p.p_id}';
			list += tmp;
		}

		return list;
	}

	public static function SendTransitionUpdateToAllClosePlayers(player:GlobalPlayerInstance, tx:Int, ty:Int, newFloorId:Int, newTileObject:Array<Int>,
			doTransition:Bool, isPlayerAction:Bool = true) {
		try {
			for (c in connections) {
				// since player has relative coordinates, transform them for player
				//var targetX = tx - c.player.gx;
				//var targetY = ty - c.player.gy;
				var p = c.player;
				var targetX = WorldMap.world.transformX(p, tx);
				var targetY = WorldMap.world.transformY(p, ty);

				// update only close players
				if (c.player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsClose) == false) continue;

				c.send(PLAYER_UPDATE, [player.toRelativeData(c.player)]);

				if (doTransition) {
					c.sendMapUpdate(targetX, targetY, newFloorId, newTileObject, (-1) * player.p_id);
				} else {
					c.sendMapUpdate(targetX, targetY, newFloorId, newTileObject, player.p_id);
				}

				c.send(FRAME);
			}

			for (c in ais) {
				// since player has relative coordinates, transform them for player
				//var targetX = tx - c.player.gx;
				//var targetY = ty - c.player.gy;
				var p = c.player;
				var targetX = WorldMap.world.transformX(p, tx);
				var targetY = WorldMap.world.transformY(p, ty);

				// update only close players
				if (c.player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsClose) == false) continue;

				c.ai.mapUpdate(tx, ty);
			}
		} catch (ex)
			trace(ex);
	}

	public static function SendMapUpdateToAllClosePlayers(tx:Int, ty:Int, obj:Array<Int>) {
		try {
			var floorId = Server.server.map.getFloorId(tx, ty);

			for (c in connections) {
				// since player has relative coordinates, transform them for player
				var player = c.player;
				var targetX = WorldMap.world.transformX(player, tx);
				var targetY = WorldMap.world.transformY(player, ty);

				// update only close players
				if (c.player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsCloseForMapChanges) == false) continue;

				try {
					c.sendMapUpdate(targetX, targetY, floorId, obj, -1, false);
					c.send(FRAME, null, false);
				} catch (ex) {
					trace(ex);
				}
			}
		} catch (ex)
			trace(ex);
	}

	public static function SendAnimalMoveUpdateToAllClosePlayers(fromTx:Int, fromTy:Int, toTx:Int, toTy:Int, fromObj:Array<Int>, toObj:Array<Int>,
			speed:Float) {
		try {
			var floorIdTarget = Server.server.map.getFloorId(toTx, toTy);
			var floorIdFrom = Server.server.map.getFloorId(fromTx, fromTy);
			var world = WorldMap.world;

			for (c in connections) {
				var player = c.player;

				// since player has relative coordinates, transform them for player				
				var fromX = world.transformX(player, fromTx);
				var fromY = world.transformY(player, fromTy);
				var toX = world.transformX(player, toTx);
				var toY = world.transformY(player, toTy);
				//trace('fromX: $fromXTmp ==> $fromX fromY: $fromYTmp ==> $fromY');

				// update only close players
				if (player.isClose(toX, toY, ServerSettings.MaxDistanceToBeConsideredAsCloseForMapChanges) == false
					&& player.isClose(fromX, fromY, ServerSettings.MaxDistanceToBeConsideredAsCloseForMapChanges) == false) continue;

				// c.mutex.acquire(); // do all in one frame

				c.sendMapUpdate(fromX, fromY, floorIdFrom, fromObj, -1, false);
				c.sendMapUpdateForMoving(toX, toY, floorIdTarget, toObj, -1, fromX, fromY, speed);
				c.send(FRAME, null, false);

				// c.mutex.release();
			}

			for (c in ais) {
				var player = c.player;

				// since player has relative coordinates, transform them for player
				var fromX = world.transformX(player, fromTx);
				var fromY = world.transformY(player, fromTy);
				var toX = world.transformX(player, toTx);
				var toY = world.transformY(player, toTy);

				// update only close players
				if (player.isClose(toX, toY, ServerSettings.MaxDistanceToBeConsideredAsClose) == false
					&& player.isClose(fromX, fromY, ServerSettings.MaxDistanceToBeConsideredAsClose)) continue;

				c.ai.mapUpdate(fromTx, fromTy, true);
			}
		} catch (ex)
			trace(ex);
	}

	public static function SendMoveUpdateToAllClosePlayers(player:GlobalPlayerInstance, isPlayerAction:Bool = true) {
		try {
			for (c in connections) {
				// since player has relative coordinates, transform them for player
				//var targetX = player.tx - c.player.gx;
				//var targetY = player.ty - c.player.gy;
				var p = c.player;
				var targetX = WorldMap.world.transformX(p, player.tx);
				var targetY = WorldMap.world.transformY(p, player.ty);

				// update only close players
				if (c.player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsClose) == false) continue;

				var moveString = player.moveHelper.generateRelativeMoveUpdateString(c.player);

				c.send(PLAYER_MOVES_START, [moveString]);

				// c.send(PLAYER_MOVES_START,['${player.p_id} ${targetX} ${targetY} ${totalMoveTime} $eta ${trunc} ${moveString}']);

				c.send(FRAME);
			}

			for (c in ais) {
				// since player has relative coordinates, transform them for player
				//var targetX = player.tx - c.player.gx;
				//var targetY = player.ty - c.player.gy;
				var p = c.player;
				var targetX = WorldMap.world.transformX(p, player.tx);
				var targetY = WorldMap.world.transformY(p, player.ty);

				// update only close players
				if (c.player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsClose) == false) continue;

				c.ai.playerMove(player, player.tx, player.ty);
			}
		} catch (ex)
			trace(ex);
	}

	private function addToConnections() {
		// GlobalPlayerInstance.AllPlayerMutex.acquire();

		// it copies the connection array to be thread save
		// other threads should meanwhile be able to iterate on connections.
		var newConnections = [];

		newConnections.push(this);

		for (c in connections) {
			newConnections.push(c);
		}

		connections = newConnections;

		// GlobalPlayerInstance.AllPlayerMutex.release();
	}

	public function close() {
		GlobalPlayerInstance.AllPlayerMutex.acquire();

		try {
			// set all stuff null so that nothing is hanging around
			// this.player.delete();

			// it copies the connection array to be thread save
			// other threads should meanwhile be able to iterate on connections. replaces: //connections.remove(this);
			var newConnections = [];

			// TODO remove only from connections if dead or even better use as AI?

			for (c in connections) {
				if (c == this) continue;

				newConnections.push(c);
			}

			connections = newConnections;

			running = false;
			sock.close();
			this.sock = null;
			if (this.player.deleted == false) {
				this.serverAi = new ServerAi(this.player);
			}
		} catch (ex) {
			trace(ex);
		}

		GlobalPlayerInstance.AllPlayerMutex.release();
	}

	// KA x y#
	public function keepAlive() {}

	// DIE x y#
	public function die() {
		this.close();
	}

	/**
		FL
		p_id face_left
		p_id face_left
		...
		p_id face_left
		#

		Tells player about other players that have flipped.  face_left is true if facing
		left, false if facing right.  Only sent in response to stationary player flip
		requests (clients should still auto-flip players based on movement).
	**/
	public function flip(x:Int, y:Int) {
		for (c in connections) {
			// since player has relative coordinates, transform them for player
			var targetX = player.tx - c.player.gx;
			var targetY = player.ty - c.player.gy;

			// update only close players
			if (c.player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsClose) == false) continue;

			var face_left = x < player.x ? 'true' : 'false';
			c.send(FLIP, ['${player.p_id} $face_left']);
		}
	}

	public function sendSayToAllClose(text:String, curse:Int) {
		for (c in connections) {
			// since player has relative coordinates, transform them for player
			var targetX = player.tx - c.player.gx;
			var targetY = player.ty - c.player.gy;

			// only close players
			if (c.player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsCloseForSay) == false) continue;

			c.send(PLAYER_SAYS, ['${player.id}/$curse $text']);
			c.send(FRAME);
		}

		for (ai in Connection.getAis()) {
			ai.ai.say(player, curse == 1, text);
		}
	}

	public function sendMapChunk(x:Int, y:Int, width:Int = 32, height:Int = 30) {
		if (sock == null) return;

		if(ServerSettings.DebugSend) trace('sendMapChunk');

		// this.mutex.acquire();

		try {
			x -= Std.int(width / 2);
			y -= Std.int(height / 2);

			var map = server.map.getChunk(x + player.gx, y + player.gy, width, height).toString();
			var uncompressed = Bytes.ofString(map);
			var bytes = haxe.zip.Compress.run(uncompressed, -1);

			send(MAP_CHUNK, ['$width $height $x $y', '${uncompressed.length} ${bytes.length}']);
			sock.output.write(bytes);
			// send(VALLEY_SPACING,["40 40"]); // TODO what is this for?
			// send(FRAME);
		} catch (ex) {
			trace(ex);
		}

		// this.mutex.release();
	}

	/*
		MX
		x y new_floor_id new_id p_id
		#

		Or 

		MX
		x y new_floor_id new_id p_id old_x old_y speed
		#
	 */
	public function sendMapUpdate(x:Int, y:Int, newFloorId:Int, newObjectId:Array<Int>, playerId:Int, isPlayerAction:Bool = true) {
		if (serverAi != null) return;
		send(MAP_CHANGE, ['$x $y $newFloorId ${MapData.stringID(newObjectId)} $playerId'], isPlayerAction);
	}

	public function sendMapUpdateForMoving(toX:Int, toY:Int, newFloorId:Int, newObjectId:Array<Int>, playerId:Int, fromX:Int, fromY:Int, speed:Float) {
		if (serverAi != null) return;
		send(MAP_CHANGE, [
			'$toX $toY $newFloorId ${MapData.stringID(newObjectId)} $playerId $fromX $fromY $speed'
		], false);
	}

	public static function SendDyingToAll(dyingPlayer:GlobalPlayerInstance) {
		for (c in connections) {
			c.send(ClientTag.DYING, ['${dyingPlayer.p_id}']);
		}
	}

	/**
		PE
		p_id emot_index ttl_sec
		p_id emot_index
		p_id emot_index ttl_sec
		...
		p_id emot_index ttl_sec
		#                
				ttl_sec is optional, and specifies how long the emote should be shown, in
				seconds, client-side.  If it is omitted, the client should display the emotion
				for the standard amount of time.  If ttl is -1, this emot is permanent and
				should layer with other permanent and non-permanent emots.

				If ttl is -2, the emot is permanent but not new, so sound shoudl be skipped.
	**/
	public static function SendEmoteToAll(fromPlayer:GlobalPlayerInstance, id:Int, seconds:Int = -10) {
		for (toConnection in connections) {
			Macro.exception(DoHumanPlayerEmote(fromPlayer, toConnection, id, seconds));
		}

		try {
			for (c in ais) {
				// since player has relative coordinates, transform them for player
				//var targetX = fromPlayer.tx - c.player.gx;
				//var targetY = fromPlayer.ty - c.player.gy;
				var p = c.player;
				var targetX = WorldMap.world.transformX(p, fromPlayer.tx);
				var targetY = WorldMap.world.transformY(p, fromPlayer.ty);

				// update only close players
				if (c.player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsClose) == false) continue;

				c.ai.emote(fromPlayer, id);
			}
		} catch (ex)
			trace(ex);
	}

	private static function DoHumanPlayerEmote(fromPlayer:GlobalPlayerInstance, toConnection:Connection, id:Int, seconds:Int = -10) {
		// since player has relative coordinates, transform them for player
		//var targetX = fromPlayer.tx - toConnection.player.gx;
		//var targetY = fromPlayer.ty - toConnection.player.gy;
		var p = toConnection.player;
		var targetX = WorldMap.world.transformX(p, fromPlayer.tx);
		var targetY = WorldMap.world.transformY(p, fromPlayer.ty);

		// update only close players
		if (toConnection.player.isClose(targetX, targetY, ServerSettings.MaxDistanceToBeConsideredAsClose) == false) return;

		if (seconds < 3) toConnection.send(PLAYER_EMOT, ['${fromPlayer.p_id} $id']); else
			toConnection.send(PLAYER_EMOT, ['${fromPlayer.p_id} $id $seconds']);

		toConnection.send(FRAME);
	}

	public function sendPlayerUpdate(forcePosition:Bool = false) {
		player.forced = forcePosition;
		player.connection.send(PLAYER_UPDATE, [player.toData()]);
		player.forced = false;
	}

	private static var lastSend:String = "";

	public function send(tag:ClientTag, data:Array<String> = null, isPlayerAction:Bool = true, force:Bool = false) {
		// send frame only once every time step
		if(tag == FRAME){
			if(force && sendFrame == false) return; // no frame to send
			if(force) sendFrame = false;
			else {
			 sendFrame = true;
			 return;
			}
		}

		/*
			if(tag == PLAYER_UPDATE)
			{
				var strings = data[0].split(' ');
				var playerId = Std.parseInt(strings[0]);
				var player = GlobalPlayerInstance.AllPlayers[playerId];

				for (c in connections)
				{
					var cp = c.player;
					var xx = player.tx - cp.gx;
					var yy = player.ty - cp.gy;
					var text = 'M${playerId}x${xx}y${yy}';

					trace('Location: $playerId x${player.tx}y${player.ty}');

					//c.send(ClientTag.LOCATION_SAYS, ['${xx} ${yy} $text']);
				}
		}*/

		// if(playerAccount != null && playerAccount.isAi)
		if (this.sock == null) {
			try {
				// TODO call serverAi
			} catch (ex) {
				trace(ex);
			}

			return;
		}

		var message = '';

		try {
			// TODO why this creates sometimes Access Violation?
			var message = data != null ? '$tag\n${data.join("\n")}\n#' : '$tag\n#';

			// if(tag == MAP_CHANGE) trace('send: ${message}');

			if(ServerSettings.DebugSend){
				var tmpString = StringTools.replace(message, "\n", "\t");
				trace('send: ${Connection.debugText} ${tmpString}');
			}

			sendHelper(message, isPlayerAction);
		} catch (ex) {
			var tmpString = StringTools.replace(message, "\n", "\t");

			if ('$ex' == 'Eof') {
				this.close();
			}

			trace('WARNING Send: $tmpString ' + ex);
		}

		// this.mutex.release();
	}

	private function sendHelper(message:String, isPlayerAction:Bool = true) {
		sock.output.writeString(message);

		// if(StringTools.contains(string, 'LR')) trace(string);
		// if(StringTools.contains(string, 'LR')) trace(data);
		// if(ServerSettings.TraceSend && tag != MAP_CHANGE && tag != FRAME)
		if ((ServerSettings.TraceSendPlayerActions && isPlayerAction)
			|| (ServerSettings.TraceSendNonPlayerActions && isPlayerAction == false)) {
			var tmpString = StringTools.replace(message, "\n", "\t");
			trace("Send: " + tmpString);
		}
	}

	public function sendPong(unique_id:String) {
		// this.mutex.acquire();

		try {
			var tmpString = '$PONG\n$unique_id#';

			sock.output.writeString(tmpString);

			if (ServerSettings.TraceSendPlayerActions) trace("Send: " + tmpString);
		} catch (ex) {
			trace(ex);
		}

		// this.mutex.release();
	}

	public function sendGlobalMessage(message:String) {
		if (player.isAi()) return;
		if (message.length < 1) return;

		message = message.toUpperCase();
		message = StringTools.replace(message, ' ', '_');

		if (timeToWaitBeforeNextMessageSend > 0) {
			messageQueue.push(message);
			return;
		}

		timeToWaitBeforeNextMessageSend = ServerSettings.SecondsBetweenMessages;

		send(ClientTag.GLOBAL_MESSAGE, [message]);
	}

	public static function SendGlobalMessageToAll(message:String) {
		for (c in connections) {
			c.sendGlobalMessage(message);
		}
	}

	/**
		BABY_WIGGLE (BW): p_id
		A list of player IDs that are babies who just started wiggling.
	**/
	public function sendWiggle(player:GlobalPlayerInstance) {
		send(BABY_WIGGLE, ['${player.p_id}'], true);
	}

	/**( 
		"PS\n"
		"%d/0 OUTSIDER %s IS MY NEW FOLLOWER "
		"*visitor %d *map %d %d\n#",
		otherToFollow->id,
		name,
		nextPlayer->id,
		nextPlayer->xs - 
		otherToFollow->birthPos.x,
		nextPlayer->ys - 
		otherToFollow->birthPos.y );
	**/

	public function sendMapLocation(toPlayer:GlobalPlayerInstance, text1:String, text2:String) {
		var player = this.player;
		//var message = '${player.p_id}/0 $text1 *$text2 ${toPlayer.p_id} *map ${toPlayer.tx - player.gx} ${toPlayer.ty - player.gy}';
		var message = '${player.p_id}/0 $text1 *$text2 ${toPlayer.p_id} *map ${WorldMap.world.transformX(player, toPlayer.tx)} ${WorldMap.world.transformY(player, toPlayer.ty)}';
		
		//trace('MAPSAY: $message');

		this.send(ClientTag.PLAYER_SAYS, [message], true);
	}

	public function sendLeader() {
		var player = this.player;
		var leader = player.getTopLeader();

		if (leader == null) leader == player;

		//trace('LEADER: ${player.id}-->${leader.id} deleted: ${leader.isDeleted()}');
		//send(PLAYER_UPDATE, [leader.toRelativeData(player)], false);
		this.sendMapLocation(leader, "LEADER", "leader");
	}

	/**
		(OW)
		OW
		x y p_id p_id p_id ... p_id
		#

		Provides owner list for position x y
	**/
	public function sendOwners(x:Int, y:Int) {
		var tx = player.gx + x;
		var ty = player.gy + y;
		var message = '$x $y';
		var helper = WorldMap.world.getObjectHelper(tx, ty);

		if (helper.hasOwners() == false) {
			if (helper.objectData.isOwned == false) return;

			// give ownership to the player that found this not owned gate
			helper.addOwner(player);

			WorldMap.world.setObjectHelper(tx, ty, helper);
		}

		message += helper.createOwnerString();

		// trace('OWNERS: $message');

		this.send(ClientTag.OWNER_LIST, [message], false);
	}

	/**
		(LR)
		LR
		tool_id tool_id ... tool_id 
		#

		Report about new tool objects that have just been learned

		Each tool_id is an object id.
	**/
	public function sendLearnedTool(id:Int) {
		player.connection.send(LEARNED_TOOL_REPORT, ['${id}']);
		// player.connection.send(TOOL_SLOTS,['1 100']);
	}

	/**(GV)
		GV
		x y p_id
		#

		Grave at x y belongs to player p_id.
	**/
	public static function SendGraveInfoToAll(grave:ObjectHelper) {
		for (c in connections) {
			//var x = grave.tx - c.player.gx;
			//var y = grave.ty - c.player.gy;
			var p = c.player;
			var x = WorldMap.world.transformX(p, grave.tx);
			var y = WorldMap.world.transformY(p, grave.ty);

			c.send(GRAVE, ['$x $y ${grave.getCreatorId()}']);
		}
	}

	/**
		(GO)
		GO
		x y p_id po_id death_age underscored_name mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id
		#

		Provides info about an old grave that wasn't created during your lifetime.
		In response to a GRAVE message from client.

		underscored_name is name with spaces replaced by _
		If player has no name, this will be ~ character instead.

		death_age is how long the player has been dead (in in-game years)

		Same semantics for eve= tag at end as for LN message.
	**/
	public function sendGraveInfo(x:Int, y:Int) {
		var tx = player.gx + x;
		var ty = player.gy + y;
		var grave = WorldMap.world.getObjectHelper(tx, ty);
		var creatorLinage = grave.getLinage();

		if (creatorLinage == null) return;
		var linageString = creatorLinage.createLineageString(false);
		var message = '$x $y ${creatorLinage.myId} ${creatorLinage.po_id} ${creatorLinage.getDeadSince()} ${creatorLinage.getFullName(true)} ${linageString}';

		trace('GRAVE: $message');

		send(GRAVE_OLD, [message]);
	}

	/*
		(GM)
		GM
		xs ys xd yd swap_dest
		#


		Grave at xs,ys moved to xd,yd.

		If optional swap_dest parameter is 1, it means that some other grave at 
		destination is in mid-air.  If 0, not
	 */
	/**
		(CU)
		CU
		p_id level word
		p_id level
		...
		p_id level word
		#

		List of player ids that are cursed, and their curse level
		word is optional, which is a persistent name tag for this person.
	**/
	public static function SendCurseToAll(player:GlobalPlayerInstance, level:Int = 1) {
		for (c in connections) {
			c.send(CURSED, ['${player.p_id} $level']);
		}
	}

	public function doTime(passedTimeInSeconds:Float) {
		if (this.timeToWaitBeforeNextMessageSend > 0) {
			this.timeToWaitBeforeNextMessageSend -= passedTimeInSeconds;
		} else if (this.messageQueue.length > 0) {
			this.sendGlobalMessage(this.messageQueue.shift());
		}
	}
}
#end
