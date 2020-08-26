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
        generated = [-2,-1,0];
        for (id in vector)
        {
            if (new ObjectData(id).isNatural()) generated.push(id);
        }
        transitions = importer.transitions.copy();
        create();
        trace("generated " + generated.length);
        while (true)
        {
            var id = read("Object");
            var data = Node.map.get(id);
            Sys.println("index " + generated.indexOf(id));
            for (trans in importer.transitions)
            {
                if (trans.newActorID != id && trans.newTargetID != id) continue;
                Sys.println(trans);
            }
            if (data == null)
            {
                Sys.println("data: null");
                continue;
            }
            Sys.println("node " + data[0]);
        }
    }
    var generated:Array<Int> = [];
    var prev:Int = 0;
    var transitions:Array<TransitionData>;
    private function create()
    {
        for (trans in transitions)
        {
            if (trans.actorID == 0 && trans.targetID == 500)
            {
                trace("SHOVEL HEAD " + trans + " index 0 " + generated.indexOf(trans.actorID) + " " + generated.indexOf(trans.targetID));
            }
            if (trans.targetID == 500) 
            {
                trace("steel shovel head found " + generated.indexOf(trans.targetID));
            }
            if (generated.indexOf(trans.actorID) == -1 && generated.indexOf(trans.targetID) == -1) continue;
            if (trans.targetID == 500)
            {
                trace("created shovel head node");
            }
            new Node(trans);
            transitions.remove(trans);
            if (generated.indexOf(trans.newTargetID) == -1) generated.push(trans.newTargetID);
            if (generated.indexOf(trans.newActorID) == -1) generated.push(trans.newActorID);
        }
        trace("left " + transitions.length);
        if (transitions.length == prev) 
        {
            trace("LEFT:");
            for (trans in transitions)
            {
                trace(trans);
            }
            return;
        }
        prev = transitions.length;
        create();
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
    public static var map:Map<Int,Array<Node>> = new Map<Int,Array<Node>>();
    public var target:NodeData;
    public var actor:NodeData;
    public var newTarget:NodeData;
    public var newActor:NodeData;
    public var nodes:Array<Node> = []; //other ways to make the object
    public function new(trans:TransitionData)
    {
        //either natural object, 0 = empty ground, hand, -1 = decay, consuming food
        this.actor = get(trans.actorID);
        this.target = get(trans.targetID);
        this.newActor = get(trans.newActorID);
        this.newTarget = get(trans.newTargetID);
        if (trans.newActorID > 0 && newActor != actor) fill(trans.newTargetID);
        if (trans.newTargetID > 0 && newTarget != target) fill(trans.newTargetID);
    }
    private function fill(id:Int)
    {
        var array = map.get(id);
        if (array == null) 
        {
            map.set(id,[this]);
            return;
        }
        array.unshift(this);
    }
    private function get(id:Int):NodeData
    {
        return map.exists(id) ? {id: id, nodes: map.get(id)} : {id: id,nodes: []};
    }
    public function toString():String
    {
        return actor.id + " + " + target.id + " = " + newActor.id + " + " + newTarget.id + " depth: " + depth();
    }
    public function depth():Int
    {
        return 0;
    }
}
typedef NodeData = {id:Int,nodes:Array<Node>}