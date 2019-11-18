package data.animation;
#if openfl
import sys.io.File;
import haxe.ds.Vector;
import sys.FileSystem;
class AnimationData extends LineReader
{
    /**
     * If animation failed to load
     */
    public var fail:Bool = false;
    /**
     * Records of animation
     */
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
            readLines(File.getContent(Static.dir + "animations/" + id + "_" + i + ".txt"));
            if (line.length > 0) record[i] = process();
        }
        line = null;
    }
    /**
     * Process animation
     * @return AnimationRecord
     */
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
            if (animation.numSounds > 0) for(i in 0...animation.numSounds) 
            {
                var sound = new SoundParameter();
                //sound.
            }
        }
        animation.numSprites = getInt();
        animation.numSlots = getInt();
        //Params
        if(animation.numSprites <= 0) return animation;
        animation.params = new Vector<AnimationParameter>(animation.numSprites);
        for(i in 0...animation.params.length) animation.params[i] = processParam();
        return animation;
    }
    /**
     * Process the paramaters of record
     * @return AnimationParameter
     */
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
#end