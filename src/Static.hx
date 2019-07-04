import haxe.ds.Vector;
import openfl.Lib;
import openfl.display.Bitmap;
import haxe.io.Input;
import openfl.display.BitmapData;
import openfl.geom.Rectangle;
import openfl.utils.ByteArray;
#if sys
import sys.io.FileInput;
import sys.FileSystem;
import sys.io.File;
#end
import haxe.Http;
class Static 
{
    public static inline var GRID:Int = 128;
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
}