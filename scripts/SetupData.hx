package scripts; // used globally

import haxe.io.Path;
import sys.io.Process;
import sys.FileSystem;
import sys.io.File;

class SetupData {
	public static function main() {
		new SetupData();
	}

	var users:Array<String> = ["jasonrohrer", "twohoursonelife"];
	var index:Null<Int>;

	public function new() {
		if (index == null || index < 0 || index > users.length - 1) index = 0;
		var cwd = Sys.getCwd();
		// linux is folder name case senetive
		if (!FileSystem.exists("OneLifeData7")) {
			Sys.println('Rep input an index of (0) or (1) for $users :');
			index = Std.parseInt(Sys.stdin().readLine());
			Sys.println('Downloading ${users[index]}');
			trace("clone-");
			Sys.command('git clone https://github.com/${users[index]}/OneLifeData7.git');
		}
		Sys.setCwd("OneLifeData7");
		trace("pull-");
		Sys.command('git fetch --force');
		Sys.command("git fetch --tags");
		var proc = new Process("git for-each-ref --sort=-creatordate --format '%(refname:short)' --count=1");

		var tag = proc.stdout.readLine();
		tag = StringTools.trim(tag);
		tag = StringTools.replace(tag, "'", "");
		trace("tag = |" + tag + "|");
		Sys.command('git checkout -q $tag');
		trace("checkout!");
		Sys.setCwd(cwd);
		if (!FileSystem.exists("OneLifeGameSourceData")) {
			Sys.command('git clone https://github.com/PXshadow/OneLifeGameSourceData');
		}
		Sys.setCwd("OneLifeGameSourceData");
		var proc = new Process("git pull --force");
		var line = proc.stdout.readLine();
		trace('line |$line|');
		Sys.setCwd(cwd);
		if (line != "Already up to date." || !FileSystem.exists("OneLifeData7/graphics")) {
			trace("copy dir!");
			// copydir
			for (path in ["graphics", "settings", "languages", "groundTileCache"]) {
				FileTools.copyDir('OneLifeGameSourceData/$path', 'OneLifeData7/$path');
			}
		}
		FileTools.deleteDir("OneLifeGameSourceData");
	}
}
