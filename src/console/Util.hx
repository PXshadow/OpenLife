package console;
import haxe.io.Path;
import sys.FileSystem;
import data.object.ObjectData;
import sys.io.File;
#if openfl
import openfl.geom.ColorTransform;
import openfl.display.Shader;
#end

class Util
{
    //util for hscript
    public static function object(i:Int)
    {
        Static.execute(Static.dir + "objects/" + i + ".txt");
    }
    #if openfl
    public static function shader(name:String):Shader
    {
        return switch(name.toLowerCase())
        {
            //case "pixel": new shaders.Pixelated();
            case "dot": new shaders.DotScreen();
            case "film": new shaders.FilmShader();
            //case "gray" | "grey": new shaders.GrayScale();
            case "hex": new shaders.Hexagonate(0.08);
            //case "hue" | "saturate": new shaders.HueSaturationShader();
            //case "invert": new shaders.Invert();
            //case "deut": new shaders.Deuteranopia();
            //case "tech": new shaders.Technicolor();
            default: null;
        }
    }
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