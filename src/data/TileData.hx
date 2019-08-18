package data;
import openfl.display.Tile;
import data.MapData.ArrayData;
class TileData
{
    //biome 0-7
    public var biome:ArrayData<Int> = new ArrayData<Int>();
    //floor objects
    public var floor:ArrayData<Array<Tile>> = new ArrayData<Array<Tile>>();
    //object is a postive number, container is a negative that maps 
    public var object:ArrayData<Array<Tile>> = new ArrayData<Array<Tile>>();

    public function new()
    {

    }
}