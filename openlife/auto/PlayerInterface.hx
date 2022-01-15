package openlife.auto;
import openlife.data.object.player.PlayerInstance;
import openlife.data.Pos;

interface PlayerInterface
{
    public function getWorld() : WorldInterface;   
    public function getPlayerInstance() : PlayerInstance;

    public function isMoving() : Bool;

    public function doEmote(id:Int, seconds:Int = -10) : Void;
    public function say(text:String, toSelf:Bool = false) : Void;
    //public function eat(); 
    public function self(x:Int = 0, y:Int = 0, clothingSlot:Int = -1) : Void; // for eating and clothing
    public function move(x:Int,y:Int,seq:Int,moves:Array<Pos>) : Void;
    public function remove(x:Int, y:Int, index:Int = -1) : Bool;
    public function specialRemove(x:Int,y:Int,clothingSlot:Int,index:Null<Int>) : Bool;
    public function use(x:Int, y:Int, containerIndex:Int = -1, target:Int = 0) : Bool;
    public function drop(x:Int, y:Int, clothingIndex:Int = -1) : Bool;
    public function dropPlayer() : Bool;
    public function doOnOther(x:Int, y:Int, clothingSlot:Int, playerId:Int) : Bool; // UBABY
    public function doBaby(x:Int, y:Int, playerId:Int) : Bool;
    public function jump() : Bool; 
}