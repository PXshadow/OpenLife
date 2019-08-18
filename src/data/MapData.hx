package data;
import haxe.ds.Either;
import haxe.ds.Vector;
#if sys
import sys.io.File;
#end
import haxe.io.Bytes;
import game.Player;
class MapData
{
    //container links index to objects array data when negative number
    public var containers:Array<Vector<Int>> = [];
    //biome 0-7
    public var biome:ArrayData<Int> = new ArrayData<Int>();
    //floor objects
    public var floor:ArrayData<Int> = new ArrayData<Int>();
    //object is a postive number, container is a negative that maps 
    public var object:ArrayData<Int> = new ArrayData<Int>();

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
    }
}
class ArrayData<T>
{
    var array:Array<Array<T>> = [];
    //diffrence
    public var dx:Int = 0;
    public var dy:Int = 0;
    var lx:Int = 0;
    public function new()
    {
        array[0] = [];
    }
    public function row(y:Int):Array<T>
    {
        return array[y-dy];
    }
    public function lengthY():Int
    {
        return array.length;
    }
    public function lengthX():Int
    {
        return lx;
    }
    public function get(x:Int,y:Int):T
    {
        if (array[y - dy] != null)
        {
            return array[y - dy][x - dx];
        }
        return null;
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
    public function shiftX(x:Int,value:T)
    {
        if (x < dx)
        {
            //trace("x shift " + Std.string(dx - x) + " array " + array.length);
            for (j in 0...array.length)
            {
                if (array[j] == null) array[j] = [];
                for (i in 0...dx - x) 
            	{
                    array[j].unshift(null);
                }
            }
            dx = x;
        }
    }
    public function set(x:Int,y:Int,value:T)
    {
        shiftY(y);
        shiftX(x,value);
        //null array fill
        if (array[y - dy] == null)
        {
            array[y - dy] = [];
        }
        //set value
        x += -dx;
        //set lengthX
        if (lx < x) lx = x;
        array[y - dy][x] = value;
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
    public var speed:Float = 0;
    public function new(array:Array<String>)
    {
        x = Std.parseInt(array[0]);
        y = Std.parseInt(array[1]);
        floor = Std.parseInt(array[2]);
        id = Std.parseInt(array[3]);
        pid = Std.parseInt(array[4]);
        //optional speed
        if(array.length > 5)
        {
            oldX = Std.parseInt(array[5]);
            oldY = Std.parseInt(array[6]);
            speed = Std.parseFloat(array[7]);
        }
    }
}