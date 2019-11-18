package data.animation;

import data.sound.SoundData;

class SoundParameter
{
    public var sound:SoundData;

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