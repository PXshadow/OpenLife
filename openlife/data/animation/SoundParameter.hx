package openlife.data.animation;

import openlife.data.sound.SoundData;
import haxe.ds.Vector;

class SoundParameter
{
    public var sounds:Vector<SoundData>;

    public var repeatPerSec:Float = 0;
    public var repeatPhase:Float = 0;

    public var ageStart:Float;
    public var ageEnd:Float;

    /**
     * default footstep sounds are replaced with floor usage
     */
    var footstep:String = "";
    public function new()
    {

    }
}