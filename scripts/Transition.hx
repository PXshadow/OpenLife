package;

import openlife.data.transition.Recipe;
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
        importer = new TransitionImporter();
        //189 rabbit bone
        //502 shovel
        //59 rope
        //diesel engine 2365
        while (true)
        {
            var result = read("Object");
            Sys.println('Looking for transition through ${importer.transitions.length} total...');
            var rep = new Recipe(result);
            rep.generate(importer.transitions,importer.categories);
            Sys.println("depth: " + rep.depth());
            //return;
        }
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