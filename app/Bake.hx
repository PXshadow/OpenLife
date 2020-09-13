import openlife.engine.Engine;
import openlife.data.object.ObjectData;
import sys.io.File;
import openlife.resources.ObjectBake;
import sys.FileSystem;
import haxe.Serializer;
import haxe.Unserializer;
import haxe.ds.Vector;
class Bake
{
    public static function run():Vector<Int>
    {
        var vector = ObjectBake.objectList();
        if (FileSystem.exists(Engine.dir + "dummymap") && ObjectBake.baked)
        {
            ObjectBake.dummies = cast Unserializer.run(File.getContent(Engine.dir + "dummymap"));
            return vector;
        }
        var list = ObjectBake.objectData(vector);
        var index = ObjectBake.nextObjectNumber;
        var i:Int = 0;
        var p:Float = 0;
        var last:Float = -0.05;
        for (obj in list)
        {
            obj.id = ++index;
            ObjectBake.dummy(obj);
            p = ++i/list.length;
            if (last + 0.1 < p)
            {
                trace("baking " + Std.int(p * 100) + "%");
                Sys.sleep(0.01);
                last = p;
            }
        }
        File.saveContent(Engine.dir + "dummymap",Serializer.run(ObjectBake.dummies));
        ObjectBake.finish();
        return vector;
    }
}