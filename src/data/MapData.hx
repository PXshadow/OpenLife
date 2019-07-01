package data;
import lime.app.Future;
#if sys
import sys.io.File;
#end
import haxe.io.Bytes;
class MapData
{
    //column, row
    /*public var biome:Array<Array<Int>> = [];
    public var floor:Array<Array<Int>> = [];
    public var object:Array<Array<String>> = [];*/

    public var biome = new Map<String,Int>();
    public var floor = new Map<String,Int>();
    public var object = new Map<String,String>();

    public var setX:Int = 0;
    public var setY:Int = 0;
    public var setWidth:Int = 0;
    public var setHeight:Int = 0;
    public function new()
    {
        
    }
    public function setRect(x:Int,y:Int,width:Int,height:Int,string:String,update:(x:Int,y:Int,width:Int,height:Int)->Void)
    {
        //async
        new Future(function()
        {
            #if sys
            //File.write("assets/map.txt",false).writeString(string);
            #end
            var a:Array<String> = string.split(" ");
            //trace("a " + a);
            var data:Array<String> = [];
            var string:String = "0.0";
            var index:Int = 0;
            //trace("y " + y);
            for(j in y...y + height)
            {
                for(i in x...x + width)
                {
                    //index = (y + height) - (j * (y + height)) + i;
                    data = a[index++].split(":");
                    string = i + "." + Std.string(j * -1);
                    //trace("data " + data);
                    //trace("set key: " + string);
                    biome.set(string,Std.parseInt(data[0]));
                    floor.set(string,Std.parseInt(data[1]));
                    object.set(string,data[2]);
                }
            }
        },true).onComplete(function(_)
        {
            update(x,y,width,height);
        });
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