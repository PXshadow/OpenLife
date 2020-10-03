package openlife.data.animation.emote;
@:expose
class EmoteData
{
    /**
     * Word to trigger the emote
     */
    public var triggerWord:String = "";
    /**
     * Eye index
     */
    public var eyeEmot:Int = 0;
    /**
     * Mouth index
     */
    public var mouthEmot:Int = 0;
    /**
     * Other index
     */
    public var otherEmot:Int = 0;
    /**
     * Face index
     */
    public var faceEmot:Int = 0;
    /**
     * Body index
     */
    public var bodyEmot:Int = 0;
    /**
     * Head index
     */
    public var headEmot:Int = 0;
    /**
     * Create new emote
     * @param word trigger
     * @param string data buffer
     */
    public function new(word:String,string:String)
    {
        triggerWord = word;
        var array = string.split(" ");
        eyeEmot = Std.parseInt(array[0]);
        mouthEmot = Std.parseInt(array[1]);
        otherEmot = Std.parseInt(array[2]);
        faceEmot = Std.parseInt(array[3]);
        bodyEmot = Std.parseInt(array[4]);
        headEmot = Std.parseInt(array[5]);
    }
} 