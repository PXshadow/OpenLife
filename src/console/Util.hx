package console;
import haxe.io.Path;
import sys.FileSystem;
import data.object.ObjectData;
import sys.io.File;
import game.Game;
#if openfl
import openfl.geom.ColorTransform;
import openfl.display.Shader;
#end

class Util
{
    //util for hscript
    public static function object(i:Int)
    {
        Static.execute(Game.dir + "objects/" + i + ".txt");
    }
    #if openfl
    public static function color(name:String):ColorTransform
    {
        var transform = new ColorTransform();
        switch(name.toLowerCase())
        {
            case "red":
            transform.redOffset = 255;
        }
        return transform;
    }
    #end
}