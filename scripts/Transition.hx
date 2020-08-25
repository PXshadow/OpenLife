package;

import haxe.ds.Either;
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
    var vector:Vector<Int>;
    var importer:TransitionImporter;
    public function new()
    {
        vector = ObjectBake.objectList();
        trace("start");
        importer = new TransitionImporter();
        trace("imported");
        generated = [-1,0];
        for (id in vector)
        {
            if (new ObjectData(id).isNatural()) generated.push(id);
        }
        create();
        trace("generated " + generated.length);
        while (true)
        {
            var id = read("Object");
            var data = Node.map.get(id);
            trace("index " + generated.indexOf(id));
            if (data == null)
            {
                Sys.println("data: null");
                continue;
            }
            if (data.node == null) 
            {
                Sys.println("int: " + data.id);
                continue;
            }
            Sys.println("node: " + data.node + " nodes: " + data.node.nodes);
        }
    }
    var generated:Array<Int> = [];
    private function create(depth:Int=0)
    {
        var temp:Array<Int> = [];
        for (trans in importer.transitions)
        {
            //if (trans.actorID == trans.newActorID) continue;
            //if (trans.targetID == trans.actorID) continue;
            if (generated.indexOf(trans.actorID) == -1 || generated.indexOf(trans.targetID) == -1) continue;
            new Node(trans.actorID,trans.targetID,trans.newActorID,trans.newTargetID,depth);
            if (temp.indexOf(trans.newActorID) == -1 && generated.indexOf(trans.newActorID) == -1) temp.push(trans.newActorID);
            if (temp.indexOf(trans.newTargetID) == -1 && generated.indexOf(trans.newTargetID) == -1) temp.push(trans.newTargetID);
        }
        trace("tmp " + temp);
        if (temp.length == 0) return;
        generated = generated.concat(temp);
        create(++depth);
    }
    public function read(type:String):Int
    {
        Sys.println('$type id or desc:');
        var value = Sys.stdin().readLine();
        var int = Std.parseInt(value);
        if (int != null) return int;
        return get(value);
    }
    public function get(desc:String):Int
    {
        for (id in vector) 
        {
            var objDesc = new ObjectData(id,true).description;
            if (objDesc.indexOf(desc) > -1)
            {
                Sys.println('found object $objDesc');
                return id;
            }
        }
        return 0;
    }
}
class Node
{
    public static var map:Map<Int,NodeData> = new Map<Int,NodeData>();
    public var target:NodeData;
    public var actor:NodeData;
    var newTarget:NodeData;
    var newActor:NodeData;
    var depth:Int = 0;
    public var nodes:Array<Node> = []; //other ways to make the object
    public function new(actor:Int,target:Int,newActor:Int,newTarget:Int,depth:Int)
    {
        //either natural object, hand or decay, or Node leading to others
        this.depth = depth;
        this.actor = get(actor);
        this.target = get(target);
        this.newActor = get(newActor);
        this.newTarget = get(newTarget);
        if (newActor > 0)
        {
            fill(newTarget);
            map.set(newActor,{id:newActor,node: this});
        }
        if (newTarget > 0)
        {
            fill(newTarget);
            map.set(newTarget,{id: newTarget,node: this});
        }
    }
    private function fill(id:Int)
    {
        var data = map.get(id);
        if (data == null) return;
        if (data.id != id) return;
        nodes.push(data.node);
    }
    private function get(id:Int):NodeData
    {
        return map.exists(id) ? map.get(id) : {id: id,node: null};
    }
    public function toString():String
    {
        return actor.id + " + " + target.id + " = " + newActor.id + " + " + newTarget.id;
    }
}
typedef NodeData = {id:Int,node:Node}