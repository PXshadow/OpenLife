package states.game;

import openfl.display.Tileset;
import openfl.geom.Matrix;
import sys.io.File;
import data.TgaData;
import openfl.geom.Rectangle;
import openfl.display.BitmapData;
import openfl.display.Shape;
import openfl.Vector;

//possibly implement drawQuads instead for increased performance
class Ground extends Shape
{
    var game:Game;
    var tileset:Tileset;
    var rect:Rectangle = new Rectangle(0,0,Static.GRID,Static.GRID);
    public static inline var biomeNum:Int = 6 + 1;
    var reader:TgaData;
    private var indices:Vector<Int> = new Vector<Int>();
	private var transforms:Vector<Float> = new Vector<Float>();
    public function new(game:Game)
    {
        super();
        this.game = game;
        tileset = new Tileset(new BitmapData(2000,2000,false));
        reader = new TgaData();
        for (i in 0...biomeNum) cacheBiome(i);
        graphics.beginBitmapFill(tileset.bitmapData);
        graphics.drawQuads(tileset.rectData,indices,transforms);
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
                    indices.push(indices.length - 1);
                    transforms.push(rect.x);
                    transforms.push(rect.y);
                    rect.x += rect.width;
                    if (rect.x >= tileset.bitmapData.width)
                    {
                        rect.x = 0;
                        rect.y += rect.height;
                    }
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
}