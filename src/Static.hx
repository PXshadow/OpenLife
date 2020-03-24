import haxe.ds.Vector;
#if (sys || nodejs)
import sys.io.FileInput;
import sys.FileSystem;
import sys.io.File;
#end
import haxe.Http;
/**
 * Static functions used across classes
 */
 #if nativeGen @:nativeGen #end
class Static 
{
    public static inline var GRID:Int = 128;
    public static inline var tileHeight:Int = 0;//30;
    public static function main()
    {
        new Main();
    }
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
    public static function arrayEqual(a:Array<Dynamic>,b:Array<Dynamic>):Bool
    {
        if (a[0] != b[0]) return false;
        if (a.length == 1 && b.length == 1) return true;
        if (a.length != b.length) return false;
        for (i in 1...a.length) if (a[i] != b[i]) return false;
        return true;
    }
}