package openlife.auto;

import openlife.data.object.ObjectHelper;
import openlife.data.object.player.PlayerInstance;


abstract class AiBase
{
    public var seqNum = 1;
    public var myPlayer(default, default):PlayerInterface;

    public abstract function doTimeStuff(timePassedInSeconds:Float) : Void;

    public abstract function newChild(child:PlayerInterface) : Void;
    public abstract function say(player:PlayerInterface, curse:Bool, text:String) : Void;
    public abstract function finishedMovement() : Void;
    public abstract function newBorn() : Void;
    public abstract function emote(player:PlayerInstance, index:Int) : Void;
	public abstract function playerUpdate(player:PlayerInstance) : Void;
	public abstract function mapUpdate(targetX:Int, targetY:Int, isAnimal:Bool = false) : Void;
	public abstract function playerMove(player:PlayerInstance, targetX:Int, targetY:Int) : Void;

    public abstract function isObjectNotReachable(tx:Int, ty:Int):Bool;
    public abstract function addNotReachableObject(obj:ObjectHelper, time:Float = 90) : Void;
    public abstract function addNotReachable(tx:Int, ty:Int, time:Float = 90) : Void;
    public abstract function isObjectWithHostilePath(tx:Int, ty:Int):Bool; 

    public abstract function resetTargets() : Void;
}