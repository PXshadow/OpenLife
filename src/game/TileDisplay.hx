package game;

import sys.io.File;
import openfl.geom.Rectangle;
import data.TgaData;
import openfl.utils.ByteArray;
import haxe.io.Input;
import openfl.display.BitmapData;
import openfl.display.Tileset;
import openfl.display.Tilemap;
import openfl.display.Tile;

class TileDisplay extends Tilemap
{
    var reader:TgaData = new TgaData();
    public function new()
    {
        super(0,0,new Tileset(new BitmapData(4096,4096,true,0)),false);
        tileBlendModeEnabled = false;
        tileColorTransformEnabled = true;
    }
    public function getFill():Float
    {
        if (tileset.numRects > 0)
        {
            var rect = tileset.getRect(tileset.numRects - 1);
            var percent:Float = (rect.y + rect.height) / tileset.bitmapData.height + ((rect.x + rect.width) / tileset.bitmapData.width / tileset.bitmapData.height);
            return percent;
        }else{
            return 0;
        }
    }
}