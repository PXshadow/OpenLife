package data;
import game.Ground;
import game.Player;
import openfl.display.TileContainer;
import openfl.display.Tile;
import openfl.geom.Rectangle;
//data stored for the game to function (map data -> game data)
class GameData
{
    //block walking
    public var blocking:Map<String,Bool> = new Map<String,Bool>();
    public var playerMap:Map<Int,Player> = new Map<Int,Player>();
    public var spriteMap:Map<Int,SpriteData> = new Map<Int,SpriteData>();
    public var map:MapData;
    public var tileData:TileData;
    public function new()
    {
        map = new MapData();
        tileData = new TileData();
    }
}