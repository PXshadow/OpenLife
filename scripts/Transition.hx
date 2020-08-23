package;

import openlife.data.object.ObjectData;
import haxe.ds.Vector;
import openlife.resources.ObjectBake;
import openlife.engine.Utility;
import openlife.engine.Engine;
import haxe.ds.Either;

class Transition
{
    public static function main()
    {
        Engine.dir = Utility.dir();
        new Transition();
    }
    var vector:Vector<Int>;
    public function new()
    {
        vector = ObjectBake.objectList();
        var actor = read("Actor");
        var target = read("Target");
        Sys.println("Looking for transition...");
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
        for (id in vector) if (new ObjectData(id,true).description.indexOf(desc) > -1) return id;
        return 0;
    }
}