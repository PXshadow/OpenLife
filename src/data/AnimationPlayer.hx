package data;

import motion.easing.Sine;
import data.AnimationData.AnimationType;
import motion.Actuate;
import haxe.Timer;
import openfl.display.Tile;
import openfl.display.TileContainer;
import data.AnimationData.AnimationParameter;
import haxe.ds.Vector;
class AnimationPlayer
{
    var parents:Map<Int,TileContainer> = new Map<Int,TileContainer>();
    var parent:TileContainer = null;
    var time:Float = 0;
    private static var current:Array<Int> = [];
    var param:Vector<AnimationParameter>;
    var sprites:Array<Tile> = [];
    var type:AnimationType = null;
    public function new(id:Int,int:Int,sprites:Array<Tile>,x:Int=0,y:Int=0)
    {
        if (current.indexOf(id) > -1) return;
        current.push(id);
        var objectData = Main.data.objectMap.get(id);
        if (objectData == null || objectData.animation == null) return;
        param = objectData.animation.record[int].params;
        this.sprites = sprites;
        type = objectData.animation.record[int].type;
        setup();
    }
    public function setup()
    {
        
    }
    private function getOscOffset(inFrameTime:Float,inOffset:Float,inOscPerSec:Float,inAmp:Float,inPhase:Float):Float
    {
        return inOffset + inAmp * Math.sin((inFrameTime * inOscPerSec + inPhase) * 2 * Math.PI);
    }
}