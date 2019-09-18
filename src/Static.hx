import haxe.ds.Vector;
#if sys
import sys.io.FileInput;
import sys.FileSystem;
import sys.io.File;
#end
import haxe.Http;
class Static 
{
    //dir
    public static var dir:String = "";

    public static inline var GRID:Int = 128;
    public static inline var tileHeight:Int = 30;
    //player constants
    public static inline var babyHeadDownFactor:Float = 0.6;
    public static inline var babyBodyDownFactor:Float = 0.75;
    public static inline var oldHeadDownFactor:Float = 0.35;
    public static inline var oldHeadForwardFactor:Float = 2;
    
    public static function request(url:String,complete:String->Void)
    {
        var http = new Http(url);
        http.onData = complete;
        http.onError = function(error:Dynamic)
        {
            trace("error " + error);
        }
        http.request(false);
    }
    public static function execute(url:String)
    {
        switch (Sys.systemName()) 
        {
            case "Linux", "BSD": Sys.command("xdg-open", [url]);
            case "Mac": Sys.command("open", [url]);
            case "Windows": Sys.command("start", [url]);
            default:
        }
    }
    //get object list number
    public static function number():Int
    {
        return Std.parseInt(File.getContent(Static.dir + "objects/nextObjectNumber.txt"));
    }
}