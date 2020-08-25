import openlife.data.object.ObjectData;
import sys.io.File;
import openlife.resources.ObjectBake;
import sys.FileSystem;
import haxe.Serializer;
import haxe.Unserializer;
import haxe.ds.Vector;
class Bake
{
    public static function dummies():Vector<Int>
    {
        if (FileSystem.exists("dummymap"))
        {
            ObjectBake.dummies = cast Unserializer.run(File.getContent("dummymap"));
            return ObjectBake.objectList();
        }
        var vector = ObjectBake.objectList();
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
                last = p;
            }
        }
        File.saveContent("dummymap",Serializer.run(ObjectBake.dummies));
        return vector;
    }
}