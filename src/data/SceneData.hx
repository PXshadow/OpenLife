package data;

import haxe.ds.Vector;

class SceneData
{
    public function new()
    {

    }
}
class SceneCell
{
    public var biome:Int = 0;
    public var oID:Int = 0;
    public var heldID:Int = 0;
    
    public var contained:Vector<Vector<Int>>;

    public var clothing:Array<Int> = [];

    public var flipH:Bool = false;
    public var age:Float = 0;

    public var heldAge:Float = 0;
    public var heldClothing:Array<Int> = [];
    public var heldEmotion:Emotion;
    public var anim:AnimationData = null;
    public var frozenAnimTime:Float = 0;
    public var numUsesRemaining:Int = 0;
    public var xOffset:Int = 0;
    public var yOffset:Int = 0;
    public var destCellXOffset:Int = 0;
    public var destCellYOffset:Int = 0;
    public var moveFractionDone:Float = 0;
    public var moveOffset:Point;
    public var moveDelayTime:Float = 0;
    public var moveStartTime:Float = 0;
    public var frameCount:Int = 0;
    public var graveID:Int = 0;
    public var currentEmot:Emotion;
}