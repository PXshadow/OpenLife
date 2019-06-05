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

    public var update:Void->Void;
    public var setX:Int = 0;
    public var setY:Int = 0;
    public var setWidth:Int = 0;
    public var setHeight:Int = 0;
    public function new()
    {

    }
    public function setRect(x:Int,y:Int,width:Int,height:Int,string:String)
    {
        var a:Array<String> = string.split(" ");
        //trace("a " + a);
        var data:Array<String> = [];
        var index:Int = 0;
        var string:String = "0.0";
        for(j in y...y + height)
        {
            for(i in x...x + width)
            {
                data = a[index++].split(":");
                string = i + "." + j;
                //trace("data " + data);
                //trace("set key: " + string);
                biome.set(string,Std.parseInt(data[0]));
                floor.set(string,Std.parseInt(data[1]));
                object.set(string,data[2]);
            }
        }
        trace("update");
        if(update != null) update();
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
    public var bytes:Bytes;
    public var index:Int = 0;
    
    public function new()
    {

    }
    public function toString():String
    {
        return "pos(" + x + "," + y +") size(" + sizeX + "," + sizeY + ") raw: " + rawSize + " compress: " + compressedSize;
    }
}