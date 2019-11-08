package data;
import haxe.ds.ObjectMap;
import game.Player;
#if full
import game.Ground;
import data.TransitionData;
#end
import sys.FileSystem;
import sys.io.File;
import haxe.io.Path;
#if openfl
import openfl.display.TileContainer;
import openfl.display.Tile;
import openfl.geom.Rectangle;
#end
//data stored for the game to function (map data -> game data)
class GameData
{
    //block walking
    public var blocking:Map<String,Bool> = new Map<String,Bool>();
    public var playerMap:Map<Int,Player> = new Map<Int,Player>();
    #if full
    public var transitionData:TransitionData;
    public var map:MapData;
    #end
    public var spriteMap:Map<Int,SpriteData> = new Map<Int,SpriteData>();
    public var objectMap:Map<Int,ObjectData> = new Map<Int,ObjectData>();
    //object alternative ids to refrence same object
    public var objectAlt:Map<Int,Int> = new Map<Int,Int>();
    public var nextObjectNumber:Int = 0;
    #if openfl
    public var tileData:TileData;
    #end
    public function new()
    {
        #if openfl
        tileData = new TileData();
        #end
        //transitionData = new TransitionData();
        #if full
        map = new MapData();
        objectData();
        #end
        #if openfl
        openfl.Lib.current.stage.frameRate = 60;
        #end
    }
    private function objectData()
    {
        //nextobject
        nextObjectNumber = Std.parseInt(File.getContent(Static.dir + "objects/nextObjectNumber.txt"));
        //go through objects
        var list:Array<Int> = [];
        for (path in FileSystem.readDirectory(Static.dir + "objects"))
        {
            list.push(Std.parseInt(Path.withoutExtension(path)));
        }
        list.sort(function(a:Int,b:Int)
        {
            if (a > b) return 1;
            return -1;
        });
        var data:ObjectData = null;
        var nextObjectNumberInt:Int = nextObjectNumber;
        for (i in list)
        {
            data = new ObjectData(i);
            //alternative set
            if (data.numUses > 1) for (j in 0...data.numUses) 
            {
                objectAlt.set(++nextObjectNumberInt,i);
            }
            objectMap.set(data.id,data);
        }
    }
}