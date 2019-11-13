package data.object;
//gets set by objectData
class SpriteData
{
    /**
     * Name of sprite
     */
    public var name:String;
    /**
     * Id of sprite
     */
    public var spriteID:Int=54;
    /**
     * position of sprite
     */
    public var pos:Point;//=166.000000,107.000000
    /**
     * Rotation
     */
    public var rot:Float=0.000000;
    /**
     * Horizontal flip
     */
    public var hFlip:Int=0;
    /**
     * Color Array RGB 0-1
     */
    public var color:Array<Float> = [];//=0.952941,0.796078,0.756863
    /**
     * Age range Array -1-60
     */
    public var ageRange:Array<Float> = [];//=-1.000000,-1.000000
    /**
     * Parent id for child
     */
    public var parent:Int = 0;//=-1
    /**
     * Invisible when holding
     */
    public var invisHolding:Bool = false;
    /**
     * Invisible when worn
     */
    public var invisWorn:Bool = false;
    /**
     * Behind slots
     */
    public var behindSlots:Bool = false;
    /**
     * N/A
     */
    public var invisCont:Bool = false;
    /**
     * Offset of center x
     */
    public var inCenterXOffset:Int = 0;
    /**
     * Offset of center y
     */
    public var inCenterYOffset:Int = 0;
    public function new()
    {

    }
}