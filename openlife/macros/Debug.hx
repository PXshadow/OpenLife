package openlife.macros;

import openlife.settings.OpenLifeData;
import sys.FileSystem;

class Debug {
	#if macro
	public static function run() {
		if (!FileSystem.exists("data.json")) return;
		var data = OpenLifeData.getData();
		if (data.debug) {
			haxe.macro.Compiler.define("debug", "");
			// haxe.macro.Compiler.addGlobalMetadata("--no-inline");
			// haxe.macro.Compiler.addNativeArg("-v");
		}
	}
	#end
}
