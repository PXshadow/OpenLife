import openlife.data.transition.TransitionData;
import openlife.data.object.ObjectData;
import haxe.ds.Vector;
import openlife.resources.ObjectBake;
import openlife.engine.Utility;
import openlife.engine.Engine;
import openlife.data.transition.TransitionImporter;
class Transition
{
    public static function main()
    {
        Engine.dir = Utility.dir();
        new Transition();
    }
    var catMap:Map<Int,Array<Int>>;
    var transMap:Map<Int,Array<NodeData>>;
    var blacklisted:Array<Int> = [];
    private function new()
    {
        var importer = new TransitionImporter();
        trace("object list");
        var vector = ObjectBake.objectList();
        for (id in vector)
        {
            var obj = new ObjectData(id);
            if (obj.isNatural() || obj.numUses > 1)
            {
                blacklisted.push(id);
            }
        }
        trace("categories");
        importer.importCategories();
        catMap = new Map<Int,Array<Int>>();
        for (cat in importer.categories)
        {
            if (cat.pattern) continue;
            catMap.set(cat.parentID,cat.ids);
        }
        importer.categories = [];
        trace("transitions");
        importer.importTransitions();
        transMap = new Map<Int,Array<NodeData>>();
        for (trans in importer.transitions)
        {
            if (trans.actorID != trans.newActorID) add(trans.newActorID,trans);
            if (trans.targetID != trans.newTargetID) add(trans.newTargetID,trans);
        }
        while (true)
        {
            trace("id:");
            var id = Std.parseInt(Sys.stdin().readLine());
            var obj = transMap.get(id);
            if (obj != null) 
            {
                sort(obj);
                for (o in obj)
                {
                    Sys.println(o);
                    //trace("depth: " + depth(o));
                }
            }else{
                trace("trans null");
            }
        }
    }
    private inline function get(id:Int):Array<Int>
    {
        return catMap.exists(id) ? catMap.get(id) : [id];
    }
    private inline function add(id:Int,trans:TransitionData)
    {
        //expirmental reduction of transitions for production (does not work for some cases)
        //if (id < trans.actorID) return;
        //if (id < trans.targetID) return;
        if (transMap.exists(trans.targetID) && (transMap.get(trans.targetID)[0].target[0] == id || transMap.get(trans.targetID)[0].actor[0] == id)) return;
        if (transMap.exists(trans.actorID) && (transMap.get(trans.actorID)[0].actor[0] == id || transMap.get(trans.actorID)[0].target[0] == id)) return;
        if (blacklisted.indexOf(id) != -1) return;
        //limit only objects
        if (id <= 0) return;
        //go through all add add potential list of objects in formula
        for (id in get(id))
        {
            if (id == trans.targetID || id == trans.actorID) continue;
            if (!transMap.exists(id)) transMap.set(id,[]);
            var array = transMap.get(id);
            var a = get(trans.actorID);
            var b = get(trans.targetID);
            array.unshift({actor: a,target: b,tool: trans.tool,decay: trans.decay}); 
        }
    }
    private inline function depth(node:NodeData):Int
    {
        if (node.actor.length > 1) trace("actors: " + node.actor);
        var a = transMap.get(node.actor[0]);
        var b = transMap.get(node.target[0]);
        return (a == null ? 1 : depth(a[0])) + (b == null ? 1 : depth(b[0]));
    }
    private inline function sort(nodes:Array<NodeData>)
    {
        nodes.sort(function(a:NodeData,b:NodeData)
        {
            if (a.tool && !b.tool) return 1;
            if (!a.tool && b.tool) return -1;
            return 0;
        });
    }
}
typedef NodeData = {target:Array<Int>,actor:Array<Int>,tool:Bool,decay:Int} 