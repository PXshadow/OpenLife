package data;
import lime.media.AudioBuffer;
import openfl.geom.Rectangle;
#if openfl
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
        trace("numTiles " + parent.numTiles);
        setup();
        trace("numTilesAfter " + parent.numTiles);
    }
    public function setup()
    {
        var sprite:Tile = null;
        var p:Int = 0;
        //temporary
        var tc:TileContainer;
        var bounds:Rectangle;
        var index:Int = 0;
        //offset
        for (i in 0...sprites.length)
        {
            sprite = sprites[i];
            //set pos
            Main.objects.setSprite(sprite,objectData.spriteArray[i],tx,ty);
            //continue;
            sprite.x += param[i].offset.x;
            sprite.y += -param[i].offset.y;
            sprite.originX += param[i].rotationCenterOffset.x;
            sprite.originY += -param[i].rotationCenterOffset.y;
        }
        //parent nesting
        for (i in 0...sprites.length)
        {
            p = objectData.spriteArray.get(i).parent;
            sprite = sprites[i];
            while(p != -1)
            {
                if (!Std.is(sprites[p],TileContainer))
                {
                    tc = new TileContainer();
                    bounds = sprites[p].getBounds(parent);
                    tc.x = bounds.x;
                    tc.y = bounds.y;
                    tc.originX = sprites[p].originX;
                    tc.originY = sprites[p].originY;
                    sprites[p].x = 0;
                    sprites[p].y = 0;
                    sprites[p].originX = 0;
                    sprites[p].originY = 0;
                    sprites[p].parent.removeTile(sprites[p]);
                    tc.addTile(sprites[p]);
                    parent.addTileAt(tc,p);
                    sprites[p] = tc;
                }else{
                    tc = cast sprites[p];
                }
                bounds = sprite.getBounds(parent);
                trace("bound " + bounds.x + " " + bounds.y);
                sprite.x = bounds.x;
                sprite.y = bounds.y;
                sprite.parent.removeTile(sprite);
                tc.addTile(sprite);
                //next
                sprite = sprites[p];
                p = objectData.spriteArray.get(p).parent;
                p = -1;
            }
        }
        //debug
        /*for (i in 0...sprites.length)
        {
            if (Std.is(sprites[i],TileContainer))
            {
                trace(i + " t: c num: " + cast(sprites[i],TileContainer).numTiles);
            }else{
                trace(i + " t: t ");
            }
        }*/
        //for (p in [71,40]) Actuate.tween(sprites[p],1,{rotation:180}).repeat().reflect();
        return;
        //animation
        for (i in 0...param.length)
        {
            sprite = sprites[i];
            //stop
            Actuate.stop(sprite);
            //phase
            sprite.x += phase(param[i].xPhase) * param[i].xAmp;
            sprite.y += phase(param[i].yPhase) * param[i].yAmp;
            //sprite.rotation += -phase(param[i].rotPhase) * 365;
            //sprite.rotation += phase(param[i].rockPhase) * param[i].rockAmp;
            //animate
            if (param[i].xAmp > 0) tween(sprite,{x:sprite.x + param[i].xAmp/2},{x:sprite.x - param[i].xAmp/2},1/param[i].xOscPerSec,param[i].xPhase);
            if (param[i].yAmp > 0) tween(sprite,{y:sprite.y + param[i].yAmp/2},{y:sprite.y - param[i].yAmp/2},1/param[i].yOscPerSec,param[i].yPhase);
            if (param[i].rockAmp > 0) tween(sprite,{rotation:sprite.rotation + (param[i].rockAmp * 365)/2},{rotation:sprite.rotation - (param[i].rockAmp * 365)/2},1/param[i].rockOscPerSec,param[i].rockPhase);
        }
    }
    private inline function phase(x:Float):Float
    {
        if (x > 0.75) return x - 1;
        return (x * 2 - 1) * -2;
    }
    private function tween(sprite:Tile,a:Dynamic,b:Dynamic,time:Float,phase:Float=0)
	{
        //shorten
        if (phase >= 0.25 && phase <= 0.5)
        {
            Actuate.tween(sprite,time/2,b,false).ease(Sine.easeInOut).onComplete(function()
            {
                tween(sprite,a,b,time);
            });
        }else{
		    Actuate.tween(sprite,time/2,a,false).ease(Sine.easeInOut).onComplete(function()
		    {
			    Actuate.tween(sprite,time/2,b,false).ease(Sine.easeInOut).onComplete(function()
                {
                    tween(sprite,a,b,time);
                });
		    });
        }
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
#end