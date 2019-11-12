package data;
#if openfl
import openfl.media.Sound;
import openfl.media.SoundChannel;
import openfl.media.SoundTransform;
class SoundPlayer extends Player
{
    public static var soundVolume:Float = 1;
    public static var musicVolume:Float = 1;
    //aiff -> ogg
    var channel:SoundChannel;
    var sound:Sound;
    var tryPlay:Bool = false;
    var type:Int = 0;
    var multi:Float = 0;
    public function new(string:String)
    {
        var array = string.split(":");
        var i:Int = Std.parseInt(array[0]);
        if (i == -1) return;
        multi = Std.parseFloat(array[1]);
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
        channel.soundTransform.volume = (type == 0 ? soundVolume : musicVolume) * multi;
    }
    public function stop()
    {
        if (channel == null) return;
        channel.stop();
        channel = null;
    }
    #end