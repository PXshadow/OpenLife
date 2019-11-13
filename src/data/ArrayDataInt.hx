package data;

class ArrayDataInt
{
    var array:Array<Array<Int>> = [];
    //diffrence
    public var dx:Int = 0;
    public var dy:Int = 0;
    public static inline var NAN:Int = -1;
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
        return NAN;
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
                    array[j].unshift(NAN);
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