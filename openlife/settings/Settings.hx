package openlife.settings;

import haxe.DynamicAccess;
import haxe.io.Path;
import openlife.client.Client;
import openlife.engine.Engine;
import openlife.resources.Resource;
#if (sys || nodejs)
import sys.FileSystem;
import sys.io.File;
import sys.io.FileOutput;
#end

@:expose
class Settings {
	@:isVar public var data(default, set):Data = {};

	function set_data(value:Data):Data {
		var a = value.keys();
		var b = data.keys();
		if (a.length > b.length) {
			var name = a[a.length - 1] + ".ini";
			var obj = value.get(name);
			// set settings
			var file = File.write(Engine.dir + "settings/" + name, false);
			file.writeString(obj);
			file.close();
		}
		return data = value;
	}

	public function new() {
		var path:String = Engine.dir + "settings/";
		if (!FileSystem.exists(path)) {
			FileSystem.createDirectory(Engine.dir + "settings");
		}
		for (name in FileSystem.readDirectory(path)) {
			Reflect.setField(data, Path.withoutExtension(name), File.getContent(path + name));
		}
	}

	var string:String;

	public function config():ConfigData {
		var config:ConfigData = {
			legacy: false,
			email: "test",
			key: "0000",
			ip: "localhost",
			port: 8005,
			seed: "",
			twin: "",
			tutorial: false
		};
		// settings to use infomation
		if (valid(data.get("email"))) config.email = string;
		if (valid(data.get("accountKey"))) config.key = string;
		if (valid(data.get("useCustomServer")) && string == "1") {
			if (valid(data.get("customServerAddress"))) config.ip = string;
			if (valid(data.get("customServerPort"))) config.port = Std.parseInt(string);
		}
		// by pass settings and force email and key if secret account
		#if secret
		trace("set secret");
		config.email = Secret.email;
		config.key = Secret.key;
		config.ip = Secret.ip;
		config.port = Secret.port;
		#end
		return config;
	}

	private inline function valid(obj:Dynamic):Bool {
		if (obj == null || obj == "") return false;
		string = cast obj;
		return true;
	}
}

@:expose
typedef ConfigData = {?legacy:Bool, ?tag:String, ?email:String, ?key:String, ip:String, ?port:Int, ?seed:String, ?tutorial:Bool, ?twin:String}

typedef Data = DynamicAccess<Dynamic>
