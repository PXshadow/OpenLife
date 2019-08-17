package states.game;

import data.TgaData;
import openfl.display.TileContainer;
import openfl.display.BitmapData;
import openfl.display.Tileset;
import openfl.display.Tilemap;
import openfl.geom.Rectangle;
import sys.io.File;
import openfl.display.Tile;

class Ground extends Tilemap
{
    public var group:TileContainer;
    var reader:TgaData = new TgaData();
    var tileHeight:Int = 0;
    //for tileset
    var tileX:Float = 0;
    var tileY:Float = 0;
    var game:Game;
    public function new(game:Game)
    {
        super(0,0,new Tileset(new BitmapData(2000,2000,false,0)),false);
        this.game = game;
        /*tileAlphaEnabled = false;
        tileColorTransformEnabled = false;
        tileBlendModeEnabled = false;*/
        //opaqueBackground = 0;
        group = new TileContainer();
        addTile(group);
        //add cached ground
        for (i in 0...6 + 1) cache(i);
    }
    private inline function ci(i:Int):Int
    {
        if(i > 0)
        {
            while (i > 2 + 1) i += -3 - 1;
        }else{
            while (i < 0) i += 3 + 1;
        }
        return i;
    }
    public function add(id:Int,x:Int,y:Int):Tile
    {
        var object = new Tile();
        object.id = id * 16 + ci(x) + ci(y) * 3;
        object.x = x * Static.GRID - Static.GRID/2;
        object.y = (Static.tileHeight - y) * Static.GRID - Static.GRID/2;
        group.addTile(object);
        //if (group.numTiles > 900)group.removeTileAt(0);
        //add to chunk
        game.data.chunk.latest.ground.set(x,y,object);
        return object;
    }
    //cache ground tiles
    public function cache(id:Int)
    {
        var a = "_square";
        var rect:Rectangle = new Rectangle(tileX,tileY);
            //16
            for(j in 0...4)
            {
                for(i in 0...4)
                {
                    reader.read(File.read(Static.dir + "groundTileCache/biome_" + id + "_x" + i + "_y" + j + a + ".tga").readAll());
                    //set dimensions
                    rect.x = tileX;
                    rect.y = tileY;
                    rect.width = reader.rect.width;
                    rect.height = reader.rect.height;
                    //move down column
                    if(rect.x + rect.width >= tileset.bitmapData.width)
                    {
                        tileX = 0;
                        tileY += tileHeight;
                        rect.x = tileX;
                        rect.y = tileY;
                        tileHeight = 0;
                    }
                    //move tilesystem
                    tileX += Std.int(rect.width) + 1;
                    //set to bitmapData
                    tileset.bitmapData.setPixels(rect,reader.bytes);
                    tileset.addRect(rect);
                    if (rect.height > tileHeight) tileHeight = Std.int(rect.height);
                }
            }
    }
}