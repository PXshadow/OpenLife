package data.animation;
import haxe.ds.Vector;
#if openfl
import haxe.Timer;
import openfl.display.Tile;
#if nativeGen @:nativeGen #end
class AnimationChannel
{
    /**
     * Timers used to update children to parent's transformations
     */
    public var timer:Timer;
    /**
     * Sprites of animated object
     */
    public var sprites:Array<Tile> = [];
    /**
     * clothing of animated object
     */
     public var cloths:Vector<Array<Tile>> = null;
    /**
     * Tile X
     */
    public var x:Float = 0;
    /**
     * Tile Y
     */
    public var y:Float = 0;
    /**
     * Object Id
     */
    public var id:Int = 0;
    /**
     * Create new Animation Channel
     */
    public function new()
    {

    }
}
#end