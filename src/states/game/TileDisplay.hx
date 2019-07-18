package states.game;

import sys.io.File;
import openfl.geom.Rectangle;
import data.TgaData;
import openfl.utils.ByteArray;
import haxe.io.Input;
import openfl.display.BitmapData;
import openfl.display.Tileset;
import openfl.display.Tilemap;
import states.launcher.Launcher;
import openfl.display.Tile;

class TileDisplay extends Tilemap
{
    var reader:TgaData;
    //pos
    var tileWidth:Int = 0;
    var tileHeight:Float = 0;
    public function new(tilesetWidth:Int,tilesetHeight:Int,transparent:Bool=true)
    {
        super(0,0,new Tileset(new BitmapData(tilesetWidth,tilesetHeight,transparent)),false);
        tileBlendModeEnabled = false;
        tileColorTransformEnabled = true;
        reader = new TgaData();
        //pos code
        x = 0;
        y = 0;
    }
    public function size(width:Int,height:Int) 
    {
        tileWidth = width;
        tileHeight = height;
        this.width = tileWidth * Static.GRID;
        this.height = tileHeight * Static.GRID;
    }
    public function getFill():Float
    {
        var rect = tileset.getRect(tileset.numRects - 1);
        var percent:Float = (rect.y + rect.height) / tileset.bitmapData.height;
        trace("percent of fill " + percent);
        return percent;
    }
}