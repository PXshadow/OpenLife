package data;
import haxe.ds.Vector;
#if sys
import sys.io.File;
#end
import haxe.io.Bytes;
import states.game.Player;
class MapData
{
    //container links index to objects array data when negative number
    public var containers:Array<Vector<Int>> = [];
    //biome 0-7
    public var biome:ArrayData = new ArrayData();
    //floor objects
    public var floor:ArrayData = new ArrayData();
    //object is a postive number, container is a negative that maps 
    public var object:ArrayData = new ArrayData();

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
        for(j in y...y + height)
        {
            for (i in x...x + width)
            {
                string = a.shift();
                k = string.lastIndexOf(":");
                data = string.substring(0,k).split(":");
                biome.set(i,j,Std.parseInt(data[0]));
                floor.set(i,j,Std.parseInt(data[1]));
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
                    object.set(i,j,Std.parseInt(string));
                }
            }
        }
        trace("dx " + object.dx + " lx " + object.lengthX());
    }
}
class ArrayData
{
    var array:Array<Array<Int>> = [];
    //diffrence
    public var dx:Int = 0;
    public var dy:Int = 0;
    public function new()
    {
        array[0] = [];
    }
    public function lengthY():Int
    {
        return array.length;
    }
    public function lengthX():Int
    {
        return array[0].length;
    }
    public function get(x:Int,y:Int):Int
    {
        if (array[y - dy] != null)
        {
            return array[y - dy][x - dx];
        }
        return 0;
    }
    public function shiftY(y:Int)
    {
        //shift
        if (y < dy) 
        {
            for(i in 0...dy - y) array.unshift([]);
            dy = y;
        }
    }
    public function shiftX(x:Int)
    {
        if (x < dx)
        {
            trace("x shift " + Std.string(dx - x) + " array " + array.length);
            for (j in 0...array.length)
            {
                if (array[j] == null) array[j] = [];
                for (i in 0...dx - x) 
            	{
                    //causes crash sometimes (figure out why)
                	array[j].unshift(0);
                }
            }
            dx = x;
        }
    }
    public function set(x:Int,y:Int,value:Int)
    {
        shiftY(y);
        shiftX(x);
        //null array fill
        if (array[y - dy] == null)
        {
            array[y - dy] = [];
        }
        //set value
        array[y - dy][x - dx] = value;
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