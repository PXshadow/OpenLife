package data;
import openfl.display.Tile;
import data.MapData.ArrayDataArray;
class TileData
{
    //floor objects
    public var floor:ArrayDataArray<Tile> = new ArrayDataArray<Tile>();
    //object is a postive number, container is a negative that maps 
    public var object:ArrayDataArray<Tile> = new ArrayDataArray<Tile>();

    public function new()
    {

    }
}