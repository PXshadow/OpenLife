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
    static inline var biomeNum:Int = 6;
    var cacheMap:Map<Int,Vector<Int>> = new Map<Int,Vector<Int>>();
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
    override function cacheRect(length:Int) 
    {
        if (length >= biomeNum * 2 * 16)
        {
            tileset.bitmapData.lock();
            //cache is loaded up now let's display
            new Future(function()
            {
                for(i in 0...tileset.numRects)
                {
                    tileset.bitmapData.setPixels(tileset.getRect(i),bytesArray[i]);
                }
            }).onComplete(function(_)
            {
                tileset.bitmapData.unlock();
            }).onError(function(error:Dynamic)
            {
                trace("error drawing " + error);
            });
        }
    }
}