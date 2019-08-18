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
    public function new(tilesetWidth:Int,tilesetHeight:Int,transparent:Bool=true)
    {
        super(0,0,new Tileset(new BitmapData(tilesetWidth,tilesetHeight,transparent)),false);
        tileBlendModeEnabled = false;
        tileColorTransformEnabled = true;
    }
    public function getFill():Float
    {
        if (tileset.numRects > 0)
        {
            var rect = tileset.getRect(tileset.numRects - 1);
            var percent:Float = (rect.y + rect.height) / tileset.bitmapData.height;
            return percent;
        }else{
            return 0;
        }
    }
}