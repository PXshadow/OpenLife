package game;

import openfl.geom.Matrix;
import data.GameData;
import openfl.Vector;
import data.TgaData;
import openfl.display.BitmapData;
import openfl.display.Tileset;
import openfl.geom.Rectangle;
import sys.io.File;
import openfl.display.Shape;

class Ground extends Shape
{
    var reader:TgaData = new TgaData();
    var tileHeight:Int = 0;
    //for tileset
    var tileX:Float = 0;
    var tileY:Float = 0;
    var tileset:Tileset;
    public var indices:Vector<Int>;
    public var transforms:Vector<Float>;
    public var data:GameData = null;
    public var simple:Bool = false;
    public var simpleIndex:Int = 0;
    public function new()
    {
        super();
        clear();
        //opaqueBackground = 0;
        //cacheAsBitmapMatrix = new Matrix();
        tileset = new Tileset(new BitmapData(2000,2000,false,0));
        //0 is blank for tileData reading
        //add cached ground
        for (i in 0...6 + 1) cache(i);
        simpleIndex = tileset.numRects;
        /*for (color in [0x80ad57,0xe0a437,0x5c584e,0xffffff,0x467c06])
        {
            simpleCache(color);
        }*/
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
    public function render()
    {
        graphics.clear();
        graphics.beginBitmapFill(tileset.bitmapData);
        graphics.drawQuads(tileset.rectData,indices,transforms);
    }
    public function clear()
    {
        indices = new Vector<Int>();
        transforms = new Vector<Float>();
    }
    public function add(id:Int,x:Int,y:Int)
    {
        if (simple)
        {
            indices.push(simpleIndex + id);
        }else{
            indices.push(id * 16 + ci(x) + ci(y) * 3 + 1);
        }
        transforms.push(x * Static.GRID - Static.GRID/2);
        transforms.push((Static.tileHeight - y) * Static.GRID - Static.GRID/2);
    }
    public function simpleCache(color:UInt)
    {
        var rect = new Rectangle(tileX,tileY,Static.GRID,Static.GRID);
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
        tileX += Math.ceil(rect.width) + 1;
        tileset.bitmapData.fillRect(rect,color);
        tileset.addRect(rect);
        if (rect.height > tileHeight) tileHeight = Math.ceil(rect.height) + 1;
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
                    if (rect.height > tileHeight) tileHeight = Std.int(rect.height) + 1;
                }
            }
    }
}