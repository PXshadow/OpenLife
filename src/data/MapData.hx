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
    public var biome:ArrayDataInt = new ArrayDataInt();
    //floor objects
    public var floor:ArrayDataInt = new ArrayDataInt();
    //object is a postive number, container is a negative that maps 
    public var object:ArrayDataArray<Int> = new ArrayDataArray<Int>();

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
        //bottom left
        for(j in y...y + height)
        {
            for (i in x...x + width)
            {
                string = a.shift();
                data = string.split(":");
                biome.set(i,j,Std.parseInt(data[0]));
                floor.set(i,j,Std.parseInt(data[1]));
                //setup containers
                object.set(i,j,[Std.parseInt(data[2])]);
            }
        }
    }
}
class ArrayDataInt
{
    var array:Array<Array<Int>> = [];
    //diffrence
    public var dx:Int = 0;
    public var dy:Int = 0;
    public function new()
    {
        array[0] = [];
    }
    public function clear()
    {
        array = [];
        dx = 0;
        dy = 0;
    }
    public function row(y:Int):Array<Int>
    {
        return array[y-dy];
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
    public function shiftX(x:Int,value:Int)
    {
        if (x < dx)
        {
            for (j in 0...array.length)
            {
                if (array[j] == null) array[j] = [];
                for (i in 0...dx - x) 
            	{
                    array[j].unshift(0);
                }
            }
            dx = x;
        }
    }
    public function set(x:Int,y:Int,value:Int)
    {
        shiftY(y);
        shiftX(x,value);
        //set value
        if (array[y - dy] == null) array[y - dy] = [];
        array[y - dy][x - dx] = value;
    }
}
class ArrayDataArray<T>
{
    var array:Array<Array<Array<T>>> = [];
    //diffrence
    public var dx:Int = 0;
    public var dy:Int = 0;
    public function new()
    {
        array[0] = [];
    }
    public function clear()
    {
        array = [];
        dx = 0;
        dy = 0;
    }
    public function get(x:Int,y:Int):Array<T>
    {
        if (array[y - dy] != null)
        {
            return array[y - dy][x - dx];
        }
        return [];
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
            for (j in 0...array.length)
            {
                if (array[j] == null) array[j] = [];
                for (i in 0...dx - x) 
            	{
                    array[j].unshift([]);
                }
            }
            dx = x;
        }
    }
    public function set(x:Int,y:Int,value:Array<T>)
    {
        shiftY(y);
        shiftX(x);
        //set value
        if (array[y - dy] == null) array[y - dy] = [];
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
    public var speed:Float = 0;
    public function new(array:Array<String>)
    {
        x = Std.parseInt(array[0]);
        y = Std.parseInt(array[1]);
        floor = Std.parseInt(array[2]);
        //trace("change " + array[3]);
        //array[3].split(",")
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