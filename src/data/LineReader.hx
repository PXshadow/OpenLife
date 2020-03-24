package data;
//multi platform to read input
import haxe.io.Input;
import haxe.ds.Vector;
#if nativeGen @:nativeGen #end
class LineReader
{
    @:doxHide(false)
    /**
     * line vector of data
     */
    var line:Vector<String>;
    @:doxHide(false)
    /**
     * next 
     */
    var next:Int = 0;
    @:doxHide(false)
    var input:Input;
    public function new()
    {

    }
    /**
     * Read lines and put into line vector
     * @param string text split into lines
     */
    public function readLines(string:String)
    {
        next = 0;
        line = Vector.fromArrayCopy(string.split("\n"));
    }
    /**
     * Float Array value from line
     * @return Array<Float>
     */
    public function getFloatArray():Array<Float>
    {
        var array:Array<Float> = [];
        for(o in getStringArray())
        {
            array.push(Std.parseFloat(o));
        }
        return array;
    }
    /**
     * Int Array value from line
     * @return Array<Int>
     */
    public function getIntArray():Array<Int>
    {
        var array:Array<Int> = [];
        for(o in getStringArray())
        {
            array.push(Std.parseInt(o));
        }
        return array;
    }
    /**
     * String Array value from line
     * @return Array<String>
     */
    public function getStringArray():Array<String>
    {
        return getString().split(",");
    }
    /**
     * Point (x,y) from line
     * @return Point
     */
    public function getPoint():Point
    {
        var string = getString();
        var comma:Int = string.indexOf(",");
        return new Point(Std.parseInt(string.substring(0,comma)),Std.parseInt(string.substring(comma + 1,string.length)));
    }
    /**
     * Boolean value from line
     * @return Bool
     */
    public function getBool():Bool
    {
        return getString() == "1" ? true : false;
    }
    /**
     * Int from string
     * @return Int
     */
    public function getInt():Int
    {
        return Std.parseInt(getString());
    }
    /**
     * Multi property array int value from line
     * @return Array<Int>
     */
    public function getArrayInt():Array<Int>
    {
        var array:Array<Int> = [];
        var string = line[next++];
        var i:Int = 0;
        var j:Int = 0;
        var bool:Bool = true;
        while(bool)
        {
            i = string.indexOf("=",i + 1) + 1;
            j = string.indexOf(",",i);
            j = j < 0 ? string.length : j;
            array.push(Std.parseInt(string.substring(i,j)));
            if(j == string.length) bool = false;
        }
        return array;
    }
    /**
     * Float value from line
     * @return Float
     */
    public function getFloat():Float
    {
        return Std.parseFloat(getString());
    }
    /**
     * String value from line
     * @return String
     */
    public function getString():String
    {
        if (next + 1 > line.length)
        {
            throw("max " + line);
        }
        var string = line[next++];
        if(string == null || string == "") return "";
        var equals = string.indexOf("=");
        return string.substr(equals + 1);
    }
    /**
     * Name from line
     * @param name 
     * @return Bool
     */
    public function readName(name:String):Bool
    {
        var string = line[next];
        if(name == string.substring(0,name.length)) return true;
        return false;
    }
    public function getName():String
    {
        var string = line[next];
        return string.substring(0,string.indexOf("="));
    }
}