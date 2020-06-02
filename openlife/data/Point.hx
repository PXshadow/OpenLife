package openlife.data;

class Point
{
    /**
     * Float value x
     */
    public var x:Float = 0;
    /**
     * Float value y
     */
    public var y:Float = 0;
    /**
     * set new point
     * @param x 
     * @param y 
     */
    public function new(x:Float=0,y:Float=0) 
    {
        this.x = x;
        this.y = y;
    }
}