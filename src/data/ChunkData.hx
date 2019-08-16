package data;

class ChunkData
{
    public var array:Array<Chunk> = [];
    public function new() 
    {

    }
    public function add(x:Int,y:Int,width:Int,height:Int,start:Int):Chunk
    {
        var chunk = new Chunk();
        chunk.x = x;
        chunk.y = y;
        chunk.width = width;
        chunk.height = height;
        chunk.start = start;
        array.push(chunk);
        return chunk;
    }
    public function remove(chunk:Chunk)
    {
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
    //start and end of tiles
    public var start:Int = 0;
    public var end:Int = 0;
    public function new()
    {

    }
}