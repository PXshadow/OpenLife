package states.game;

import haxe.io.Bytes;
import haxe.io.BytesInput;
import openfl.Assets;
import openfl.geom.Rectangle;
import haxe.io.Input;
import sys.io.File;
import format.tga.Data.Header;
import openfl.utils.ByteArray;
import lime.app.Future;
import haxe.ds.Vector;
import openfl.display.Tileset;
import openfl.display.Tilemap;
import openfl.display.BitmapData;
import openfl.display.Tile;

class Ground extends TileDisplay
{
    var game:Game;
    public static inline var biomeNum:Int = 6;
    public function new(game:Game)
    {
        super(4096,4096);
        this.game = game;
        for(i in 0...biomeNum) cacheBiome(i);
    }
    private function cacheBiome(id:Int)
    {
        for(a in ["","_square"])
        {
            for(y in 0...4)
            {
                for(x in 0...4)
                {
                    cache(Static.dir + "groundTileCache/biome_" + id + "_x" + x + "_y" + y + a + ".tga");
                }
            }
        }
    }
    public function add(id:Int,x:Int,y:Int)
    {
        //0-16 corners,17-32 square
        trace("id " + id);
        var pos:Int = Math.floor(id/32) * 32;
        var index:Int = ci(x) + ci(y) * 3;
        var tile = new Tile(pos + index);
        addTile(tile);
        var tile = new Tile(0);
        addTile(tile);
        trace("add ground " + Std.string(pos + index) + " x " + x + " y " + y);
    }
    private inline function ci(i:Int):Int
    {
        if(i > 0)
        {
            while (i > 2) i += -3;
        }else{
            while (i < 0) i += 3;
        }
        return i;
    }
    override function cacheRect(len:Int) 
    {
        if (len >= biomeNum * 2 * 16)
        {
            trace("finish rects");
            tileset.bitmapData.lock();
            //cache is loaded up now let's display
            new Future(function()
            {
                for(i in 0...tileset.numRects)
                {
                    tileset.bitmapData.setPixels(tileset.getRect(i),bytesArray[i]);
                }
                return true;
            },true).onComplete(function(value)
            {
                tileset.bitmapData.unlock();
                trace("finish tileset");
            }).onError(function(error:Dynamic)
            {
                trace("error drawing " + error);
            });
        }
    }
}