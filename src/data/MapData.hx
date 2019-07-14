package data;
import haxe.ds.Vector;
#if sys
import sys.io.File;
#end
import haxe.io.Bytes;
import states.game.Player;
class MapData
{
    public var biome:Array<Array<Int>> = [];
    public var floor:Array<Array<Int>> = [];
    //container -> container -> obj
    public var object:Array<Array<Vector<Int>>> = [];

    public var setX:Int = 0;
    public var setY:Int = 0;
    public function new()
    {
        
    }
    public function setRect(x:Int,y:Int,width:Int,height:Int,string:String)
    {
        var a:Array<String> = string.split(" ");
        var data:Array<String>;
        var k:Int = 0;
        var index:Int = 0;
        trace("a " + a.length);
        //bottom left
        for(j in y - setY...y - setY + height)
        {
            biome[j] = [];
            floor[j] = [];
            object[j] = [];
            for (i in x - setX...x - setX + width)
            {
                string = a.shift();
                k = string.lastIndexOf(":");
                data = string.substring(0,k).split(":");
                biome[j][i] = Std.parseInt(data[0]);
                floor[j][i] = Std.parseInt(data[1]);
                //final
                string = string.substring(k + 1,string.length);
                if (string.indexOf(",") >= 0)
                {
                    if (string.indexOf(":") >= 0)
                    {
                        //double container
                    }else{
                        //single container
                    }
                }else{
                    object[j][i] = Vector.fromArrayCopy([Std.parseInt(string)]);
                }
            }
        }
        //trace(biome);   
    }
}
class MapInstance
{
    public var x:Int = 0;
    public var y:Int = 0;
    public var sizeX:Int = 0;
    public var sizeY:Int = 0;
    public var rawSize:Int = 0;
    public var compressedSize:Int = 0;
    
    public function new()
    {

    }
    public function toString():String
    {
        return "pos(" + x + "," + y +") size(" + sizeX + "," + sizeY + ") raw: " + rawSize + " compress: " + compressedSize;
    }
}
class MapChange
{
    public var x:Int = 0;
    public var y:Int = 0;
    public var floor:Int = 0;
    public var id:Int = 0;
    public var pid:Int = 0;
    public var oldX:Int = 0;
    public var oldY:Int = 0;
    public var speed:Int = 0;
    public function new(array:Array<String>)
    {
        x = Std.parseInt(array[0]);
        y = Std.parseInt(array[1]);
        floor = Std.parseInt(array[2]);
        id = Std.parseInt(array[3]);
        pid = Std.parseInt(array[4]);
        //optional speed
        if(array.length > 4)
        {
            var old = array[5] + "." + array[6];
            var speed = array[7];
        }
    }
}