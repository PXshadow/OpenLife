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
    public function new(id:Int,int:Int,parent:TileContainer,sprites:Array<Tile>,x:Int=0,y:Int=0)
    {
        //if (current.indexOf(id) > -1) return;
        //current.push(id);
        objectData = Main.data.objectMap.get(id);
        if (objectData == null || objectData.animation == null) return;
        param = objectData.animation.record[int].params;
        this.sprites = sprites;
        this.parent = parent;
        type = objectData.animation.record[int].type;
        tx = x * Static.GRID;
        ty = (Static.tileHeight - y) * Static.GRID;
        trace("tx " + tx + " ty " + ty);
        setup();
    }
    public function setup()
    {
        var sprite:Tile = null;
        var p:Int = 0;
        var index:Int = 0;
        for (i in 0...sprites.length)
        {
            //set pos
            Main.objects.setSprite(sprites[i],objectData.spriteArray[i],tx,ty);
        }
        for (i in 0...sprites.length)
        {
            p = objectData.spriteArray.get(i).parent;
            sprite = sprites[i];
            while(p != -1)
            {
                sprites[p] = container(sprites[p]);
                parent.addTileAt(sprites[p],p);
                index = sprite.parent.getTileIndex(sprite);
                sprite.parent.removeTileAt(index);
                cast(sprites[p],TileContainer).addTileAt(sprite,index);
                //reset
                sprite = sprites[p];
                p = objectData.spriteArray.get(p).parent;
            }
        }
        for (i in 0...param.length)
        {
            sprite = sprites[i];
            //rot phase
            sprite.rotation += param[i].rotPhase * 365;
            //offset
            sprite.x += param[i].offset.x;
            sprite.y += param[i].offset.y;
            sprite.originX += param[i].rotationCenterOffset.x;
            sprite.originY += param[i].rotationCenterOffset.y;
            //animation
            Actuate.stop(sprite);
            if (param[i].xAmp > 0) tween(sprite,{x:sprite.x + param[i].xAmp/2},{x:sprite.x - param[i].xAmp/2},1/param[i].xOscPerSec);
            if (param[i].yAmp > 0) tween(sprite,{y:sprite.y + param[i].yAmp/2},{y:sprite.y - param[i].yAmp/2},1/param[i].yOscPerSec);
            //sprite.rotation = -30;
            //Actuate.tween(sprite,1,{rotation:30}).repeat().reflect();
            if (param[i].rockAmp > 0) tween(sprite,{rotation:sprite.rotation + (param[i].rockAmp * 365)/2},{rotation:sprite.rotation - (param[i].rockAmp * 365)/2},1/param[i].rockOscPerSec);
            /*if (param[i].rotPerSec > 0)
            {
                Actuate.tween(sprites[i],1/param[i].rotPerSec,{rotation:365}).ease(Linear.easeNone);
                trace("rps " + param[i].rotPerSec);
            }*/
        }
    }
    private function container(tile:Tile):TileContainer
    {
        var c = new TileContainer();
        tile.parent.removeTile(tile);
        c.addTile(tile);
        return c;
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
    private function clean()
    {
        for (i in 0...sprites.length)
        {
            Actuate.stop(sprites[i]);
            if (!Std.is(sprites[i],TileContainer))
            {
                if (!parent.contains(sprites[i]))
                {
                    sprites[i].parent.removeTile(sprites[i]);
                    parent.addTile(sprites[i]);
                }
            }
        }
    }
}