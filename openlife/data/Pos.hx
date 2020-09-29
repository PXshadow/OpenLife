package openlife.data;
@:expose("Pos")
class Pos
{
    public var x:Int;
    public var y:Int;
    public function new(x:Int=0,y:Int=0)
    {
        this.x = x;
        this.y = y;
    }
    public function clone():Pos
    {
        return new Pos(x,y);
    }
    public function toString():String
    {
        return '($x,$y)';
    }
}