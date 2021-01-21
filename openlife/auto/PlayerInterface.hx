package openlife.auto;
import openlife.data.object.player.PlayerInstance;
import openlife.data.Pos;

interface PlayerInterface
{
    public function getWorld() : WorldInterface;   
    public function getPlayerInstance() : PlayerInstance;

    public function say(text:String):Void;
    public function self(x:Int,y:Int,clothingSlot:Int):Void;
    public function move(x:Int,y:Int,seq:Int,moves:Array<Pos>):Void;
    public function remove(x:Int, y:Int, index:Int = -1) : Bool;
    public function specialRemove(x:Int,y:Int,clothingSlot:Int,index:Null<Int>) : Bool;
    public function use(x:Int, y:Int, containerIndex:Int = -1, target:Int = 0) : Bool;
    public function drop(x:Int, y:Int, clothingIndex:Int = -1) : Bool;
    public function dropPlayer() : Bool;
    public function doOnOther(x:Int, y:Int, clothingSlot:Int, playerId:Int) : Bool; // UBABY
    public function doBaby(x:Int, y:Int, playerId:Int) : Bool;

    
}