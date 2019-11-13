package data.animation;
#if openfl
import haxe.ds.Vector;
import data.animation.AnimationParameter;
import data.animation.AnimationType;
import data.animation.AnimationData;
class AnimationRecord
{
    public var id:Int = -1;
    public var type:AnimationType;
    public var params:Vector<AnimationParameter>;
    public var numSounds:Int = 0;
    public var numSprites:Int = 0;
    public var numSlots:Int = 0;
    public var randStartPhase:Float = 0;
    public var forceZeroStart:Float = 0;
    public function new()
    {

    }
    public function toString():String
    {
        var string:String = "";
        for(field in Reflect.fields(this))
        {
            string += field + ": " + Reflect.getProperty(this,field) + "\n";
        }
        return string;
    }
}
#end