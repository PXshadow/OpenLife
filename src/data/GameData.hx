package data;
import openfl.display.Tile;
import openfl.geom.Rectangle;
import states.game.Player;
//data stored for the game to function (map data -> game data)
class GameData
{
    //block walking
    public var blocking:Map<String,Bool> = new Map<String,Bool>();
    public var playerMap:Map<Int,Player> = new Map<Int,Player>();
    public var spriteMap:Map<Int,SpriteData> = new Map<Int,SpriteData>();
    public var map:MapData;
    public var chunk:ChunkData;
    public function new()
    {
        map = new MapData();
        chunk = new ChunkData();
    }
}