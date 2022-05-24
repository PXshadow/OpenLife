package scripts;

import haxe.io.Path;
import sys.FileSystem;
import sys.io.File;

class FileTools {
	public static function copyDir(path:String, newpath:String) {
		path = Path.addTrailingSlash(path);
		newpath = Path.addTrailingSlash(newpath);
		if (FileSystem.exists(path) && FileSystem.isDirectory(path)) {
			if (!FileSystem.exists(newpath)) FileSystem.createDirectory(newpath);
			var dir = FileSystem.readDirectory(path);
			for (name in dir) {
				if (name.substring(0, 1) == ".") continue; // skip git

				if (FileSystem.isDirectory(path + name)) {
					FileSystem.createDirectory(newpath + name);
				} else {
					File.copy(path + name, newpath + name);
				}
			}
		}
	}

	public static function deleteDir(path:String) {
		path = Path.addTrailingSlash(path);
		if (FileSystem.exists(path) && FileSystem.isDirectory(path)) {
			var dir = FileSystem.readDirectory(path);
			var i:Int = 0;
			for (name in dir) {
				// if (name.substring(0,1) == ".") continue; //skip git

				if (FileSystem.isDirectory(path + name)) {
					deleteDir(path + name);
					sys.FileSystem.deleteDirectory(path + name);
				} else {
					FileSystem.deleteFile(path + name);
				}
			}
		}
	}
}
