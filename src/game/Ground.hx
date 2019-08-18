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
    public var indices:Vector<Int> = new Vector<Int>();
    public var transforms:Vector<Float> = new Vector<Float>();
    public var data:GameData = null;
    public function new()
    {
        super();
        indices [0];
        //opaqueBackground = 0;
        //cacheAsBitmapMatrix = new Matrix();
        tileset = new Tileset(new BitmapData(2000,2000,false,0));
        //0 is blank for tileData reading
        tileset.addRect(new Rectangle());
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
    public function render()
    {
        graphics.clear();
        graphics.beginBitmapFill(tileset.bitmapData);
        graphics.drawQuads(tileset.rectData,indices,transforms);
    }
    public function add(id:Int,x:Int,y:Int)
    {
        var index = get();
        indices[index] = id * 16 + ci(x) + ci(y) * 3;
        transforms[index * 2] = x * Static.GRID - Static.GRID/2;
        transforms[index * 2 + 1] = (Static.tileHeight - y) * Static.GRID - Static.GRID/2;
        data.tileData.biome.set(x,y,index);
    }
    public function get():Int
    {
        var index = indices.indexOf(0);
        //not found add new
        if (index == -1) return indices.length;
        return index;
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