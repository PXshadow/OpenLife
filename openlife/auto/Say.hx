package openlife.auto;
@:expose("Say")
class Say
{
    public static function has(text:String,sub:String):Bool
    {
        return text.indexOf(sub) > -1;
    }
    public static function any(text:String,list:Array<String>):Bool
    {
        for (sub in list)
        {
            if (has(text,sub)) return true;
        }
        return false;
    }
}