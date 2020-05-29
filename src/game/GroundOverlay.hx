package game;
import resources.Resource;
#if openfl
import openfl.display.Sprite;
import openfl.display.Shape;
import openfl.geom.Rectangle;
import graphics.TgaData;
import openfl.Vector;
import openfl.display.Tileset;
import openfl.display.BitmapData;
#if nativeGen @:nativeGen #end
class GroundOverlay extends Shape
{
    var reader:TgaData = new TgaData();
    var tileHeight:Int = 0;
    //for tileset
    var tileX:Float = 0;
    var tileY:Float = 0;
    var tileset:Tileset;
    var indices:Vector<Int>;
    var transforms:Vector<Float>;
    var overlayBool:Bool = false;
    var ground:Ground;
    public function new(ground:Ground)
    {
        this.ground = ground;
        super();
        alpha = 0.3;
        tileset = new Tileset(new BitmapData(4096,4096,true));
        cacheOverlay();
    }
    public function render()
    {
        clear();
        overlay();
        graphics.clear();
        graphics.beginBitmapFill(tileset.bitmapData,null,false,true);
        graphics.drawQuads(tileset.rectData,indices,transforms);
    }
    private function clear()
    {
        indices = new Vector<Int>();
        transforms = new Vector<Float>();
    }
    private function overlay()
    {
        var width = Math.ceil(ground.width / (Static.GRID * 8));
        var height = Math.ceil(ground.height / (Static.GRID * 8));
        for (x in 0...width)
        {
            for (y in 0...height)
            {
                indices.push(x % 2 + (y % 2) * 2);
                transforms.push(x * Static.GRID * 8 - Static.GRID - ground.width/2);
                transforms.push((Static.tileHeight - y) * Static.GRID * 8 - Static.GRID * 8 + ground.height/2);
            }
        }
    }
    private function cacheOverlay()
    {
        if (!sys.FileSystem.exists('${Game.dir}/graphics')) return;
        var rect:Rectangle = new Rectangle(tileX,tileY);
        for (i in 0...4)
        {
            reader.read(Resource.groundOverlay(i));
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
            tileX += Std.int(rect.width);
            //set to bitmapData
            tileset.bitmapData.setPixels(rect,reader.bytes);
            tileset.addRect(rect);
            if (rect.height > tileHeight) tileHeight = Std.int(rect.height);
        }
        overlayBool = true;
    }
}
#end