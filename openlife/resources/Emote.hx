package openlife.resources;

import haxe.ds.Vector;
import openlife.data.animation.emote.EmoteData;
@:expose("Emote")
class Emote
{
    /**
     * Visual generate emote data
     */
     public static function get(settings:openlife.settings.Settings):Vector<EmoteData>
        {
            if (!settings.data.exists("emotionObjects") || settings.data.exists("emotionWords"))
            {
                trace("no emote data in settings");
                return null;
            }
            var arrayObj:Array<String> = settings.data.get("emotionObjects").split("\n");
            var arrayWord:Array<String> = settings.data.get("emotionWords").split("\n");
            var emotes = new Vector<EmoteData>(arrayObj.length);
            for (i in 0...arrayObj.length) emotes[i] = new openlife.data.animation.emote.EmoteData(arrayWord[i],arrayObj[i]);
            return emotes;
        }
}