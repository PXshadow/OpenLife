package openlife.auto;
import openlife.data.object.ObjectHelper;
import openlife.data.object.player.PlayerInstance;
import openlife.data.Pos;

interface PlayerInterface
{
    public function getAi() : Ai;
    public function getWorld() : WorldInterface;   
    public function getPlayerInstance() : PlayerInstance;

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

    // variables
    public var id(get, null):Int;
    public var name(get, null):String;
    public var tx(get, null):Int;
    public var ty(get, null):Int;
    public var gx(default, default):Int;
    public var gy(default, default):Int;

    public var food_store(default, default):Float;
    public var food_store_max(default, default):Float;
    public var age(default, default):Float;

    public var mother(get, null):PlayerInterface;
    public var heldObject(default, default):ObjectHelper;

    public function isDeleted() : Bool;
    public function isHuman() : Bool;
    public function isAi() : Bool;
    public function isFemale() : Bool;
    public function isMale() : Bool;
    public function isFertile() : Bool;
    public function isMoving() : Bool;
    public function isWounded() : Bool;
    public function isHoldingWeapon() : Bool;

    public function isBlocked(tx:Int, ty:Int) : Bool;
    
    public function getFollowPlayer():PlayerInterface;
    public function getHeldPlayer():PlayerInterface;
    public function getHeldByPlayer():PlayerInterface;

    public function getCraving():Int;
    public function getCountEaten(foodId:Int) : Float;
}