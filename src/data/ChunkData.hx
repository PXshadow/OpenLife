package data;
import openfl.display.TileContainer;
import openfl.display.Tile;
import haxe.ds.Vector;

class ChunkData
{
    public var array:Array<Chunk> = [];
    public var latest:Chunk = null;
    public var parent:TileContainer;
    public function new(parent:TileContainer) 
    {
        this.parent = parent;
    }
    public function add(x:Int,y:Int,width:Int,height:Int):Chunk
    {
        latest = new Chunk();
        latest.x = x;
        latest.y = y;
        latest.width = width;
        latest.height = height;
        latest.gen();
        array.push(latest);
        return latest;
    }
    public function remove(chunk:Chunk)
    {
        //clean chunk
        var array:Array<Tile> = [];
        for (i in 0...chunk.width * chunk.height)
        {
            parent.removeTile(chunk.ground.i(i));
            array = chunk.floor.i(i);
            if (array != null) for (floor in array) parent.removeTile(floor);
            array = chunk.object.i(i);
            if (array != null) for (object in array) parent.removeTile(object);
        }
        this.array.remove(chunk);
        chunk = null;
    }
}
class Chunk
{
    public var x:Int = 0;
    public var y:Int = 0;
    public var width:Int = 0;
    public var height:Int = 0;
    //x - y - group
    public var ground:ChunkVector<Tile>;
    public var floor:ChunkVector<Array<Tile>>;
    public var object:ChunkVector<Array<Tile>>;
    public function new()
    {

    }
    public function gen()
    {
        ground = new ChunkVector<Tile>(width,height);
        floor = new ChunkVector<Array<Tile>>(width,height);
        object = new ChunkVector<Array<Tile>>(width,height);
    }
}
class ChunkVector<T>
{
    var width:Int = 0;
    var height:Int = 0;
    var vector:Vector<T>;
    public function new(width:Int,height:Int)
    {
        this.width = width;
        this.height = height;
        vector = new Vector<T>(width * height);
    }
    public function get(x:Int,y:Int):T
    {
        return vector[x + y * width];
    }
    public function set(x:Int,y:Int,value:T):T
    {
        return vector[x + y * width] = value;
    }
    public function i(index:Int):T
    {
        return vector[index];
    }
}