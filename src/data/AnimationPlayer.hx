package data;

import motion.easing.Linear;
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
    var parent:TileContainer;
    var time:Float = 0;
    private static var current:Array<Int> = [];
    var param:Vector<AnimationParameter>;
    var sprites:Array<Tile> = [];
    var type:AnimationType = null;
    var objectData:ObjectData;
    //tile position
    var tx:Float = 0;
    var ty:Float = 0;
    public function new(id:Int,int:Int,sprites:Array<Tile>,x:Int=0,y:Int=0)
    {
        //if (current.indexOf(id) > -1) return;
        //current.push(id);
        objectData = Main.data.objectMap.get(id);
        if (objectData == null || objectData.animation == null) return;
        param = objectData.animation.record[int].params;
        this.sprites = sprites;
        type = objectData.animation.record[int].type;
        tx = x * Static.GRID;
        ty = (Static.tileHeight - y) * Static.GRID;
        parent = sprites[0].parent;
        trace("tx " + tx + " ty " + ty);
        setup();
    }
    public function setup()
    {
        var p:Int = 0;
        for (i in 0...sprites.length)
        {
            //set pos
            Main.objects.setSprite(sprites[i],objectData.spriteArray[i],tx,ty);
            //rot phase
            sprites[i].rotation += param[i].rotPhase * 365;
            //offset
            sprites[i].x += param[i].offset.x;
            sprites[i].y += param[i].offset.y;
            sprites[i].originX += param[i].rotationCenterOffset.x;
            sprites[i].originY += param[i].rotationCenterOffset.y;
        }
        //establish parent list
        for (i in 0...param.length)
        {
            p = objectData.spriteArray[i].parent;
            if (p > -1)
            {
                var container = parents.get(p);
                if (container == null)
                {
                    container = new TileContainer();//sprites[p].x,sprites[p].y,1,1,sprites[p].rotation,sprites[p].originX,sprites[p].originY);
                    /*sprites[p].x = 0;
                    sprites[p].y = 0;
                    sprites[p].rotation = 0;
                    sprites[p].originX = 0;
                    sprites[p].originY = 0;*/
                    parent.addTile(container);
                    parent.removeTile(sprites[p]);
                    container.addTile(sprites[p]);
                    parents.set(p,container);
                }
                sprites[i].parent.removeTile(sprites[i]);
                sprites[i].x = sprites[i].x - container.x;
                sprites[i].y = sprites[i].y - container.y;
                container.addTile(sprites[i]);
            }
        }
        var sprite:Tile = null;
        for (i in 0...param.length)
        {
            sprite = parents.get(i);
            if (sprite == null) sprite = sprites[i];
            //animation
            Actuate.stop(sprite);
            if (param[i].xAmp > 0) tween(sprite,{x:sprite.x + param[i].xAmp/2},{x:sprite.x - param[i].xAmp/2},1/param[i].xOscPerSec);
            if (param[i].yAmp > 0) tween(sprite,{y:sprite.y + param[i].yAmp/2},{y:sprite.y - param[i].yAmp/2},1/param[i].yOscPerSec);
            if (param[i].rockAmp > 0) tween(sprite,{rotation:sprite.rotation + (param[i].rockAmp * 365)/2},{rotation:sprite.rotation - (param[i].rockAmp * 365)/2},1/param[i].rockOscPerSec);
            /*if (param[i].rotPerSec > 0)
            {
                Actuate.tween(sprites[i],1/param[i].rotPerSec,{rotation:365}).ease(Linear.easeNone);
                trace("rps " + param[i].rotPerSec);
            }*/
        }
    }
    private function tween(sprite:Tile,a:Dynamic,b:Dynamic,time:Float)
	{
		Actuate.tween(sprite,time/2,a,false).ease(Sine.easeInOut).onComplete(function()
		{
			Actuate.tween(sprite,time/2,b,false).ease(Sine.easeInOut).onComplete(function()
            {
                tween(sprite,a,b,time);
            });
		});
	}
}