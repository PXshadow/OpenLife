package game;
import data.map.MapInstance;
import data.object.player.PlayerInstance;
import data.object.player.PlayerMove;
import data.map.MapChange;
class GameHeader #if openfl extends openfl.display.Sprite #elseif heaps hxd.App #end
{
    public function playerEmot(p_id:Int,emot_index:Int,ttl_sec:Int) {};
    public function playerUpdate(instance:PlayerInstance) {};
    public function playerMoveStart(move:PlayerMove) {};
    public function playerHeatchange() {};
    public function playerName() {};

    public function mapChunk(instance:MapInstance) {};
    public function mapChange(change:MapChange) {};

    public function heatChange(current:Float,total:Float) {};
    public function frame() {};
    public function lineage() {};
    public function healed() {};
    public function monument() {};
    public function grave() {};
    public function graveMove() {};
    public function dying() {};
    public function ownerList() {};
    public function valley() {};
    public function flight() {};
}