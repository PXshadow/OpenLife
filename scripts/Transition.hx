import openlife.data.transition.TransitionData;
import openlife.data.object.ObjectData;
import haxe.ds.Vector;
import openlife.resources.ObjectBake;
import openlife.engine.Utility;
import openlife.engine.Engine;
import openlife.data.transition.TransitionImporter;

class Transition {
	public static function main() {
		Engine.dir = Utility.dir();
		new Transition();
	}

	var catMap:Map<Int, Array<Int>>;
	var transMap:Map<Int, Array<NodeData>>;
	var blacklisted:Array<Int> = [];

	private function new() {
		var importer = new TransitionImporter();
		trace("object list");
		var vector = ObjectBake.objectList();
		blacklisted = [50, 51, 52]; // add milkweed as it's causing errors
		for (id in vector) {
			var obj = new ObjectData(id);
			if (obj.isNatural() || obj.numUses > 1) {
				blacklisted.push(id);
			}
		}
		trace("categories");
		importer.importCategories();
		catMap = new Map<Int, Array<Int>>();
		for (cat in importer.categories) {
			if (cat.pattern)
				continue;
			cat.ids.sort(function(a:Int, b:Int) {
				return a > b ? 1 : -1;
			});
			catMap.set(cat.parentID, cat.ids);
		}
		importer.categories = [];
		trace("transitions");
		importer.importTransitions();
		transMap = new Map<Int, Array<NodeData>>();
		for (trans in importer.transitions) {
			if (trans.actorID != trans.newActorID)
				add(trans.newActorID, trans);
			if (trans.targetID != trans.newTargetID)
				add(trans.newTargetID, trans);
		}
		while (true) {
			trace("id:");
			var id = Std.parseInt(Sys.stdin().readLine());
			var obj = transMap.get(id);
			if (obj != null) {
				sort(obj);
				trace(obj[0] + " possibilities " + obj.length);
				var stepsArray:Array<NodeData> = [];
				steps(obj[0], stepsArray);
				trace("steps: " + stepsArray.length);
				/*for (o in obj)
					{
						Sys.println(o);
						//trace("depth: " + depth(o));
						var stepsArray:Array<NodeData> = [];
						steps(o,stepsArray);
						trace("steps " + stepsArray.length);
				}*/
			} else {
				trace("trans null");
			}
		}
	}

	private inline function get(id:Int):Array<Int> {
		return catMap.exists(id) ? catMap.get(id) : [id];
	}

	private inline function add(id:Int, trans:TransitionData) {
		if (transMap.exists(trans.targetID)
			&& (transMap.get(trans.targetID)[0].target[0] == id || transMap.get(trans.targetID)[0].actor[0] == id))
			return;
		if (transMap.exists(trans.actorID)
			&& (transMap.get(trans.actorID)[0].actor[0] == id || transMap.get(trans.actorID)[0].target[0] == id))
			return;
		if (blacklisted.indexOf(id) != -1)
			return;
		// limit only objects
		if (id <= 0)
			return;
		// go through all add add potential list of objects in formula
		for (id in get(id)) {
			if (id == trans.targetID || id == trans.actorID)
				continue;
			if (!transMap.exists(id))
				transMap.set(id, []);
			var array = transMap.get(id);
			var a = get(trans.actorID);
			var b = get(trans.targetID);
			array.unshift({
				actor: a,
				target: b,
				tool: trans.tool,
				decay: trans.autoDecaySeconds
			});
		}
	}

	private inline function steps(node:NodeData, array:Array<NodeData>, count:Int = 0) {
		// if (++count > 30) return;
		var actorId = node.actor[0];
		var targetId = node.target[0];
		var actor = transMap.get(actorId);
		var target = transMap.get(targetId);
		Sys.sleep(0.5);
		trace(node);
		if (actor != null) {
			steps(actor[0], array, count);
		}
		if (target != null) {
			steps(target[0], array, count);
		}
		array.push(node);
	}

	private inline function sort(nodes:Array<NodeData>) {
		nodes.sort(function(a:NodeData, b:NodeData) {
			if (a.tool && !b.tool)
				return 1;
			if (!a.tool && b.tool)
				return -1;
			return (a.actor[0] + a.target[0]) >= (b.actor[0] + b.target[0]) ? 1 : -1;
		});
	}
}

typedef NodeData = {target:Array<Int>, actor:Array<Int>, tool:Bool, decay:Int}
