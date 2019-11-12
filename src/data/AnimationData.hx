package data;
#if openfl
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
        //fail = true;
        //return;
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
            line = readLines(File.getContent(Static.dir + "animations/" + id + "_" + i + ".txt"));
            if (line.length > 0) record[i] = process();
        }
        line = null;
    }
    public function process():AnimationRecord
    {
        //id
        var animation = new AnimationRecord();
        animation.id = getInt();
        //type
        var string = getString();
        var cut:Int = string.indexOf(",");
        var sep = string.indexOf(":");
        if (animation.type == moving) trace("moving process");
        if (sep == -1)
        {
            sep = cut;
        }else{
            //extra index read
        }
        switch(Std.parseInt(string.substring(0,sep)))
        {
            case 0: animation.type = ground;
            case 1: animation.type = held;
            case 2: animation.type = moving;
            case 3: animation.type = eating;
            case 4: animation.type = doing;
            case 5: animation.type = endAnimType;
        }
        //rand start phase
        animation.randStartPhase = Std.parseFloat(string.substring(cut + 1,string.length));

        if (readName("forceZeroStart"))
        {
            next++;
        }
        //next++;
        //num
        if (readName("numSounds"))
        {
            animation.numSounds = getInt();
            //skip over sounds
            if (animation.numSounds > 0) for(i in 0...animation.numSounds) getString();
        }
        animation.numSprites = getInt();
        animation.numSlots = getInt();
        //Params
        if(animation.numSprites <= 0) return animation;
        animation.params = new Vector<AnimationParameter>(animation.numSprites);
        for(i in 0...animation.params.length) animation.params[i] = processParam();
        return animation;
    }
    public function processParam():AnimationParameter
    {
        var param:AnimationParameter = new AnimationParameter();
        if(readName("offset"))
        {
            var s:String = getString();
            var cut = s.indexOf(",");
            param.offset = new Point(Std.parseFloat(s.substring(1,cut)),Std.parseFloat(s.substring(cut + 1,s.length - 1)));
        }
        if(readName("startPause"))
        {
            param.startPauseSec = getFloat();
        }
        //animation param
        var animParam = getString();
        param.process(animParam.split(" "));
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
    public var forceZeroStart:Float = 0;
    public function new()
    {

    }
    public function toString():String
    {
        var string:String = "";
        for(field in Reflect.fields(this))
        {
            string += field + ": " + Reflect.getProperty(this,field) + "\n";
        }
        return string;
    }
}
class AnimationParameter
{
    //https://github.com/twohoursonelife/OneLifeDocs/blob/master/OHOL%20editor%20guide.txt
    public var offset:Point;
    // the first pause, before the first animation duration,
    // for controling the phase of the duration-pause cycle
    public var startPauseSec:Float = 0;
    //param array start here, amount of seconds to complete cycle
    public var xOscPerSec:Float = 0;
    //in pixels distance from middle
    public var xAmp:Float = 0;
    // between 0 and 1 of where to start the animation 0 = middle-right, 0.25 = right-left, 0.50, 0.50 middle-left, 0.75 left-right
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
        var i:Int = 0;
        xOscPerSec = Std.parseFloat(array[i++]);
        xAmp = Std.parseFloat(array[i++]);
        xPhase = Std.parseFloat(array[i++]);
        
        yOscPerSec = Std.parseFloat(array[i++]);
        yAmp = Std.parseFloat(array[i++]);
        yPhase = Std.parseFloat(array[i++]);

        var s:String = array[i++];
        if (s == null) return;
        var cut = s.indexOf(",");
        rotationCenterOffset = new Point(Std.parseFloat(s.substring(1,cut)),Std.parseFloat(s.substring(cut + 1,s.length - 1)));

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
#end