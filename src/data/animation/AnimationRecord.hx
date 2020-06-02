package data.animation;
import haxe.ds.Vector;
import data.animation.AnimationParameter;
import data.animation.AnimationType;
import data.animation.AnimationData;

class AnimationRecord
{
    /**
     * Record id
     */
    public var id:Int = -1;
    /**
     * Record type
     */
    public var type:AnimationType;
    /**
     * Parameters of record
     */
    public var params:Vector<AnimationParameter>;
    /**
     * Number of sounds
     */
    public var numSounds:Int = 0;
    /**
     * Number of sprites
     */
    public var numSprites:Int = 0;
    /**
     * Number of slots
     */
    public var numSlots:Int = 0;
    /**
     * Random start phase
     */
    public var randStartPhase:Float = 0;
    /**
     * N/A
     */
    public var forceZeroStart:Float = 0;
    /**
     * Sound record
     */
    public var soundAnim:Vector<SoundParameter>;
    /**
     * Slot animation
     */
     public var slotAnim:Vector<AnimationParameter>;
    public function new()
    {

    }
    /**
     * String to show fields and properties
     * @return String
     */
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