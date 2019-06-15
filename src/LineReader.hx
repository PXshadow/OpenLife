import sys.io.FileInput;
import haxe.ds.Vector;
import openfl.geom.Point;

class LineReader
{
    var line:Vector<String>;
    var next:Int = 0;
    public function new()
    {

    }
    public function readLines(input:FileInput):Vector<String>
    {
        return Vector.fromArrayCopy(input.readAll().toString().split("\n"));
    }
    public function getFloatArray():Array<Float>
    {
        var array:Array<Float> = [];
        for(o in getStringArray())
        {
            array.push(Std.parseFloat(o));
        }
        return array;
    }
    public function getIntArray():Array<Int>
    {
        var array:Array<Int> = [];
        for(o in getStringArray())
        {
            array.push(Std.parseInt(o));
        }
        return array;
    }
    public function getStringArray():Array<String>
    {
        return getString().split(",");
    }
    public function getPoint():Point
    {
        var string = getString();
        var comma:Int = string.indexOf(",");
        return new Point(Std.parseInt(string.substring(0,comma)),Std.parseInt(string.substring(comma + 1,string.length)));
    }
    public function getInt():Int
    {
        return Std.parseInt(getString());
    }
    public function getArrayInt():Array<Int>
    {
        var array:Array<Int> = [];
        var string = line[next++];
        var i:Int = 0;
        var j:Int = 0;
        var bool:Bool = true;
        while(bool)
        {
            i = string.indexOf("=",i + 1);
            j = string.indexOf(",",i);
            j = j < 0 ? string.length : j;
            array.push(Std.parseInt(string.substring(i,j)));
            if(j == string.length) bool = false;
        }
        return array;
    }
    public function getFloat():Float
    {
        return Std.parseFloat(getString());
    }
    public function getString():String
    {
        var string = line[next++];
        var equals = string.indexOf("=");
        return string.substring(equals + 1,line.length);
    }
    public function readName(name:String):Bool
    {
        var string = line[next];
        if(name == string.substring(0,name.length)) return true;
        return false;
    }
}