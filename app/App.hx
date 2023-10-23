package;

import openlife.auto.Ai;
import haxe.Exception;
import openlife.client.Client;
import haxe.Json;
import sys.io.File;
import openlife.data.object.ObjectData;
import haxe.ds.Vector;
import openlife.auto.Automation;
import sys.FileSystem;
import openlife.resources.ObjectBake;
import openlife.settings.Settings;
import haxe.ds.Map;
import openlife.data.object.player.PlayerInstance;
import openlife.engine.Program;
import openlife.data.map.MapInstance;
import openlife.engine.*;
import openlife.data.object.player.PlayerMove;
import openlife.data.map.MapChange;
import openlife.auto.Overseer;
import openlife.settings.OpenLifeData;
import openlife.settings.OpenLifeData.OpenLifeData;
import ClientAi.Bot;

using StringTools;

class App {
	public static var vector:Vector<Int>;

	var followingId:Int = -1;

	static var overseer = new Overseer();

	public function new() {
		// openlife.auto.actions.
		Engine.dir = Utility.dir();
		vector = Bake.run();
		// start program
		var data = OpenLifeData.getData();
		var config = new Settings().config();
		if (data.syncSettings) {
			File.saveContent("data.json", Json.stringify(data));
		}
		if (!FileSystem.exists("config.json")) {
			File.saveContent("config.json", Json.stringify(config));
		} else {
			config = Json.parse(File.getContent("config.json"));
		}
		if (!data.relay && data.combo > 0) {
			// // multiple ais from combo
			// if (!FileSystem.exists("combo.txt")) throw "no combo list found";
			// var list = File.getContent("combo.txt").split("\r\n");
			// var ais:Array<Ai> = [];
			// if (data.combo > list.length) data.combo = list.length;
			// for (i in 0...data.combo) {
			// 	var config = configClone(config);
			// 	var data = list[i].split(":");
			// 	config.email = data[0];
			// 	config.key = data[1];
			// 	var client = new Client();
			// 	client.config = config;
			// 	var ai = new Ai(client);
			// 	ai.connect(false, false);
			// 	bots.push(bot);
			// 	Sys.sleep(0.1);
			// }
			// trace("finish going through combo list length: " + data.combo);
			// while (true) {
			// 	overseer.run(bots);
			// 	Sys.sleep(1 / 120);
			// }
		} else {
			var client = new Client();
			client.config = config;
			var bot = new Bot(client);
			bot.relayPort = 8000;
			bot.connect(false, data.relay);
			while (true) {
				bot.update();
				Sys.sleep(1 / 20);
			}
		}
	}

	private function configClone(cred:ConfigData):ConfigData {
		return {
			email: cred.email,
			key: cred.key,
			ip: cred.ip,
			port: cred.port,
			tutorial: cred.tutorial,
			seed: cred.seed,
			twin: cred.twin,
			legacy: cred.legacy
		};
	}
}
