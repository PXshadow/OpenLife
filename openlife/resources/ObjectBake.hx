package openlife.resources;

import haxe.ds.Map;
import openlife.engine.Engine;
import haxe.ds.Vector;
import openlife.data.object.ObjectData;
import haxe.io.Path;

/**
 * Bakes the numUses objects into files, rather than having to run through all the objects in the start of the session
 */
@:expose
class ObjectBake {
	public static var nextObjectNumber:Int = 0;
	public static var baked:Bool = false;
	public static var dummies = new Map<Int, Array<Int>>();
	public static var dummiesMap = new Map<Int, Int>();

	public static function finish() {
		#if (nodejs || sys)
		sys.io.File.saveContent(Engine.dir + "bake.res", Std.string(nextObjectNumber));
		#end
	}

	public static function objectList():Vector<Int> {
		if (!sys.FileSystem.exists(Engine.dir + "objects/nextObjectNumber.txt")) {
			trace("object data failed to load");
			trace("In order to fix run: haxe setup_data_client.hxml");
			nextObjectNumber = 0;
			return null;
		}
		nextObjectNumber = Std.parseInt(sys.io.File.getContent(Engine.dir + "objects/nextObjectNumber.txt"));
		var list:Array<Int> = [];
		var num:Int = 0;
		for (path in sys.FileSystem.readDirectory(Engine.dir + "objects")) {
			num = Std.parseInt(Path.withoutExtension(path));
			if (num > 0 && num < nextObjectNumber) {
				list.push(num);
			}
		}
		list.sort(function(a:Int, b:Int) {
			if (a > b) return 1;
			return -1;
		});
		if (sys.FileSystem.exists(Engine.dir + "bake.res")) {
			baked = nextObjectNumber == Std.parseInt(sys.io.File.getContent(Engine.dir + "bake.res"));
		}
		return Vector.fromArrayCopy(list);
	}

	public static function objectData(vector:Vector<Int>):Array<ObjectData> {
		var array:Array<ObjectData> = [];
		var data:ObjectData;
		var i:Int = 0;
		for (id in vector) {
			data = new ObjectData(id);
			if (data.numUses > 1) {
				for (j in 1...data.numUses - 1) {
					data.id = 0;
					data.numUses = 0;
					data.dummy = true;
					data.dummyParent = data;
					array.push(data);
				}
			}
		}
		return array;
	}

	public static function dummy(obj:ObjectData) {
		var array = dummies.get(obj.dummyParent.id);
		if (array == null) array = [];
		array.push(obj.id);
		dummies.set(obj.dummyParent.id, array);
		dummiesMap.set(obj.id, obj.dummyParent.id);
	}
}
