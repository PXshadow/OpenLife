package states.game;
import openfl.media.Sound;
class Music
{
    var sound:Sound;
    public function new()
    {
        Sound.loadFromFile("assets/music_01.ogg").onComplete(function(s:Sound)
        {
            sound = s;
        });
    }
}