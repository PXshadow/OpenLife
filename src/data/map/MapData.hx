package data.map;
import haxe.ds.Vector;
import data.ArrayDataArray;
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

    public var valleyOffsetY:Int = 0;
    public var valleyOffsetX:Int = 0;
    public var valleySpacing:Int = 0;
    public var valleyBool:Bool = true;

    public var offsetX:Int = 0;
    public var offsetY:Int = 0;
    public var offsetBoolX:Bool = true;
    public var offsetBoolY:Bool = true;
    
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
        var objectArray:Array<Int> = [];
        var array:Array<Array<Int>> = [];
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
                object.set(i,j,id(data[2]));
            }
        }
    }
    public static function id(string:String):Array<Int>
    {
        //postive is container, negative is subcontainer that goes into postive container
        //0 is first container, untill another postive number comes around
            var a = string.split(",");
            var array:Array<Int> = [];
            for (i in 0...a.length)
            {
                //container
                var s = a[i].split(":");
                array.push(Std.parseInt(s[0]));
                for (k in 1...s.length - 1)
                {
                    //subcontainer
                    array.push(Std.parseInt(s[k]) * -1);
                }
            }
            if (array.length == 1 && array[0] > Main.data.nextObjectNumber)
            {
                var alt = Main.data.objectAlt.get(array[0]);
                if (array != null) return [alt];
            }
            return array;
    }
}