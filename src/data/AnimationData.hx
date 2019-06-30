package data;
import sys.io.File;
import openfl.geom.Point;
import haxe.ds.Vector;
import sys.FileSystem;
class AnimationData extends LineReader
{
    public var id:Int = -1;
    public var type:AnimationType;
    public var params:Vector<AnimationParameter>;
    //map object to animation
    //public var map:Map<Float,Animation>;
    public var fail:Bool = false;
    public function new(id:Float)
    {
        super();
        for( i in 0...5 + 1) 
        {
            var path = Static.dir + "animations/" + id + "_" + i + ".txt";
            if (!FileSystem.exists(path)) 
            {
                //trace("ANIMATION FAIL " + id + "_" + i);
                fail = true;
                return;
            }
            line = readLines(File.read(path,false));
            //process();
        }
        line = null;
    }
    public function process()
    {
        //id
        id = getInt();
        //type
        switch(getInt())
        {
            case 0: type = ground;
            case 1: type = held;
            case 2: type = moving;
            case 3: type = eating;
            case 4: type = doing;
            case 5: type = endAnimType;
        }
        //rand start phase
        next++;
        //num sounds
        var numSounds:Int = getInt();
        //numSprites
        var numSprites:Int = getInt();
        //numSlots
        var numSlts:Int = getInt();
        //Params
        //for(i in 0...numSprites) processParam();
    }
    public function processParam()
    {
        if(readName("offset"))
        {
            next++;
        }
        if(readName("startPause"))
        {
            next++;
        }
        if(readName("animParam"))
        {

        }else{
            throw("animParam not found");
        }
    }
}
class AnimationParameter
{
    var offset:Point;
        
    var xOscPerSec:Float = 0;
    // in pixels
    var xAmp:Float = 0;
    // between 0 and 1
    var xPhase:Bool = false;

    var yOscPerSec:Float = 0;
    var yAmp:Float = 0;
    var yPhase:Bool = false;

    var rotationCenterOffset:Point;

    // can be positive (CW) or negative (CCW)
    var rotPerSec:Float = 0;
    var rotPhase:Bool = false;

    var rockOscPerSec:Float = 0;
    // between 0 and 1, where 1 is full rotation before coming back
    var rockAmp:Float = 0;
    var rockPhase:Bool = false;
        
    // for animations that run for a while and pause between runs
    // ticking cogs, twitching animal noses, etc.
    var durationSec:Float = 0;
    var pauseSec:Float = 0;
    // the first pause, before the first animation duration,
    // for controling the phase of the duration-pause cycle
    var startPauseSec:Float = 0;
        

    var fadeOscPerSec:Float = 0;
        
    // 0 is sine wave, 1 is square wave, in between is a mix
    var fadeHardness:Float = 0;

    // what fade oscillates between
    var fadeMin:Float = 0;
    var fadeMax:Float = 0;

    // between 0 and 1
    var fadePhase:Float = 0;
}
enum AnimationType
{
    ground;
    held;
    moving;
    // special case of ground
    // for person who is now holding something
    // animation that only applies to a person as they eat something
    eating;
    doing;
    endAnimType;
}