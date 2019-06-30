package data;
import states.game.Player;
//data stored for the game to function (map data -> game data)
class GameData
{
    public var blocking:Map<String,Bool> = new Map<String,Bool>();
    public var playerMap:Map<Int,Player> = new Map<Int,Player>();
    public var map:MapData;
    public function new()
    {
        map = new MapData();
    }
}