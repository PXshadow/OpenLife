package openlife.settings;

import sys.io.File;
import haxe.Json;
import sys.FileSystem;
import sys.io.File;

class OpenLifeData {
	public static function getData():OpenLifeDataType {
		if (!FileSystem.exists("data.json")) return getDefault();
		var data:OpenLifeDataType = cast Json.parse(File.getContent("data.json"));
		return data;
	}

	public static function getDefault():OpenLifeDataType {
		return {
			relay: true,
			combo: 0,
			syncSettings: false,
			script: "Script.hx",
			debug: false
		};
	}
}

typedef OpenLifeDataType = {relay:Bool, combo:Int, syncSettings:Bool, script:String, debug:Bool}
