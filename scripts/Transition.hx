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
    private function new()
    {
        var importer = new TransitionImporter();
        importer.importCategories();
        catMap = new Map<Int,Array<Int>>();
        for (cat in importer.categories)
        {
            if (cat.pattern) continue;
            catMap.set(cat.parentID,cat.ids);
        }
        importer.categories = [];
        importer.importTransitions();
        transMap = new Map<Int,Array<NodeData>>();
        for (trans in importer.transitions)
        {
            add(trans.newActorID,trans);
            add(trans.newTargetID,trans);
        }
        while (true)
        {
            trace("id:");
            var id = Std.parseInt(Sys.stdin().readLine());
            trace(transMap.get(id));
        }
    }
    private inline function get(id:Int):Array<Int>
    {
        return catMap.exists(id) ? catMap.get(id) : [id];
    }
    private inline function add(id:Int,trans:TransitionData)
    {
        for (id in get(id))
        {
            if (id == trans.targetID || id == trans.actorID) continue;
            if (id <= 0) continue;
            if (!transMap.exists(id)) transMap.set(id,[]);
            transMap.get(id).unshift({actor: get(trans.actorID),target: get(trans.targetID)});
        }
    }
    private inline function depth(node:NodeData)
    {
        
    }
}
typedef NodeData = {target:Array<Int>,actor:Array<Int>} 