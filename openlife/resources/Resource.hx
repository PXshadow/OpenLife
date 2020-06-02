package openlife.resources;
import openlife.engine.Engine;

import haxe.io.Bytes;
class Resource
{
    public static function objectData(id:Int):String
    {
        return getContent("objects",'$id');
    }
    public static function spriteData(id:Int):String
    {
        return getContent("sprites",'$id');
    }
    public static function spriteImage(id:Int):Bytes
    {
        return getImage("sprites",'$id');
    }
    public static function animation(id:Int,i:Int):String
    {
        try {
            return getContent("animations",'${id}_${i}');
        }catch(e:Dynamic)
        {
            return "";
        }
    }
    public static function ground(id:Int,i:Int,j:Int,a:String):Bytes
    {
        return getImage("groundTileCache",'biome_${id}_x${i}_y$j$a');
    }
    public static function groundOverlay(id:Int):Bytes
    {
        return getImage("graphics",'ground_t$id');
    }
    public static function getImage(path:String,name:String):Bytes
    {
        #if sys
        return sys.io.File.getBytes('${Engine.dir}/$path/$name.tga');
        #else
        return Bytes.alloc(0);
        #end
    }
    public static function getContent(path:String,name:String):String
    {
        #if sys
        return sys.io.File.getContent('${Engine.dir}/$path/$name.txt');
        #else
        return "";
        #end
    }
}