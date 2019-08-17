package data;
import openfl.display.TileContainer;
import openfl.display.Tile;
import haxe.ds.Vector;

class ChunkData
{
    public var array:Array<Chunk> = [];
    public var latest:Chunk = null;
    public function new() 
    {

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
        trace("add chunk " + latest);
        return latest;
    }
    public function remove(chunk:Chunk)
    {
        //clean chunk
        var parent:TileContainer = null;
        for (i in 0...chunk.width * chunk.height)
        {
            if (parent == null)
            {
                parent = chunk.ground.i(i).parent;
            }
            parent.removeTile(chunk.ground.i(i));
            for(floor in chunk.floor.i(i)) parent.removeTile(floor);
            for (object in chunk.object.i(i)) parent.removeTile(object);
        }
        array.remove(chunk);
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