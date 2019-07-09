package console;
#if openfl
import openfl.display.Shader;
#end

class Util
{
    //util for hscript

    #if openfl
    public static function shader(string:String):Shader
    {
        string = string.toLowerCase();
        return switch(string)
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
    #end
}