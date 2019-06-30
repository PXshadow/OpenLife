import openfl.Assets;
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
class Static 
{
    public static inline var GRID:Int = 128;
    //player constants
    public static inline var babyHeadDownFactor:Float = 0.6;
    public static inline var babyBodyDownFactor:Float = 0.75;
    public static inline var oldHeadDownFactor:Float = 0.35;
    public static inline var oldHeadForwardFactor:Float = 2;
    //file system
    public static var dir:String = "assets/data";
    public static var assetSystem:Bool = false;
    public static var uiSystem:Bool = false;

    public static function getDir()
    {
        //inside contents/assets
        if(Assets.exists("assets/ui/code.svg"))
        {
            //check ui   
            uiSystem = true;
            trace("ui true");
        }
        //packaged inside objects/ground/sprites
        if(Assets.exists("assets/data/groundTileCache/biome_0_x0_y0_square.tga"))
        {
            assetSystem = true;
            trace("true asset data");
            return;
        }
        //outside of app/exec
        #if sys
        dir = lime.system.System.applicationDirectory;
        #if mac
        dir = dir.substring(0,dir.indexOf("/Contents/Resources/"));
        dir = dir.substring(0,dir.lastIndexOf("/") + 1);
        #end

        trace("dir " + dir);
        #end
    }
    //tga
    public static function tga(bitmap:Bitmap,path:String,x:Int=0,y:Int=0)
    {
        bitmap.bitmapData = tgaBitmapData(path,x,y);
    }
    public static function tgaBitmapData(path:String,x:Int=0,y:Int=0):BitmapData
    {
        var data = tgaBytes(path);
        var bitmapData = new BitmapData(data.header.width,data.header.height);
        bitmapData.setPixels(new Rectangle(x,y,data.header.width,data.header.height),ByteArray.fromBytes(data.bytes));
        return bitmapData;
    }
    public static function tgaBytes(path:String):{bytes:ByteArray,header:format.tga.Data.Header}
    {
        var data:format.tga.Data = null;
        #if sys
        try {
            var r = new format.tga.Reader(File.read(path));
            data = r.read();
        }catch(e:Dynamic)
        {
            return null;
        }
        return {bytes:ByteArray.fromBytes(format.tga.Tools.extract32(data,true)),header:data.header};
        #else
        return {bytes:null,header:null};
        #end
    }
}