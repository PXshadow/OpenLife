package states.game;
import openfl.display.Tile;
class Object
{
    public var children:Array<Tile> = [];
    @:isVar public var alpha(default,set):Float = 0;
    function set_alpha(value:Float):Float
    {
        alpha = value;
        for(child in children)
        {
            child.alpha = alpha;
        }
        return alpha;
    }
    function get_alpha():Float
    {
        return alpha;
    }
    @:isVar public var x(default,set):Float = 0;
    function set_x(value:Float):Float
    {
        var change = value - x;
        for(child in children)
        {
            child.x += change;
        }
        return x = value;
    }
    @:isVar public var y(default,set):Float = 0;
    function set_y(value:Float):Float
    {
        var change = value - y;
        for (child in children)
        {
            child.y += change;
        }
        return y = value;
    }
    public function new()
    {
        
    }
    public function add(child:Tile):Int
    {
        return children.push(child);
    }
    public function remove(child:Tile)
    {
        children.remove(child);
    }
}