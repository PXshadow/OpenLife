package openlife.server;

import sys.thread.Mutex;
import openlife.auto.Ai;
import openlife.auto.PlayerInterface;
import openlife.data.transition.TransitionImporter;
import openlife.macros.Macro;
import openlife.resources.Resource;
import openlife.settings.ServerSettings;
#if (target.threaded)
import haxe.Timer;
import haxe.ds.Vector;
import haxe.io.Bytes;
import haxe.io.Path;
import openlife.client.ClientTag;
import openlife.data.Pos;
import openlife.data.object.ObjectData;
import openlife.engine.Engine;
import openlife.engine.Utility;
import openlife.resources.ObjectBake;
import openlife.settings.Settings;
import sys.FileSystem;
import sys.io.File;
import sys.net.Host;
import sys.net.Socket;
import sys.thread.Thread;

using openlife.server.MoveHelper;

class Server {
	public static var server:Server;

	public var canStart = false;

	public var map:WorldMap; // THE WORLD

	public var playerIndex:Int = 2; // used for giving new IDs to players // better start with 2 since -1 has other use in MX update

	private static var mutex:Mutex = new Mutex();

	public static function Acquire() {
		mutex.acquire();
	}

	public static function Release() {
		mutex.release();
	}

	public static function main() {
		WebServer.Start(); // Start WebServer to display statistics

		// while (true) Sys.sleep(1);

		Sys.println("Starting OpenLife Server" #if debug + " in debug mode" #end);

		if (ServerSettings.debug) trace('Debug Mode: ${ServerSettings.debug}');

		server = new Server();

		var aiCount = Connection.getAis().length;

		if (server.canStart) trace('loaded $aiCount Ais from ${ServerSettings.NumberOfAis}');
		// else trace('Cannot start server please fix Save!');
		if (server.canStart) TimeHelper.DoTimeLoop();
	}

	public function new() {
		server = this;
		// SerializeHelper.createReadWriteFile();

		if (ServerSettings.readFromFile() == false) {
			var dir = './${ServerSettings.SaveDirectory}/';
			var path = dir + "ServerSettings.txt";

			if (FileSystem.exists(path) == false) ServerSettings.writeToFile();
		}

		if (ServerSettings.dumpOutput) {
			var dump = File.append("dump.txt", false);
			haxe.Log.trace = (v:Dynamic, ?infos:haxe.PosInfos) -> {
				dump.writeString('$v\n');
			}
		}

		Engine.dir = Utility.dir();

		ObjectData.DoAllTheObjectInititalisationStuff();

		TransitionImporter.DoAllInititalisationStuff();

		// do all the map inititalisation stuff
		map = new WorldMap();

		var dir = './${ServerSettings.SaveDirectory}/';
		var saveExists = sys.FileSystem.exists(dir + "lastDataNumber.txt");
		if (saveExists == false) saveExists = sys.FileSystem.exists(dir + "lastDataNumber.bak");

		if (ServerSettings.GenerateMapNew || saveExists == false) {
			if (saveExists == false) trace('Save ${dir + "lastDataNumber.txt and .bak"} could not be found! Generate new Map!');
			map.generate();
			map.writeToDisk();
		} else {
			this.canStart = map.readFromDisk();
			if (this.canStart == false) {
				trace('could not load Save! Please fix save or delete lastDataNumber.txt and .bak to generate a new Map!');
				return;
				// map.generate();
				// map.writeToDisk();
			}
		}

		// prevent any blocking object on global starting position // TODO change
		var startObj = map.getObjectHelper(ServerSettings.startingGx, ServerSettings.startingGy, false);
		if (startObj != null) {
			if (startObj.blocksWalking()) map.setObjectId(ServerSettings.startingGx, ServerSettings.startingGy, [0]);
		}

		NamingHelper.ReadNames();

		// run run run Thread run run run
		var thread = new ThreadServer(this, 8005);
		Thread.create(function() {
			thread.create();
		});
	}

	public function process(connection:Connection, string:String) {
		// Sys.println(string); //log messages
		var index = string.indexOf(" ");
		if (index == -1) return;
		var tag = string.substring(0, index);
		string = string.substring(index + 1);
		var array = string.split(" ");
		if (array.length == 0) return;

		var input = new Array<String>();

		for (s in array) {
			if (s.length > 0) input.push(s);
		}

		if (ServerSettings.UseOneGlobalMutex) Server.Acquire();

		Macro.exception(message(connection, tag, input, string));

		if (ServerSettings.UseOneGlobalMutex) Server.Release();
	}

	/**
		KA x y#
		USE x y id i#
		BABY x y#
		BABY x y id#
		SELF x y i#
		UBABY x y i#
		UBABY x y i id#
		REMV x y i#
		SREMV x y c i#
		DROP x y c#
		KILL x y#
		KILL x y id#
		JUMP x y#
		EMOT x y e#
		DIE x y#
		GRAVE x y#
		OWNER x y#
		FORCE x y#
		PING x y unique_id#
		VOGS x y#
		VOGN x y#
		VOGP x y#
		VOGM x y#
		VOGI x y id#
		VOGT x y text# 
		VOGX x y#
		PHOTO x y seq#
		LEAD x y#
		UNFOL x y#
		FLIP x y#
	**/
	private function message(connection:Connection, tag:ServerTag, input:Array<String>, string:String) {
		if (ServerSettings.DebugIncomingCommands) trace('TAG: $tag $input $string');

		switch (tag) {
			case LOGIN: // A normal login is treated same as a reconnect
				connection.rlogin(input[0], input[1], input[2], input[3]);
			case RLOGIN: //
				connection.rlogin(input[0], input[1], input[2], input[3]);
			case DIE: // DIE x y#
				connection.die();
			case KA: // KA x y#
				connection.keepAlive();
			case FLIP: // FLIP x y#
				connection.flip(Std.parseInt(input[0]), Std.parseInt(input[1]));
			case PING: // PING x y unique_id#
				connection.sendPong(input[2]);
			case EMOT: // PE p_id emot_index ttl_sec
				connection.player.doEmote(Std.parseInt(input[2]));
			case SREMV: // SREMV x y c i#
				connection.player.specialRemove(Std.parseInt(input[0]), Std.parseInt(input[1]), Std.parseInt(input[2]),
					input.length > 3 ? Std.parseInt(input[3]) : null);
			case REMV: // REMV x y i#
				connection.player.remove(Std.parseInt(input[0]), Std.parseInt(input[1]), Std.parseInt(input[2]));
			case USE: // USE x y id i#
				connection.player.use(Std.parseInt(input[0]), Std.parseInt(input[1]), input.length > 3 ? Std.parseInt(input[3]) : -1,
					input.length > 2 ? Std.parseInt(input[2]) : 0);
			case DROP: // DROP x y c#
				connection.player.drop(Std.parseInt(input[0]), Std.parseInt(input[1]), Std.parseInt(input[2]));
			case SELF: // SELF x y i#
				connection.player.self(Std.parseInt(input[0]), Std.parseInt(input[1]), Std.parseInt(input[2]));
			case UBABY: // UBABY x y i id#
				connection.player.doOnOther(Std.parseInt(input[0]), Std.parseInt(input[1]), Std.parseInt(input[2]),
					input.length > 3 ? Std.parseInt(input[3]) : -1);
			case SAY: // PS p_id/isCurse text
				connection.player.say(string.substring(4));
			case BABY: // BABY x y# // BABY x y id#
				connection.player.doBaby(Std.parseInt(input[0]), Std.parseInt(input[1]), input.length > 2 ? Std.parseInt(input[2]) : -1);
			case JUMP: // JUMP x y#
				connection.player.jump();
			case LEAD: // LEAD 0 0#
				connection.sendLeader();
			case OWNER: // OWNER x y#
				connection.sendOwners(Std.parseInt(input[0]), Std.parseInt(input[1]));
			case GRAVE: // GRAVE x y#
				connection.sendGraveInfo(Std.parseInt(input[0]), Std.parseInt(input[1]));
			case KILL: // KILL x y# OR KILL x y id#
				connection.player.kill(Std.parseInt(input[0]), Std.parseInt(input[1]), input.length > 2 ? Std.parseInt(input[2]) : -1);
			case FORCE:
				connection.player.moveHelper.receivedForce(Std.parseInt(input[0]), Std.parseInt(input[1]));
			case MOVE: // PM p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0 ... xdeltN ydeltN
				var x = Std.parseInt(input[0]);
				var y = Std.parseInt(input[1]);
				var seq = Std.parseInt(input[2].substr(1));
				input = input.slice(3);
				var moves:Array<Pos> = [];

				for (i in 0...Std.int(input.length / 2)) {
					moves.push(new Pos(Std.parseInt(input[i * 2]), Std.parseInt(input[i * 2 + 1])));
				}

				connection.player.move(x, y, seq, moves);
			default:
				trace('TAG NOT FOUND! $tag');
		}
	}
}
#end
