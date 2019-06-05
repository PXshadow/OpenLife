package;

class Data
{
    public static function removeInstanceString(string:String,sub:String):String
    {
        return StringTools.replace(string,sub,"");
    }
}