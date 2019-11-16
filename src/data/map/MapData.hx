package data.map;
import haxe.ds.Vector;
class MapData
{
    //container links index to objects array data when negative number
    public var containers:Array<Vector<Int>> = [];
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
    /**
     * Loaded boolean
     */
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
    /**
     * Set map chunk
     * @param x Tile X
     * @param y Tile Y 
     * @param width Tile Width
     * @param height Tile Height
     * @param string Data string buffer
     */
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
    /**
     * Generate Array container format from string buffer
     * @param string buffer data
     * @return Array<Int> Container format array
     */
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