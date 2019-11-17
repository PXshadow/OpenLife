package data;
/**
 * 2D generic Array
 */
@:generic
class ArrayDataArrayInt
{
    var array:Array<Array<Array<Int>>> = [];
    /**
     * diffrence x
     */
    public var dx:Int = 0;
    /**
     * diffrence y
     */
    public var dy:Int = 0;
    public function new()
    {
        array[0] = [];
    }
    /**
     * clear Array
     */
    public function clear()
    {
        array = [];
        dx = 0;
        dy = 0;
    }
    /**
     * Get value 2D array
     * @param x 
     * @param y 
     * @return Array<T>
     */
    public function get(x:Int,y:Int):Array<Int>
    {
        if (array[y - dy] != null)
        {
            return array[y - dy][x - dx];
        }
        return [];
    }
    /**
     * shift the array y if negative
     * @param y 
     */
    public function shiftY(y:Int)
    {
        //shift
        if (y < dy) 
        {
            for(i in 0...dy - y) array.unshift([]);
            dy = y;
        }
    }
    /**
     * shift the array x if negative
     * @param x 
     */
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
    /**
     * set property
     * @param x 
     * @param y 
     * @param value set into 2D Array
     */
    public function set(x:Int,y:Int,value:Array<Int>)
    {
        shiftY(y);
        shiftX(x);
        //set value
        if (array[y - dy] == null) array[y - dy] = [];
        array[y - dy][x - dx] = value;
    }
}