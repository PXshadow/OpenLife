package resources;

import haxe.io.Bytes;
import data.object.ObjectData;
import data.object.SpriteData;
import data.transition.TransitionData;

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
        return getContent("animations",'${id}_${i}');
    }
    public static function ground(id:Int,i:Int,j:Int,a:String):Bytes
    {
        return getImage("groundTileCache",'biome_${id}_x${i}_y$j$a.tga');
    }
    public static function groundOverlay(id:Int):Bytes
    {
        return getImage("graphics",'ground_t$id');
    }
    public static function getImage(path:String,name:String):Bytes
    {
        #if sys
        return sys.io.File.getBytes('${game.Game.dir}/$name.tga');
        #else
        return Bytes.alloc(0);
        #end
    }
    public static function getContent(path:String,name:String):String
    {
        #if sys
        return sys.io.File.getContent('${game.Game.dir}/$name.txt');
        #else
        return "";
        #end
    }
}