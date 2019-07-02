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
    public static var dir:String = "assets/data/";
    public static var fullPath:String = "";
    public static var assetSystem:Bool = false;
    public static var uiSystem:Bool = false;

    public static function getDir()
    {
        //get full path
        fullPath = lime.system.System.applicationDirectory;
        //inside contents/assets
        if(Assets.exists("assets/ui/code.svg"))
        {
            //check ui   
            uiSystem = true;
            trace("ui true");
        }
        //packaged inside objects/ground/sprites
        if(Assets.exists(dir + "ground/ground_0.tga"))
        {
            assetSystem = true;

            trace("true asset data");
            return;
        }
        //outside of app/exec
        #if sys
        dir = fullPath;
        #if mac
        dir = dir.substring(0,dir.indexOf("/Contents/Resources/"));
        dir = dir.substring(0,dir.lastIndexOf("/") + 1);
        #end

        trace("dir " + dir);
        #end
    }
}