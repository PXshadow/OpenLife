package data;

class EmoteData
{
    public var triggerWord:String = "";
    public var eyeEmot:Int = 0;
    public var mouthEmot:Int = 0;
    public var otherEmot:Int = 0;
    public var faceEmot:Int = 0;
    public var bodyEmot:Int = 0;
    public var headEmot:Int = 0;
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