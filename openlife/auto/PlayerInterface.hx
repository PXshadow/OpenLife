package openlife.auto;

import haxe.ds.Vector;
import openlife.data.Pos;
import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;
import openlife.data.object.player.PlayerInstance;

interface PlayerInterface {
	public function getAi():AiBase;
	public function getWorld():WorldInterface;
	public function getPlayerInstance():PlayerInstance;

	public function doEmote(id:Int, seconds:Int = -10):Void;
	public function say(text:String, toSelf:Bool = false):Void;
	// public function eat();
	public function self(x:Int = 0, y:Int = 0, clothingSlot:Int = -1):Void; // for eating and clothing
	public function move(x:Int, y:Int, seq:Int, moves:Array<Pos>):Void;
	public function remove(x:Int, y:Int, index:Int = -1):Bool;
	public function specialRemove(x:Int, y:Int, clothingSlot:Int, index:Null<Int>):Bool;
	public function use(x:Int, y:Int, containerIndex:Int = -1, target:Int = 0):Bool;
	public function drop(x:Int, y:Int, clothingIndex:Int = -1):Bool;
	public function dropPlayer(x:Int, y:Int):Bool;
	public function doOnOther(x:Int, y:Int, clothingSlot:Int, playerId:Int):Bool; // UBABY
	public function doBaby(x:Int, y:Int, playerId:Int):Bool;
	public function jump():Bool;

	// variables
	public var id(get, null):Int;
	public var name(get, null):String;
	public var x(default, default):Int;
	public var y(default, default):Int;
	public var gx(default, default):Int;
	public var gy(default, default):Int;
	public var tx(get, null):Int;
	public var ty(get, null):Int;

	public var food_store(default, default):Float;
	public var food_store_max(default, default):Float;
	public var age(default, default):Float;
	public var hits(default, default):Float;

	public var mother(get, null):PlayerInterface;
	public var heldObject(default, default):ObjectHelper;
	public var hiddenWound(default, default):ObjectHelper;

	public var clothingObjects(default, default):Vector<ObjectHelper>;

	public function isDeleted():Bool;
	public function isHuman():Bool;
	public function isAi():Bool;
	public function isFemale():Bool;
	public function isMale():Bool;
	public function isFertile():Bool;
	public function isMoving():Bool;
	public function isWounded():Bool;
	public function isHoldingWeapon():Bool;
	public function isBlocked(tx:Int, ty:Int):Bool;
	public function isEveOrAdam():Bool;
	public function isIll():Bool;
	public function isAnimalDeadlyForMe(animal:ObjectHelper):Bool;
	public function isAnimalNotDeadlyForMe(animal:ObjectHelper):Bool;

	public function isHoldingYum():Bool;
	public function isYum(food:ObjectHelper):Bool;
	public function isMeh(food:ObjectHelper):Bool;
	public function canEat(food:ObjectHelper):Bool;
	public function canFeedToMe(food:ObjectHelper):Bool;
	public function canFeedToMeObj(objData:ObjectData):Bool;
	public function getMaxChildFeeding():Float; // gives back how much a child can be fed

	public function getFollowPlayer():PlayerInterface;
	public function getHeldPlayer():PlayerInterface;
	public function getHeldByPlayer():PlayerInterface;

	public function getCraving():Int;
	public function getCountEaten(foodId:Int):Float;
}
