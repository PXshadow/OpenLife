package openlife.resources;

import openlife.engine.Engine;
import haxe.ds.Vector;
import openlife.data.object.ObjectData;

/**
 * Bakes the numUses objects into files, rather than having to run through all the objects in the start of the session
 */
 
class ObjectBake
{
    public function new()
    {
        run();
    }
    private function run()
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
        var vector = Engine.data.objectData();
        if (bakeNum == Engine.data.nextObjectNumber)
        {
            trace("bake complete and set");
            return;
        }
        objectData(vector);
        sys.io.File.saveContent(Engine.dir + "bake.res",Std.string(Engine.data.nextObjectNumber));
        #end
    }
    private function objectData(vector:Vector<Int>)
    {
        var int = Engine.data.nextObjectNumber;
        var data:ObjectData;
        var dummyObject:ObjectData;
        var i:Int = 0;
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
                    Engine.data.objectMap.set(dummyObject.id,dummyObject);
                }
            }
            Engine.data.objectMap.set(data.id,data);
            i++;
            if(i % 100 == 0)  trace('index: $i');
            
        }
    }
    private static function gen()
    {

    }
}