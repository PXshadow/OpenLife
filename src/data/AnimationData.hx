package data;
import states.launcher.Launcher;
import sys.io.File;
import openfl.geom.Point;
import haxe.ds.Vector;
import sys.FileSystem;
class AnimationData extends LineReader
{
    public var fail:Bool = false;
    public var record:Vector<AnimationRecord>;
    public function new(id:Float)
    {
        super();
        if (!FileSystem.exists(Launcher.dir + "assets/animations/" + id + "_0.txt"))
        {
            fail = true;
            return;
        }
        record = new Vector<AnimationRecord>(5 + 1);
        /*for( i in 0...5 + 1) 
        {
            //skip 3
            if (i == 3) continue;
            //read lines
            line = readLines(File.read(Launcher.dir + "assets/animations/" + id + "_" + i + ".txt",false));
            record[i] = process();
        }*/
        line = null;
    }
    public function process():AnimationRecord
    {
        //id
        var animation = new AnimationRecord();
        animation.id = getInt();
        //type
        switch(getInt())
        {
            case 0: animation.type = ground;
            case 1: animation.type = held;
            case 2: animation.type = moving;
            case 3: animation.type = eating;
            case 4: animation.type = doing;
            case 5: animation.type = endAnimType;
        }
        //rand start phase
        trace("i " + getFloat());
        //next++;
        //num
        animation.numSounds = getInt();
        animation.numSprites = getInt();
        animation.numSlots = getInt();
        //Params
        //for(i in 0...numSprites) processParam();
        return animation;
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
class AnimationRecord
{
    public var id:Int = -1;
    public var type:AnimationType;
    public var params:Vector<AnimationParameter>;
    public var numSounds:Int = 0;
    public var numSprites:Int = 0;
    public var numSlots:Int = 0;
    public var randStartPhase:Float = 0;
    public function new()
    {

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