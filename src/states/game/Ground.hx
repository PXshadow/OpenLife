package states.game;

import openfl.display.Tileset;
import openfl.geom.Matrix;
import sys.io.File;
import data.TgaData;
import openfl.geom.Rectangle;
import openfl.display.BitmapData;
import openfl.display.Shape;
import openfl.Vector;

//possibly implement drawQuads instead for increased performance, haha thanks previous self you were right.
class Ground extends Shape
{
    var game:Game;
    var tileset:Tileset;
    var rect:Rectangle = new Rectangle(0,0,Static.GRID,Static.GRID);
    public static inline var biomeNum:Int = 6 + 1;
    var reader:TgaData;
    //id of tile
    public var indices:Vector<Int> = new Vector<Int>();
    //x and y
	public var transforms:Vector<Float> = new Vector<Float>();
    public function new(game:Game)
    {
        super();
        //cache with ability to scale
        cacheAsBitmapMatrix = new Matrix();
        this.game = game;
        tileset = new Tileset(new BitmapData(2000,2000,false));
        reader = new TgaData();
        for (i in 0...biomeNum) cacheBiome(i);
    }
    public function createGround()
    {
        graphics.clear();
        graphics.beginBitmapFill(tileset.bitmapData);
        graphics.drawQuads(tileset.rectData,indices,transforms);
    }
    public function render()
    {
        createGround();
    }
    private function cacheBiome(id:Int)
    {
            var a = "_square";
            //16
            for(j in 0...4)
            {
                for(i in 0...4)
                {
                    reader.read(File.read(Static.dir + "groundTileCache/biome_" + id + "_x" + i + "_y" + j + a + ".tga").readAll());
                    tileset.bitmapData.setPixels(rect,reader.bytes);
                    tileset.addRect(rect);
                    rect.x += rect.width;
                    if (rect.x >= tileset.bitmapData.width)
                    {
                        rect.x = 0;
                        rect.y += rect.height;
                    }
                }
            }
    }
    public function add(id:Int,x:Int=0,y:Int=0)
    {
        indices.push(id + ci(x) + ci(y) * 3);
        transforms.push(x * Static.GRID);
        transforms.push(y * Static.GRID);
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
}