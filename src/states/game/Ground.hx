package states.game;

import openfl.display.Bitmap;
import haxe.io.Bytes;
import haxe.io.BytesInput;
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
import openfl.display.Bitmap;
import states.launcher.Launcher;

//possibly implement drawQuads instead for increased performance
class Ground extends TileDisplay
{
    var game:Game;
    public static inline var biomeNum:Int = 6 + 1;
    public var tileArray:Array<Array<Tile>> = [];
    var current:Tile;
    var row:Array<Tile>;
    var rect:Rectangle = new Rectangle();
    public function new(game:Game)
    {
        tileAlphaEnabled = false;
        tileBlendModeEnabled = false;
        tileColorTransformEnabled = false;
        super(4096,4096,false);
        this.game = game;
        for(i in 0...biomeNum) cacheBiome(i);
        cacheBiome(99999);
    }
    private function pool()
    {
        //array that holds tiles y than x
        for(j in 0...tileHeight)
        {
            tileArray[j] = [];
            for(i in 0...tileWidth)
            {
                var tile = new Tile();
                tile.x = i * Static.GRID;
                tile.y = j * Static.GRID;
                tileArray[j][i] = tile;
                addTile(tile);
            }
        }
    }
    override function size(width:Int, height:Int) 
    {
        super.size(width, height);
        pool();
        trace("create ground pool");
    }
    public function update()
    {
        //if (tileArray.length == 0) return;
        moveX();
        moveY();
    }
    private function moveX()
    {
        while (x >= Static.GRID)
        {
            game.tileX += -1;
            for (i in 0...tileArray.length) 
            {
                //move
                for (tile in tileArray[i]) tile.x += Static.GRID;
                //shift
                current = tileArray[i].pop();
                current.x = 0;
                current.id = get(0,i);
                tileArray[i].unshift(current);
            }
            x += -Static.GRID;
        }
        while (x <= -Static.GRID)
        {
            game.tileX += 1;
            for (i in 0...tileArray.length) 
            {
                //move
                for (tile in tileArray[i]) tile.x += -Static.GRID;
                //shift
                current = tileArray[i].shift();
                current.x = (tileWidth - 1) * Static.GRID;
                current.id = get(tileWidth - 1,i);
                tileArray[i].push(current);
            }
            x += Static.GRID;
        }
    }
    private function moveY()
    {
        while (y >= Static.GRID)
        {
            game.tileY += -1;
            for (array in tileArray) for (tile in array) tile.y += Static.GRID;
            row = tileArray.pop();
            for (i in 0...row.length)
            {
                row[i].y = 0;
                row[i].id = get(i,0);
            }
            tileArray.unshift(row);
            y += -Static.GRID;
        }
        while (y <= -Static.GRID)
        {
            game.tileY += 1;
            for (array in tileArray) for (tile in array) tile.y += -Static.GRID;
            row = tileArray.shift();
            for (i in 0...row.length)
            {
                row[i].y = (tileHeight - 1) * Static.GRID;
                row[i].id = get(i,tileHeight - 1);
            }
            tileArray.push(row);
            y += Static.GRID;
        }
    }
    public function get(x:Int,y:Int):Int
    {
        x = game.tileX + x;
        y = game.tileY + y;
        var id:Null<Int> = game.data.map.biome.get(x + "." + y);
        var index:Int = 0;
        if (id == null)
        {
            id = 16 * biomeNum + 1;
            //id = 0;
        }else{
            //pos in square
            index = ci(x) + ci(y) * 4;
            //id absolute
            id = id * 16 + index;
        }
        return id;
    }
    private function cacheBiome(id:Int)
    {
            var a = "_square";
            //16
            for(j in 0...4)
            {
                for(i in 0...4)
                {
                    var i = cache("groundTileCache/biome_" + id + "_x" + i + "_y" + j + a + ".tga");
                    //draw
                    tileset.bitmapData.setPixels(tileset.getRect(i),reader.bytes);
                }
            }
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
    public function cache(path:String):Int
    {
        //add task
        //setup worker
        reader.read(File.read(Static.dir + path,true).readAll());
        return setRect();
    }
    private function setRect():Int
    {
        if(rect.x + rect.width >= tileset.bitmapData.width)
        {
            //new row
            rect.y += rect.height;
            rect.width = 0;
            rect.height = 0;
            rect.x = 0;
        }
        //shift across the rows
        rect.x += rect.width;
        rect.width = reader.rect.width;
        rect.height = reader.rect.height > rect.height ? reader.rect.height : rect.height;
        //add to tileset
        return tileset.addRect(rect);
    }
}