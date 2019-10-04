package data;
import game.Ground;
import game.Player;
#if openfl
import openfl.display.TileContainer;
import openfl.display.Tile;
import openfl.geom.Rectangle;
#end
import data.TransitionData;
//data stored for the game to function (map data -> game data)
class GameData
{
    //block walking
    public var blocking:Map<String,Bool> = new Map<String,Bool>();
    public var playerMap:Map<Int,Player> = new Map<Int,Player>();
    public var spriteMap:Map<Int,SpriteData> = new Map<Int,SpriteData>();
    public var transitionData:TransitionData;
    public var map:MapData;
    #if openfl
    public var tileData:TileData;
    #end
    public function new()
    {
        map = new MapData();
        #if openfl
        tileData = new TileData();
        #end
        transitionData = new TransitionData();
    }
}