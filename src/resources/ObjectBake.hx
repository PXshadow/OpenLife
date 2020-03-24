package resources;

import game.Game;
import sys.FileSystem;
import sys.io.File;
import haxe.ds.Vector;
import data.object.ObjectData;

/**
 * Bakes the numUses objects into files, rather than having to run through all the objects in the start of the session
 */
 #if nativeGen @:nativeGen #end
class ObjectBake
{
    public function new()
    {
        run();
    }
    private function run()
    {
        if (!FileSystem.exists(Game.dir + "objects/"))
        {
            trace("could not find objects to bake");
            return;
        }
        var bakeNum = 0;
        if (FileSystem.exists(Game.dir + "bake.res"))
        {
            bakeNum = Std.parseInt(File.getContent(Game.dir + "bake.res"));
        }
        var vector = Game.data.objectData();
        if (bakeNum == Game.data.nextObjectNumber)
        {
            trace("bake complete and set");
            return;
        }
        objectData(vector);
        File.saveContent(Game.dir + "bake.res",Std.string(Game.data.nextObjectNumber));
    }
    private function objectData(vector:Vector<Int>)
    {
        var int = Game.data.nextObjectNumber;
        var data:ObjectData;
        var dummyObject:ObjectData;
        var i:Int = 0;
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
                    File.saveContent(Game.dir + 'objects/$int.txt',dummyObject.toFileString());
                    Game.data.objectMap.set(dummyObject.id,dummyObject);
                }
            }
            Game.data.objectMap.set(data.id,data);
            if (i++ % 200 == 0) trace("i " + i);
        }
    }
    private static function gen()
    {

    }
}