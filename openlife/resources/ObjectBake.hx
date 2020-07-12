package openlife.resources;

import openlife.engine.Engine;
import haxe.ds.Vector;
import openlife.data.object.ObjectData;
import haxe.io.Path;

/**
 * Bakes the numUses objects into files, rather than having to run through all the objects in the start of the session
 */
 
class ObjectBake
{
    public static var nextObjectNumber:Int = 0;
    public static var baked:Bool = false;
    public function new()
    {

    }
    public static function finish()
    {
        #if (nodejs || sys)
        sys.io.File.saveContent(Engine.dir + "bake.res",Std.string(nextObjectNumber));
        #end
    }
    public static function objectList():Vector<Int>
    {
        #if sys
        if (!sys.FileSystem.exists(Engine.dir + "objects/nextObjectNumber.txt")) 
        {
            trace("object data failed");
            nextObjectNumber = 0;
            return null;
        }
        nextObjectNumber = Std.parseInt(sys.io.File.getContent(Engine.dir + "objects/nextObjectNumber.txt"));
        var list:Array<Int> = [];
        var num:Int = 0;
        for (path in sys.FileSystem.readDirectory(Engine.dir + "objects"))
        {
            num = Std.parseInt(Path.withoutExtension(path));
            if (num > 0 && num < nextObjectNumber) 
            {
                list.push(num);
            }
        }
        list.sort(function(a:Int,b:Int)
        {
            if (a > b) return 1;
            return -1;
        });
        if (sys.FileSystem.exists(Engine.dir + "bake.res"))
        {
            baked = nextObjectNumber == Std.parseInt(sys.io.File.getContent(Engine.dir + "bake.res"));
        }
        return Vector.fromArrayCopy(list);
        #else
        return Vector.fromArrayCopy([]);
        #end
    }
    public function objectData(vector:Vector<Int>):Array<ObjectData>
    {
        var array:Array<ObjectData> = [];
        var data:ObjectData;
        var dummyObject:ObjectData;
        var id:Int = 0;
        var i:Int = 0;
        for (id in vector)
        {
            data = new ObjectData(id);
            if (data.numUses > 1)
            {
                for (j in 1...data.numUses - 1)
                {
                    dummyObject = data.clone();
                    dummyObject.id = 0;
                    dummyObject.numUses = 0;
                    dummyObject.dummy = true;
                    dummyObject.dummyParent = data.id;
                    array.push(dummyObject);
                }
            }
            if (i++ % 100 == 0 || i > 4071) trace('$id');
        }
        return array;
    }
}