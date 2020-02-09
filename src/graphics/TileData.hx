package graphics;
#if openfl
import openfl.display.Tile;
import data.ArrayDataTile;
class TileData
{
    //floor objects
    public var floor:ArrayDataTile = new ArrayDataTile();
    //object is a postive number, container is a negative that maps 
    public var object:ArrayDataTile = new ArrayDataTile();

    public function new()
    {
        
    }
}
#end