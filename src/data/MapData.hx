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

    public var loaded:Bool = false;

    //all chunks combined
    public var x:Int = 0;
    public var y:Int = 0;
    public var width:Int = 0;
    public var height:Int = 0;
    public function new()
    {
        
    }
    public function setRect(x:Int,y:Int,width:Int,height:Int,string:String)
    {
        //loaded in data
        loaded = true;
        //create array
        var a:Array<String> = string.split(" ");
        //data array for object
        var data:Array<String>;
        var k:Int = 0;
        //bottom left
        for(j in y - this.y...y - this.y + height)
        {
            biome[j] = [];
            floor[j] = [];
            object[j] = [];
            for (i in x - this.x...x - this.x + width)
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
                        trace("double container");
                    }else{
                        //single container
                        trace("single container");
                    }
                }else{
                    object[j][i] = Vector.fromArrayCopy([Std.parseInt(string)]);
                }
            }
        }
        //trace(object);
    }
}
class MapInstance
{
    //current chunk
    public var x:Int = 0;
    public var y:Int = 0;
    public var width:Int = 0;
    public var height:Int = 0;
    public var rawSize:Int = 0;
    public var compressedSize:Int = 0;
    
    public function new()
    {

    }
    public function toString():String
    {
        return "pos(" + x + "," + y +") size(" + width + "," + height + ") raw: " + rawSize + " compress: " + compressedSize;
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