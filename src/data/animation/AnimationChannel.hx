package data.animation;
#if openfl
import haxe.Timer;
import openfl.display.Tile;
class AnimationChannel
{
    /**
     * Timers used to update children to parent's transformations
     */
    public var timers:Array<Timer> = [];
    /**
     * Sprites of animated object
     */
    public var sprites:Array<Tile> = [];
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