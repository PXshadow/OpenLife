package openlife.resources;

import haxe.io.Path;
import openlife.engine.Engine;
import haxe.io.Bytes;

@:expose
class Resource {
	public static function objectData(id:Int):String {
		return getContent("objects", '$id');
	}

	public static function spriteData(id:Int):String {
		return getContent("sprites", '$id');
	}

	public static function spriteImage(id:Int):Bytes {
		return getImage("sprites", '$id');
	}

	public static function sound(id:Int):Bytes {
		return bytes('sounds/$id.aiff');
	}

	public static function music(id:Int):Bytes {
		var ids = Std.string(id);
		if (ids.length == 1)
			ids = '0$ids';
		return bytes('music/music_$ids.ogg');
	}

	public static function graphicImage(name:String):Bytes {
		return getImage("graphics", name);
	}

	public static function languageData(name:String):String {
		return getContent("languages", name);
	}

	public static function animation(id:Int, i:Int):String {
		return getContent("animations", '${id}_${i}');
	}

	public static function ground(id:Int, i:Int, j:Int, a:String):Bytes {
		return getImage("groundTileCache", 'biome_${id}_x${i}_y$j$a');
	}

	public static function groundOverlay(id:Int):Bytes {
		return getImage("graphics", 'ground_t$id');
	}

	public static function dataVersionNumber():Int {
		return Std.parseInt(content("dataVersionNumber.txt"));
	}

	public static function getImage(path:String, name:String):Bytes {
		return bytes('$path/$name.tga');
	}

	public static function getContent(path:String, name:String):String {
		path = Path.addTrailingSlash(path);
		return content('$path/$name.txt');
	}

	// reciever that overrides old content and bytes system
	public static var recContent:String->String = null;
	public static var recBytes:String->Bytes = null;

	public static function content(path:String):String {
		if (recContent != null)
			return recContent(path);
		return sys.io.File.getContent('${Engine.dir}/$path');
	}

	public static function bytes(path:String):Bytes {
		if (recBytes != null)
			return recBytes(path);
		return sys.io.File.getBytes('${Engine.dir}/$path');
	}
}
