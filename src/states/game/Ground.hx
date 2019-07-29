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
    public var indices:Vector<Int> = new Vector<Int>();
	public var transforms:Vector<Float> = new Vector<Float>();
    //sets what is to be drawn
    public var drawBlocks:Bool = true;
    public var drawGrid:Bool = true;
    public var drawPath:Bool= true;
    public var dest:console.Program.Pos = null;
    public function new(game:Game)
    {
        super();
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
    public function createPath()
    {
        /*graphics.lineStyle(0x00FF00,2);
        graphics.moveTo((Player.main.instance.x - game.data.map.x - game.cameraX) * Static.GRID,(Player.main.instance.y - game.data.map.y - game.cameraY) * Static.GRID);
        graphics.lineTo((dest.x - game.data.map.x - game.cameraX) * Static.GRID,(dest.y - game.data.map.y - game.cameraY) * Static.GRID);*/
    }
    public function createBlocks()
    {
        if (!drawBlocks) return;
    }
    public function createGrid()
    {
        if (!drawGrid) return;
        //black 2 solid line
        graphics.lineStyle(2,0);
        //perimiter
        graphics.moveTo((x + 0) * Static.GRID,(y + 1) * Static.GRID);
        graphics.lineTo((x + 1) * Static.GRID,(y + 1) * Static.GRID);
        graphics.lineTo((x + 1) * Static.GRID,(y + 0) * Static.GRID);

        for (x in 0...game.mapInstance.width)
        {
            for(y in 0...game.mapInstance.height)
            {
                //square
                graphics.moveTo((x + 0) * Static.GRID,(y + 1) * Static.GRID);
                graphics.lineTo((x + 1) * Static.GRID,(y + 1) * Static.GRID);
                graphics.lineTo((x + 1) * Static.GRID,(y + 0) * Static.GRID);
            }
        }
    }
    public function render()
    {
        createGround();
        createGrid();
        createBlocks();
        createPath();
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