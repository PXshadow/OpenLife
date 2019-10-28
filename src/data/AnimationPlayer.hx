package data;

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
    var inFrameTime:Float = 0;
    var inAnimFade:Float = 0;
    var inFadeTargetAnim:Dynamic = null;
    var inFadeTargetFrameTime:Float = 0;
    var inFrozenRotFrameTime:Float = 0;
    var outFrozenRotFrameTimeUsed:Float = 0;
    var inPos:Point;
    var inRot:Float = 0;
    var inWorn:Bool = false;
    var inFlipH:Bool = false;
    var inAge:Int = 0;
    var type:AnimationType = null;
    public function new(id:Int,int:Int,sprites:Array<Tile>,x:Int=0,y:Int=0)
    {
        if (current.indexOf(id) > -1) return;
        current.push(id);
        var objectData = Main.data.objectMap.get(id);
        if (objectData == null || objectData.animation == null) return;
        param = objectData.animation.record[int].params;
        this.sprites = sprites;
        swtich(int)
        {
            case 1: type = ground;
            case 2: type = held;
            case 3: type = moving;
            case 4: type = eating;
            case 5: type = doing;
        }
    }
    public function update()
    {
        for (i in 0...param.length)
        {
            var spriteFrameTime = inFrameTime;
            var targetSpriteFrameTime = inFadeTargetFrameTime;
            var sinVal:Float = getOscOffset(inFrameTime,0,param[i].fadeOscPerSec,1,param[i].fadePhase + 0.25);
            var hardVersion:Float = 0;
            var hardness:Float = param[i].fadeHardness;
            if (hardness == 1)
            {
                if (sinVal > 0)
                {
                    hardVersion = 1;
                }else{
                    hardVersion = -1;
                }
            }else{
                var absSinVal = Math.abs(sinVal);
                if (sinVal != 0)
                {
                    hardVersion = (sinVal/absSinVal) * Math.pow(absSinVal,1/(hardness * 10 + 1));
                }else{
                    hardVersion = 0;
                }
            }

            var fade = (param[i].fadeMax - param[i].fadeMin) * (0.5 * hardVersion + 0.5) + param[i].fadeMin;
            if( hardness == 1 ) 
            {
                // don't apply cross-fade to fades
                sprites[i].alpha = fade;
            }else{
                // crossfade the fades
                sprites[i].alpha = inAnimFade * fade;
            }
            
            sprites[i].x += getOscOffset(inFrameTime,param[i].offset.x,param[i].xOscPerSec,param[i].xAmp,param[i].xPhase);
            sprites[i].y += getOscOffset(inFrameTime,param[i].offset.y,param[i].yOscPerSec,param[i].yAmp,param[i].yPhase);

            var rock = inAnimFade * getOscOffset(inFrameTime,0,param[i].rockOscPerSec,param[i].rockAmp,param[i].rockPhase);
            var rotCenterOffset = mult(param[i].rotationCenterOffset,inAnimFade);

            var targetWeight = 1 - inAnimFade;
            var sinValB = getOscOffset(targetSpriteFrameTime,0,param[i].fadeOscPerSec,1,param[i].fadePhase + 0.25);
            var hardVersionB:Float = 0;
            var hardnessB:Float = param[i].fadeHardness;
            if (hardnessB == 1)
            {
                if (sinValB > 0)
                {
                    hardVersionB = 1;
                }else{
                    hardVersionB = -1;
                }
            }else{
                var absSinValB = Math.abs(sinValB);
                if (absSinValB != 0)
                {
                    hardVersionB = (sinValB/absSinValB) * Math.pow(absSinValB,1/(hardnessB * 10 + 1));
                }else{
                    hardVersionB = 0;
                }
            }
            var fadeB = (param[i].fadeMax - param[i].fadeMin) * (0.5 * hardVersionB + 0.5) + param[i].fadeMin;
            if (hardnessB == 1)
            {
                if (targetWeight > 0.5)
                {
                    sprites[i].alpha = fadeB;
                }
            }else{
                sprites[i].alpha += targetWeight * fadeB;
            }

            sprites[i].x += targetWeight * getOscOffset(targetSpriteFrameTime,param[i].offset.x,param[i].yOscPerSec,param[i].xAmp,param[i].xPhase);
            sprites[i].y += targetWeight * getOscOffset(targetSpriteFrameTime,param[i].offset.y,param[i].yOscPerSec,param[i].yAmp,param[i].yPhase);

            rock += targetWeight * getOscOffset(targetSpriteFrameTime,0,param[i].rockOscPerSec,param[i].rockAmp,param[i].rockPhase);
            rotCenterOffset = add(rotCenterOffset,mult(param[i].rotationCenterOffset,targetWeight));
            var totalRotOffset:Float = param[i].rotPerSec * spriteFrameTime + param[i].rotPhase;
            /* FROZEN STUFF
            if (type != moving && param[i].rotPerSec == 0 && param[i].rotPhase == 0 && param[i].rockOscPerSec == 0 && param[i].rockPhase == 0)
            {
                //use frozen instead

            }else if (inAnimFade < 1 && type == moving && )
            {

            }*/

            var releativeRotOffset:Float = totalRotOffset - floor(totalRotOffset);
            //make postive
            if (releativeRotOffset < 0)
            {
                releativeRotOffset ++;
            }
            
        }
    }
    private function add(a:Point,b:Point):Point
    {
        return new Point(a.x + b.x,a.y + b.y);
    }
    private function mult(a:Point,b:Float):Point
    {
        return new Point(a.x * b,a.y * b);
    }
    private function processFrameTimeWithPauses()
    {

    }
    private function getOscOffset(inFrameTime:Float,inOffset:Float,inOscPerSec:Float,inAmp:Float,inPhase:Float):Float
    {
        return inOffset + inAmp * Math.sin((inFrameTime * inOscPerSec + inPhase) * 2 * Math.PI);
    }
}