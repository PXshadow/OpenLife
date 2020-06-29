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
    var nextObjectNumber:Int = 0;
    public function new()
    {

    }
    public function run(list:Vector<Int>)
    {
        #if sys
        if (!sys.FileSystem.exists(Engine.dir + "objects/"))
        {
            trace("could not find objects to bake");
            return;
        }
        var bakeNum = 0;
        if (sys.FileSystem.exists(Engine.dir + "bake.res"))
        {
            bakeNum = Std.parseInt(sys.io.File.getContent(Engine.dir + "bake.res"));
        }
        
        if (bakeNum == nextObjectNumber)
        {
            trace("bake complete and set");
            return;
        }
        objectData(list);
        sys.io.File.saveContent(Engine.dir + "bake.res",Std.string(nextObjectNumber));
        #end
    }
    /**
     * Generate object data
     */
    public function objectList():Vector<Int>
    {
        #if sys
        if (!sys.FileSystem.exists(Engine.dir + "objects/nextObjectNumber.txt")) 
        {
            trace("object data failed");
            nextObjectNumber = 0;
            return null;
        }
        //nextobject
        nextObjectNumber = Std.parseInt(sys.io.File.getContent(Engine.dir + "objects/nextObjectNumber.txt"));
        //go through objects
        var list:Array<Int> = [];
        var num:Int = 0;
        for (path in sys.FileSystem.readDirectory(Engine.dir + "objects"))
        {
            num = Std.parseInt(Path.withoutExtension(path));
            if (num > 0) 
            {
                list.push(num);
            }
        }
        list.sort(function(a:Int,b:Int)
        {
            if (a > b) return 1;
            return -1;
        });
        return Vector.fromArrayCopy(list);
        #else
        return Vector.fromArrayCopy([]);
        #end
    }
    private function objectData(vector:Vector<Int>)
    {
        var int = nextObjectNumber;
        var data:ObjectData;
        var dummyObject:ObjectData;
        #if sys
        var file:sys.io.FileOutput = null;
        #end
        for (id in vector)
        {
            data = new ObjectData(id);
            if (data.numUses > 1)
            {
                for (j in 1...data.numUses - 1)
                {
                    dummyObject = data.clone();
                    dummyObject.id = ++int;
                    dummyObject.numUses = 0;
                    dummyObject.dummy = true;
                    dummyObject.dummyParent = data.id;
                    #if sys
                    file = sys.io.File.write(Engine.dir + 'objects/$int.txt');
                    file.writeString(dummyObject.toFileString());
                    file.flush();
                    file.close();
                    #end
                    //Engine.data.objectMap.set(dummyObject.id,dummyObject);
                }
            }
            //Engine.data.objectMap.set(data.id,data);
        }
    }
    private static function gen()
    {

    }
}