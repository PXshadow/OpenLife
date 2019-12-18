package data.animation;

class AnimationParameter
{
    //https://github.com/twohoursonelife/OneLifeDocs/blob/master/OHOL%20editor%20guide.txt
    /**
     * Offset of sprite
     */
    public var offset:Point;
    /**
     * the first pause, before the first animation duration,
     * for controling the phase of the duration-pause cycle
     */
    public var startPauseSec:Float = 0;
    /**
     * param array start here, amount of seconds to complete cycle
     */
    public var xOscPerSec:Float = 0;
    /**
     * in pixels distance from middle
     */
    public var xAmp:Float = 0;
    /**
     * between 0 and 1 of where to start the animation 0 = middle-right, 0.25 = right-left, 0.50, 0.50 middle-left, 0.75 left-right
     */
    public var xPhase:Float = 0;


    public var yOscPerSec:Float = 0;
    public var yAmp:Float = 0;
    public var yPhase:Float = 0;

    public var rotationCenterOffset:Point;
    // can be positive (CW) or negative (CCW)
    public var rotPerSec:Float = 0;
    public var rotPhase:Float = 0;

    public var rockOscPerSec:Float = 0;
    /**
     * between 0 and 1, where 1 is full rotation before coming back
     */
    public var rockAmp:Float = 0;
    public var rockPhase:Float = 0;
    /**
     * for animations that run for a while and pause between runs
     * between 0 and 1, where 1 is full rotation before coming back
     */
    public var durationSec:Float = 0;
    public var pauseSec:Float = 0;

    public var fadeOscPerSec:Float = 0; 
    /**
     * 0 is sine wave, 1 is square wave, in between is a mix
     */
    public var fadeHardness:Float = 0;
    /**
     * what fade oscillates between max
     */
    public var fadeMin:Float = 0;
    /**
     * what fade oscillates between min
     */
    public var fadeMax:Float = 0;

    /**
     * between 0 and 1
     */
    public var fadePhase:Float = 0;
    public function new()
    {
        
    }
    /**
     * Process parameter
     * @param array properties
     */
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