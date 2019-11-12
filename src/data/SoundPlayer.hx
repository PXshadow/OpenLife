package data;
import openfl.events.Event;
#if openfl
import openfl.media.Sound;
import openfl.media.SoundChannel;
import openfl.media.SoundTransform;
class SoundPlayer
{
    private var active:Array<SoundChannel> = [];
    public static var soundVolume:Float = 1;
    public static var musicVolume:Float = 1;
    //aiff -> ogg
    public function new()
    {

    }
    public function play(data:SoundData)
    {
        if(data.music)
        {
            Sound.loadFromFile(Static.dir + "music/" + data.id + ".ogg").onComplete(function(sound:Sound)
            {
                var channel = sound.play(0,0,new SoundTransform(musicVolume * data.multi));
                channel.addEventListener(Event.SOUND_COMPLETE,complete);
                active.push(channel);
            });
        }else{
            Sound.loadFromFile(Static.dir + "sounds/" + data.id + ".ogg").onComplete(function(sound:Sound)
            {
                var channel = sound.play(0,0,new SoundTransform(soundVolume * data.multi));
                channel.addEventListener(Event.SOUND_COMPLETE,complete);
                active.push(channel);
            });
        }
    }
    private function complete(e:Event)
    {
        var channel:SoundChannel = cast e.currentTarget;
        channel.removeEventListener(Event.SOUND_COMPLETE,complete);
        active.remove(channel);
    }
    public function stop(data:SoundChannel)
    {
        active.remove(data);
        data.stop();
    }
}
#end