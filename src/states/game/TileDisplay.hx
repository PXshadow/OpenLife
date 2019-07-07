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

class TileDisplay extends Tilemap
{
    var reader:TgaData;
    var rect:Rectangle = new Rectangle();
    var backlog:Array<String> = [];
    //pos
    var tileWidth:Int = 0;
    var tileHeight:Int = 0;
    public function new(tilesetWidth:Int,tilesetHeight:Int)
    {
        super(0,0,new Tileset(new BitmapData(tilesetWidth,tilesetHeight)));
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
    public function cache(path:String):Int
    {
        //add task
        //setup worker
        reader.read(File.read(Launcher.dir + path,true).readAll());
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