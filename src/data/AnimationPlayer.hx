package data;

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
    public function new(id:Int,int:Int,sprites:Array<Tile>,x:Int=0,y:Int=0)
    {
        if (current.indexOf(id) > -1) return;
        current.push(id);

        var objectData = Main.data.objectMap.get(id);
        if (objectData == null || objectData.animation == null) return;
        var param = objectData.animation.record[int].params;
        if (param == null) return;
        var data:SpriteData = null;
        trace("play animation: " + int + " id: " + id);
        //parent system
        parent = sprites[0].parent;
        if (parent == null) return;
        //animation
        for (i in 0...param.length)
        {
            data = objectData.spriteArray[i];
            if (data == null /*|| param[i] == null*/) continue;
            Actuate.tween(sprites[i],param[i].durationSec,{rotation:param[i].rotPhase * 365});
            time = Math.max(time,param[i].durationSec);
        }
        trace("time " + time);
        //re-position
        Timer.delay(function()
        {
            trace("reset");
            for (i in 0...sprites.length)
            {
                data = objectData.spriteArray[i];
                if (data == null) continue;
                Main.objects.setSprite(sprites[i],data,x,y);
            }
            current.remove(id);
        },Std.int(time * 1000));
    }
}