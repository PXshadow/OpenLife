import sys.io.File;
import openlife.resources.ObjectBake;
import sys.FileSystem;
import haxe.Serializer;
import haxe.Unserializer;
class Bake
{
    public static function run()
    {
        if (FileSystem.exists("dummymap"))
        {
            ObjectBake.dummies = cast Unserializer.run(File.getContent("dummymap"));
            return;
        }
        var list = ObjectBake.objectData(ObjectBake.objectList());
        trace("dummies " + list.length);
        var index = ObjectBake.nextObjectNumber;
        var i:Int = 0;
        var p:Float = 0;
        var last:Float = -0.1;
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
    }
}