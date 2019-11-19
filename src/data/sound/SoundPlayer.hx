package data.sound;
import openfl.events.Event;
#if openfl
import openfl.media.Sound;
import openfl.media.SoundChannel;
import openfl.media.SoundTransform;
class SoundPlayer
{
    /**
     * active list of sounds running
     */
    private var active:Array<SoundChannel> = [];
    /**
     * static sound volume
     */
    public static var soundVolume:Float = 1;
    /**
     * static music volume
     */
    public static var musicVolume:Float = 1;
    //aiff -> ogg
    public function new()
    {

    }
    /**
     * play sound by SoundData
     * @param data 
     */
    public function play(data:SoundData,start:Float=0,repeat:Int=0):SoundChannel
    {
        var channel:SoundChannel = null;
        if(data.music)
        {
            channel = Sound.fromFile(Static.dir + "music/" + data.id + ".ogg").play(start,repeat,new SoundTransform(musicVolume * data.multi));
            channel.addEventListener(Event.SOUND_COMPLETE,complete);
            active.push(channel);
        }else{
            channel = Sound.fromFile(Static.dir + "sounds/" + data.id + ".ogg").play(0,0,new SoundTransform(soundVolume * data.multi));
            channel.addEventListener(Event.SOUND_COMPLETE,complete);
            active.push(channel);
        }
        return channel;
    }
    private function complete(e:Event)
    {
        var channel:SoundChannel = cast e.currentTarget;
        channel.removeEventListener(Event.SOUND_COMPLETE,complete);
        active.remove(channel);
    }
    /**
     * Stop the sound by refrencing SoundChannel
     * @param data 
     */
    public function stop(data:SoundChannel)
    {
        active.remove(data);
        data.stop();
    }
}
#end