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
                //trace(obj);
                for (o in obj)
                {
                    trace(o);
                    trace("depth: " + depth(o));
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
        //expirmental reduction of transitions for production
        if (id < trans.actorID) return;
        if (id < trans.targetID) return;
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
            array.unshift({actor: a,target: b}); 
        }
    }
    private inline function depth(node:NodeData,value:Int=0):Int
    {
        var a = transMap.get(node.actor[0]);
        var b = transMap.get(node.target[0]);
        Sys.sleep(1/10);
        return (a == null ? 1 : depth(a[0])) + (b == null ? 1 : depth(b[0])) + value;
    }
}
typedef NodeData = {target:Array<Int>,actor:Array<Int>} 