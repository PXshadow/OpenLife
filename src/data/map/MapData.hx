package data.map;
import haxe.Timer;
import haxe.ds.Vector;
#if nativeGen @:nativeGen #end
class MapData
{
    /**
     * Biome 2D array, id of ground
     */
    public var biome:ArrayDataInt = new ArrayDataInt();
    /**
     * Floor 2D array, id of floor
     */
    public var floor:ArrayDataInt = new ArrayDataInt();
    /**
     * Object 2D array, container format for object
     */
    public var object:ArrayDataArrayInt = new ArrayDataArrayInt();
    
    //all chunks combined
    public var x:Int = 999999999;
    public var y:Int = 999999999;
    //max pos
    public var mx:Int = -999999999;
    public var my:Int = -999999999;
    public function new()
    {
        
    }
    /**
     * Set map chunk
     * @param x Tile X
     * @param y Tile Y 
     * @param width Tile Width
     * @param height Tile Height
     * @param string Data string buffer
     */
    public function setRect(chunk:MapInstance,string:String)
    {
        //combine
        if (this.x > chunk.x) this.x = chunk.x;
        if (this.y > chunk.y) this.y = chunk.y;
        if (this.mx < chunk.x + chunk.width) this.mx = chunk.x + chunk.width;
        if (this.my < chunk.y + chunk.height) this.my = chunk.y + chunk.height;
        //create array
        var a:Array<String> = string.split(" ");
        //data array for object
        var data:Array<String>;
        var objectArray:Array<Int> = [];
        var array:Array<Array<Int>> = [];
        //bottom left
        for(j in chunk.y...chunk.y + chunk.height)
        {
            for (i in chunk.x...chunk.x + chunk.width)
            {
                string = a.shift();
                data = string.split(":");
                biome.set(i,j,Std.parseInt(data[0]));
                floor.set(i,j,Std.parseInt(data[1]));
                //setup containers
                object.set(i,j,id(data[2]));
            }
        }
    }
    #if sys
    public function mapFile(file:sys.io.FileInput,inOffsetX:Int=0,inOffsetY:Int=0,inTimeLimitSec:Float=0)
    {
        var startTime = Timer.stamp();
        var line:Array<String> = [];
        var x:Int = 0;
        var y:Int = 0;
        // break out when read fails
        // or if time limit passed
        while (inTimeLimitSec == 0 || Timer.stamp() < startTime + inTimeLimitSec)
        {
            try {
                line = file.readLine().split(" ");
            }catch(e:Dynamic)
            {
                trace("No more lines");
                return;
            }
            //loading into
            x = Std.parseInt(line[0]);
            y = Std.parseInt(line[1]);
            biome.set(x,y,Std.parseInt(line[2]));
            floor.set(x,y,Std.parseInt(line[3]));
            object.set(x,y,id(line[4]));
        }
    }
    #end
    public function toString()
    {
        return 'x: $x y: $y maxX: $mx maxY: $my';
    }
    /**
     * Generate Array container format from string buffer
     * @param string buffer data
     * @return Array<Int> Container format array
     */
    public static function id(string:String,first:String=",",second:String=":"):Array<Int>
    {
        //postive is container, negative is subcontainer that goes into postive container
        //0 is first container, untill another postive number comes around
        var a = string.split(first);
        var s:Array<String> = [];
        var array:Array<Int> = [];
        for (i in 0...a.length)
        {
            //container split data
            s = a[i].split(second);
            //sub
            array.push(Std.parseInt(s[0]));
            for (k in 1...s.length - 1)
            {
                //subobjects
                array.push(Std.parseInt(s[k]) * -1);
            }
        }
        return array;
    }
    public static function stringID(a:Array<Int>):String
    {
        var string:String = "";
        for (i in 0...a.length)
        {
            string += i;
            if (i >= 0)
            {
                string += ",";
            }else{
                string += ":";
            }
        }
        return string.substring(0,string.length - 1);
    }
}