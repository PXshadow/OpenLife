package data;
import openfl.media.SoundChannel;
import openfl.media.SoundTransform;
#if openfl
import openfl.media.Sound;

class SoundData
{
    public static var soundVolume:Float = 1;
    public static var musicVolume:Float = 1;
    //aiff -> ogg
    var channel:SoundChannel;
    var sound:Sound;
    var tryPlay:Bool = false;
    var type:Int = 0;
    public function new(i:Int,type:Int=0)
    {
        this.type = type;
        var path:String = type == 0 ? "sounds" : "music";
        Sound.loadFromFile(Static.dir + "sounds/" + i + ".ogg").onComplete(function(sound:Sound)
        {
            this.sound = sound;
            //if tried to play during load, now play
            if (tryPlay) play();
        });
    }
    public function play()
    {
        if (sound == null) 
        {
            tryPlay = true;
            return;
        }
        channel = sound.play();
        channel.soundTransform.volume = type == 0 ? soundVolume : musicVolume;
    }
    public function stop()
    {
        if (channel == null) return;
        channel.stop();
        channel = null;
    }
}
#end