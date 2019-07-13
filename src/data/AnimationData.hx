package data;
import sys.io.File;
import haxe.ds.Vector;
import sys.FileSystem;
class AnimationData extends LineReader
{
    public var fail:Bool = false;
    public var record:Vector<AnimationRecord>;
    public function new(id:Int)
    {
        super();
        #if !openfl
        fail = true;
        return;
        #end
        if (!FileSystem.exists(Static.dir + "animations/" + id + "_0.txt"))
        {
            fail = true;
            return;
        }
        record = new Vector<AnimationRecord>(5 + 1);
        for( i in 0...5 + 1) 
        {
            //skip 3
            if (i == 3) continue;
            //read lines
            line = readLines(File.read(Static.dir + "animations/" + id + "_" + i + ".txt",false));
            record[i] = process();
        }
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
        getFloat();
        //next++;
        //num
        animation.numSounds = getInt();
        //skip over sounds
        for(i in 0...animation.numSounds) next++;
        animation.numSprites = getInt();
        animation.numSlots = getInt();
        //Params
        if(animation.numSprites == 0) return animation;
        animation.params = new Vector<AnimationParameter>(animation.numSprites);
        trace("numSprites " + animation.numSprites);
        for(i in 0...animation.params.length) animation.params[i] = processParam();
        return animation;
    }
    public function processParam():AnimationParameter
    {
        var param:AnimationParameter = new AnimationParameter();
        if(readName("offset"))
        {
            trace('offset ' + getString());
        }
        if(readName("startPause"))
        {
            param.startPauseSec = getFloat();
        }
        //animation param
        param.process(getString().split(" "));
        return param;
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
    public var offset:Point;
    // the first pause, before the first animation duration,
    // for controling the phase of the duration-pause cycle
    public var startPauseSec:Float = 0;

    //param array start here
    public var xOscPerSec:Float = 0;
    // in pixels
    public var xAmp:Float = 0;
    // between 0 and 1
    public var xPhase:Float = 0;

    public var yOscPerSec:Float = 0;
    public var yAmp:Float = 0;
    public var yPhase:Float = 0;

    public var rotationCenterOffset:Point;

    // can be positive (CW) or negative (CCW)
    public var rotPerSec:Float = 0;
    public var rotPhase:Float = 0;

    public var rockOscPerSec:Float = 0;
    // between 0 and 1, where 1 is full rotation before coming back
    public var rockAmp:Float = 0;
    public var rockPhase:Float = 0;
        
    // for animations that run for a while and pause between runs
    // ticking cogs, twitching animal noses, etc.
    public var durationSec:Float = 0;
    public var pauseSec:Float = 0;
        

    public var fadeOscPerSec:Float = 0;
        
    // 0 is sine wave, 1 is square wave, in between is a mix
    public var fadeHardness:Float = 0;

    // what fade oscillates between
    public var fadeMin:Float = 0;
    public var fadeMax:Float = 0;

    // between 0 and 1
    public var fadePhase:Float = 0;
    public function new()
    {

    }
    public function process(array:Array<String>)
    {
        trace("array " + array);
        var i:Int = 0;
        xOscPerSec = Std.parseFloat(array[i++]);
        xAmp = Std.parseFloat(array[i++]);
        xPhase = Std.parseFloat(array[i++]);

        yOscPerSec = Std.parseFloat(array[i++]);
        yAmp = Std.parseFloat(array[i++]);
        yPhase = Std.parseFloat(array[i++]);

        trace("rotation offset " + array[i++]);

        rotPerSec = Std.parseFloat(array[i++]);
        rotPhase = Std.parseFloat(array[i++]);

        rockOscPerSec = Std.parseFloat(array[i++]);
        rockAmp = Std.parseFloat(array[i++]);
        rockPhase = Std.parseFloat(array[i++]);

        durationSec = Std.parseFloat(array[i++]);
        pauseSec = Std.parseFloat(array[i++]);

        fadeOscPerSec = Std.parseFloat(array[i++]);
        fadeHardness = Std.parseFloat(array[i++]);
        fadeMin = Std.parseFloat(array[i++]);
        fadeMax = Std.parseFloat(array[i++]);
        fadePhase = Std.parseFloat(array[i++]);
        
    }
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