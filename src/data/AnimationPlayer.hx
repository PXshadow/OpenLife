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
import openfl.geom.Point;
class AnimationPlayer extends Player<AnimationChannel>
{
    /*var parent:TileContainer;
    var timers:Array<Timer> = [];
    var timerInt:Int = 0;
    private static var current:Array<Int> = [];
    var param:Vector<AnimationParameter>;
    var sprites:Array<Tile> = [];
    var type:AnimationType = null;
    var objectData:ObjectData;
    //tile position
    var tx:Float = 0;
    var ty:Float = 0;*/
    public function new()
    {
        super();
        /*objectData = Main.data.objectMap.get(id);
        if (objectData == null || objectData.animation == null) return;
        param = objectData.animation.record[int].params;
        this.sprites = sprites;
        this.parent = parent;
        type = objectData.animation.record[int].type;
        tx = x * Static.GRID;
        ty = (Static.tileHeight - y) * Static.GRID;*/
        //setup();
    }
    public function play(id:Int,index:Int,parent:TileContainer,x:Float,y:Float) 
    {
        var data = new AnimationChannel();
        active.push(data);
        var objectData = Main.data.objectMap.get(id);
        if (objectData == null || objectData.animation == null) return;
        var param = objectData.animation.record[index].params;
        var type = objectData.animation.record[index].type;
        var tx = x * Static.GRID;
        var ty = (Static.tileHeight - y) * Static.GRID;
        var sprite:Tile = null;
        var p:Int = 0;
        var timerInt:Int = 0;
        //temporary
        var tc:TileContainer;
        //sprite parent
        var sp:TileContainer;
        var point:Point = null;
        var index:Int = 0;
        //offset
        for (i in 0...data.sprites.length)
        {
            sprite = data.sprites[i];
            //set pos
            Main.objects.setSprite(sprite,objectData.spriteArray[i],tx,ty);
            sprite.x += param[i].offset.x;
            sprite.y += -param[i].offset.y;
            sprite.originX += param[i].rotationCenterOffset.x;
            sprite.originY += -param[i].rotationCenterOffset.y;
            sprite.data = {rotation:0.0};
            //parent
            p = objectData.spriteArray[i].parent;
            if (p != -1)
            {
                var px:Float = data.sprites[p].x;
                var py:Float = data.sprites[p].y;
                var pr:Float = data.sprites[p].rotation;
                data.timers[timerInt++] = new Timer(1/60 * 1000);
                data.timers[timerInt - 1].run = function()
                {
                    if (px != data.sprites[p].x)
                    {
                        data.sprites[i].x += data.sprites[p].x - px;
                        px = data.sprites[p].x;
                    }
                    if (py != data.sprites[p].y)
                    {
                        data.sprites[i].y += data.sprites[p].y - py;
                        py = data.sprites[p].y;
                    }
                    if (pr != data.sprites[p].rotation)
                    {
                        /*var rad = Math.atan2(sprite.y - sprites[p].y,sprite.x - sprites[p].x);
                        var dis = Math.sqrt(Math.pow(sprites[i].y - sprite.y,2) + Math.pow(sprite.x - sprites[p].x,2));
                        sprite.x = sprites[p].x + dis * Math.cos(rad);
                        sprite.y = sprites[p].y + dis * Math.sin(rad);*/
                        data.sprites[i].matrix.translate(-data.sprites[p].x,-data.sprites[p].y);
                        data.sprites[i].matrix.rotate((data.sprites[p].rotation - pr) * (Math.PI/180));
                        data.sprites[i].matrix.translate(data.sprites[p].x,data.sprites[p].y);
                        data.sprites[i].rotation += (data.sprites[p].rotation - pr);
                        pr = data.sprites[p].rotation;
                        //overwriting acutated tween
                    }
                }
            }
        }
        //return;
        var px:Float = 0;
        var py:Float = 0;
        var pr:Float = 0;
        for (i in 0...param.length)
        {
            sprite = data.sprites[i];
            //stop
            Actuate.stop(sprite);
            //phase
            px = phase(param[i].xPhase) * param[i].xAmp;
            py = phase(param[i].yPhase) * param[i].yAmp;
            pr = param[i].rotPhase * 365;
            sprite.x += px;
            sprite.y += py;
            sprite.rotation += pr;
            //sprite.rotation = phase(param[i].rockPhase) * param[i].rockAmp;
            //play
            if (param[i].rockOscPerSec > 0) tween(sprite,{alpha:param[i].fadeMin},{alpha:param[i].fadeMax},1/param[i].fadeOscPerSec,param[i].fadePhase);
            if (param[i].xAmp > 0) tween(sprite,{x:sprite.x + param[i].xAmp/2},{x:sprite.x - param[i].xAmp/2},1/param[i].xOscPerSec,param[i].xPhase,px);
            if (param[i].yAmp > 0) tween(sprite,{y:sprite.y + param[i].yAmp/2},{y:sprite.y - param[i].yAmp/2},1/param[i].yOscPerSec,param[i].yPhase,py);
            if (param[i].rockAmp > 0) tween(sprite,{rotation:sprite.rotation + (param[i].rockAmp * 365)},{rotation:sprite.rotation - (param[i].rockAmp * 365)},1/param[i].rockOscPerSec,param[i].rockPhase);
        }
    }
    private inline function phase(x:Float):Float
    {
        if (x > 0.75) return x - 1;
        return (x * 2 - 1) * -2;
    }
    private function tween(sprite:Tile,a:Dynamic,b:Dynamic,time:Float,phase:Float=0,phaseValue:Float=0)
	{
        //shorten
        if (phase >= 0.25 && phase <= 0.5)
        {
            Actuate.tween(sprite,time/2,tweenPhase(b,phaseValue),false).ease(Sine.easeInOut).onComplete(function()
            {
                tween(sprite,a,b,time);
            });
        }else{
		    Actuate.tween(sprite,time/2,(phase > 0 ? tweenPhase(a,phaseValue) : a),false).ease(Sine.easeInOut).onComplete(function()
		    {
			    Actuate.tween(sprite,time/2,b,false).ease(Sine.easeInOut).onComplete(function()
                {
                    tween(sprite,a,b,time);
                });
		    });
        }
	}
    private function tweenPhase(o:Dynamic,sub:Float):Dynamic
    {
        var name = Reflect.fields(o)[0];
        var value:Float = Reflect.field(o,name);
        Reflect.setField(o,name,value - sub * 2);
        return o;
    }
}
class AnimationChannel
{
    public var timers:Array<Timer>;
    public var sprites:Array<Tile>;
    public function new()
    {

    }
}
#end