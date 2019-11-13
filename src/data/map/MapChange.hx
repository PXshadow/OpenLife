package data.map;

class MapChange
{
    public var x:Int = 0;
    public var y:Int = 0;
    public var floor:Int = 0;
    public var id:Array<Int> = [];
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
        id = MapData.id(array[3]);
        pid = Std.parseInt(array[4]);
        //optional speed
        if(array.length > 5)
        {
            oldX = Std.parseInt(array[5]);
            oldY = Std.parseInt(array[6]);
            speed = Std.parseFloat(array[7]);
        }
    }
    public function toString():String
    {
        var string:String = "";
        for(field in Reflect.fields(this))
        {
            string += field + ": " + Reflect.getProperty(this,field) + "\n";
        }
        return string;
    }
}