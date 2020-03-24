package data.animation;
import game.Objects;
import data.sound.SoundData;
#if openfl
import lime.media.AudioBuffer;
import openfl.geom.Rectangle;
import motion.easing.Linear;
import motion.easing.Sine;
import data.animation.AnimationType;
import motion.Actuate;
import haxe.Timer;
import openfl.display.Tile;
import openfl.display.TileContainer;
import data.animation.AnimationParameter;
import data.animation.AnimationChannel;
import haxe.ds.Vector;
import game.Game;
import openfl.geom.Point;
#if nativeGen @:nativeGen #end
class AnimationPlayer
{
    var objects:Objects;
    /**
     * Active animations playing
     */
    private var active:Array<AnimationChannel> = [];
    /**
     * Create new player
     */
    public function new(objects:Objects)
    {
        this.objects = objects;
    }
    /**
     * Play animation
     * @param id object id
     * @param index index of animation
     * @param sprites sprites of object
     * @param x tile x
     * @param y tile y
     */
    public function play(id:Int,index:Int,sprites:Array<Tile>,x:Float,y:Float,cloths:Vector<Array<Tile>>=null) 
    {
        //check if already up
        for (obj in active)
        {
            if (obj.id == id && Static.arrayEqual(obj.sprites,sprites)) return;
        }
        var data = new AnimationChannel();
        data.cloths = cloths;
        data.id = id;
        data.x = x * Static.GRID;
        data.y = (Static.tileHeight - y) * Static.GRID;
        data.sprites = sprites;
        active.push(data);
        //data set
        var objectData = Game.data.objectMap.get(id);
        if (objectData == null || objectData.animation == null) return;
        var param = objectData.animation.record[index].params;
        if (param == null) return;
        var type = objectData.animation.record[index].type;
        var sprite:Tile = null;
        var p:Int = 0;
        var timerInt:Int = 0;
        //sounds
        /*var soundData:SoundData;
        for (soundParam in objectData.animation.record[index].soundAnim)
        {
            //Main.sounds.play()
            //for ()
        }*/
        //temporary
        var tc:TileContainer;
        //sprite parent
        var sp:TileContainer;
        var point:Point = null;
        var index:Int = 0;
        //offset
        for (i in 0...param.length)
        {
            sprite = sprites[i];
            //set pos
            objects.setSprite(sprite,objectData.spriteArray[i],data.x,data.y);
            sprite.x += param[i].offset.x;
            sprite.y += -param[i].offset.y;
            p = objectData.spriteArray[i].parent;
            if (p != -1)
            {
                sprite.data = {px:sprites[p].x,py:sprites[p].y,pr:sprites[p].rotation,rps:param[i].rotPerSec};
            }
        }
        //set clothing parent
        if (cloths != null)
        {
            for (i in 0...cloths.length)
            {
                if (cloths[i] == null) continue;
                switch(i)
                {
                    case 0: p = objectData.headIndex;
                    case 1 | 4 | 5: p = objectData.bodyIndex;
                    case 2: p = objectData.frontFootIndex;
                    case 3: p = objectData.backFootIndex;
                }
                for (sprite in cloths[i])
                {
                    sprite.data = {px:sprites[p].x,py:sprites[p].y,pr:sprites[p].rotation};
                }
            }
        }
        //update
        data.timer = new Timer(1/60 * 1000);
        data.timer.run = function()
        {
            for (i in 0...data.sprites.length)
            {
                //parent
                p = objectData.spriteArray[i].parent;
                if (p != -1)
                {
                    update(sprites[i],sprites[p]);
                }
            }
            //cloths
            if (cloths != null)
            {
                for (i in 0...cloths.length)
                {
                    if (cloths[i] == null) continue;
                    switch(i)
                    {
                        case 0: p = objectData.headIndex;
                        case 1 | 4 | 5: p = objectData.bodyIndex;
                        case 2: p = objectData.frontFootIndex;
                        case 3: p = objectData.backFootIndex;
                    }
                    for (sprite in cloths[i])
                    {
                        update(sprite,sprites[p]);
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
            sprite = sprites[i];
            //stop
            Actuate.stop(sprite);
            //phase
            px = phase(param[i].xPhase) * param[i].xAmp;
            py = phase(param[i].yPhase) * param[i].yAmp;
            pr = param[i].rotPhase * 360;
            sprite.x += px;
            sprite.y += py;
            sprite.rotation += pr;
            //sprite.rotation = phase(param[i].rockPhase) * param[i].rockAmp;
            //play
            if (param[i].rockOscPerSec > 0) tween(sprite,{alpha:param[i].fadeMin},{alpha:param[i].fadeMax},1/param[i].fadeOscPerSec,param[i].fadePhase);
            if (param[i].xAmp > 0) tween(sprite,{x:sprite.x + param[i].xAmp/2},{x:sprite.x - param[i].xAmp/2},1/param[i].xOscPerSec,param[i].xPhase,px);
            if (param[i].yAmp > 0) tween(sprite,{y:sprite.y + param[i].yAmp/2},{y:sprite.y - param[i].yAmp/2},1/param[i].yOscPerSec,param[i].yPhase,py);
            if (param[i].rockAmp > 0) tween(sprite,{rotation:sprite.rotation + (param[i].rockAmp * 360)},{rotation:sprite.rotation - (param[i].rockAmp * 360)},1/param[i].rockOscPerSec,param[i].rockPhase);
            if (param[i].rotPerSec != 0)
            {
                var dir = param[i].rotPerSec > 0 ? 1 : -1;
                trace("rot " + param[i].rotPerSec);
                rotate(sprite,1/param[i].rotPerSec * dir * 1,dir,365 * dir);
            }
        }
    }
    private function rotate(sprite:Tile,sec:Float,dir:Int,rot:Int)
    {
        Actuate.tween(sprite,sec,{rotation:rot}).onComplete(function(_)
        {
            rotate(sprite,sec,dir,rot + 365 * dir);
        }).ease(Linear.easeNone);
    }
    /**
     * update
     * @param sprite 
     * @param parent 
     */
    private inline function update(sprite:Tile,parent:Tile)
    {
        if (sprite.data.px != parent.x)
        {
            sprite.x += parent.x - sprite.data.px;
            sprite.data.px = parent.x;
        }
        if (sprite.data.py != parent.y)
        {
            sprite.y += parent.y - sprite.data.py;
            sprite.data.py = parent.y;
        }
        if (sprite.data.pr != parent.rotation)
        {
            sprite.matrix.translate(-parent.x,-parent.y);
            sprite.matrix.rotate((parent.rotation - sprite.data.pr) * (Math.PI/180));
            sprite.matrix.translate(parent.x,parent.y);
            sprite.rotation += (parent.rotation - sprite.data.pr);
            sprite.data.pr = parent.rotation;
        }
    }
    /**
     * Clear animation with object's sprites
     * @param sprites 
     */
    public function clear(sprites:Array<Tile>)
    {
        for (obj in active)
        {
            if (Static.arrayEqual(obj.sprites,sprites))
            {
                stop(obj);
                return;
            }
        }
    }
    /**
     * Stop animation with data refrence
     * @param data 
     */
    public function stop(data:AnimationChannel) 
    {
        active.remove(data);
        //Actuate.pauseAll();
        var objectData = Game.data.objectMap.get(data.id);
        //reset player sprites
        for (i in 0...data.sprites.length) 
        {
            Actuate.stop(data.sprites[i],null,false,false);
            objects.setSprite(data.sprites[i],objectData.spriteArray[i],data.x,data.y);
        }
        //reset clothing
        
        data.timer.stop();
        data = null;
    }
    /**
     * Phase algorithm
     * @param x value
     * @return Float phased value
     */
    private inline function phase(x:Float):Float
    {
        if (x > 0.75) return x - 1;
        return (x * 2 - 1) * -2;
    }
    @:doxHide(false)
    /**
     * Tweened sprite
     * @param sprite focus
     * @param a inital
     * @param b end
     * @param time duration
     * @param phase pre-processed
     * @param phaseValue processed
     */
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
#end