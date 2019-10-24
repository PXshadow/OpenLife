package data;

import openfl.display.Tile;
import data.AnimationData.AnimationParameter;
import haxe.ds.Vector;
class AnimationPlayer
{
    var parents:Map<Int,Array<Int>> = new Map<Int,Array<Int>>();
    public function new(id:Int,int:Int,sprites:Array<Tile>,x:Int=0,y:Int=0)
    {
        var objectData = Main.data.objectMap.get(id);
        if (objectData == null || objectData.animation == null) return;
        var param = objectData.animation.record[int].params;
        if (param == null) return;
        var data:SpriteData = null;
        //parent system

        for (i in 0...param.length)
        {
            data = objectData.spriteArray[i];
            if (data == null) continue;
        }
    }
}