package data.map;
import haxe.ds.Vector;
import data.animation.emote.Emotion;
#if nativeGen @:nativeGen #end
class SceneCell
{
    /**
     * Biome id for ground
     */
    public var biome:Int = 0;
    /**
     * Object id
     */
    public var oID:Int = 0;
    /**
     * Held object id
     */
    public var heldID:Int = 0;
    /**
     * Container format of vector of vector objects
     */
    public var contained:Vector<Vector<Int>>;
    /**
     * Clothing object ids
     */
    public var clothing:Array<Int> = [];
    /**
     * Flip horizontal
     */
    public var flipH:Bool = false;
    /**
     * Age of cell
     */
    public var age:Float = 0;
    /**
     * Held age of potential player
     */
    public var heldAge:Float = 0;
    /**
     * Clothing of potential held player
     */
    public var heldClothing:Array<Int> = [];
    /**
     * Emotion of potential held player
     */
    public var heldEmotion:Emotion;
    /**
     * Animation playing
     */
    public var anim:AnimationData = null;
    /**
     * Animation frozen in time
     */
    public var frozenAnimTime:Float = 0;
    /**
     * Number of uses remaining
     */
    public var numUsesRemaining:Int = 0;
    /**
     * Offset X
     */
    public var xOffset:Int = 0;
    /**
     * Offset Y 
     */
    public var yOffset:Int = 0;
    /**
     * Destination cell offset x
     */
    public var destCellXOffset:Int = 0;
    /**
     * Destination cell offset y
     */
    public var destCellYOffset:Int = 0;
    /**
     * N/A
     */
    public var moveFractionDone:Float = 0;
    /**
     * Movement of cell offset
     */
    public var moveOffset:Point;
    /**
     * Delay of movement
     */
    public var moveDelayTime:Float = 0;
    /**
     * Start of movvement
     */
    public var moveStartTime:Float = 0;
    /**
     * N/A
     */
    public var frameCount:Int = 0;
    /**
     * N/A
     */
    public var graveID:Int = 0;
    /**
     * Emotion of object cell
     */
    public var currentEmot:Emotion;
}