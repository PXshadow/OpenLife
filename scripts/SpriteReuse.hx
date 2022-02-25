package scripts;

import openlife.resources.ObjectBake;
import openlife.engine.Utility;
import openlife.data.object.ObjectData;
import openlife.engine.Engine;
import haxe.ds.Vector;

class SpriteReuse {
	public static function main() {
		Sys.println("start");
		Engine.dir = Utility.dir();
		var engine = new Engine(null);
		var vector = ObjectBake.objectList();
		var data = new Vector<Array<Int>>(vector.length);
		var index:Int = 0;
		var object:ObjectData;
		for (id in vector) {
			if (id % 500 == 0) trace("id " + id);
			object = new ObjectData(id);
			data[index] = [];
			for (sprite in object.spriteArray) {
				data[index].push(sprite.spriteID);
			}
			index++;
		}
		// compare data
		trace('index is now $index');
		var percent:Float = 0;
		var next:Int = 0;
		var reused:Int = 0;
		var used:Array<Int> = [];
		for (i in 0...data.length) {
			percent = i / data.length * 100;
			if (percent > next) {
				trace('left $percent');
				next++;
			}
			for (id in data[i]) {
				for (j in 0...data.length) {
					if (j == i) continue;
					// trace("id " + id + " j " + (j/data.length));
					for (id2 in data[j]) {
						if (id == id2) {
							reused++;
							if (used.indexOf(id) == -1) used.push(id);
						}
					}
				}
			}
		}
		// 1731806
		trace('sprites reused $reused amount of times from another object');
		// 1518
		trace('amount of sprites reused ${used.length}');
		// percent of sprites being reused 68%
		// how many sprites 2239
	}
}
