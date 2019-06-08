import PlayerData.PlayerType;
import openfl.display.BitmapDataChannel;
import sys.io.File;
import format.tga.Data.Header;
import openfl.utils.ByteArray;
import haxe.Timer;
import lime.app.Future;
import openfl.display.Bitmap;
import sys.FileSystem;
import openfl.display.BitmapData;
import openfl.display.Tileset;
import openfl.display.Tilemap;
import openfl.geom.Rectangle;
typedef RenderType = {offset:{x:Int,y:Int},graphic:Int,inCenterXOffset:Int,inCenterYOffset:Int}
class Display extends Tilemap
{
    var tileX:Int = 0;
    var tileY:Int = 0;
    var tileHeight:Int = 0;
    var cacheMap:Map<Int,Int> = new Map<Int,Int>();
    var renderMap:Map<Int,Array<RenderType>> = new Map<Int,Array<RenderType>>();
    public var inital:Bool = true;
    //entire display
    public var setX:Int = 0;
    public var setY:Int = 0;
    public var sizeX:Int = 0;
    public var sizeY:Int = 0;
    public function new()
    {
        super(Static.GRID * 32,Static.GRID * 32);
        tileset = new Tileset(new BitmapData(1600 * 4,1600 * 4));
        //return;
        for(i in 0...6 + 1) cacheBiome(i);
    }
    public function addPlayer(data:PlayerType)
    {
        var p = Display.Player.active.get(data.p_id);
        if(p == null)
        {
            p = new Display.Player(data.p_id);
        }
    }
    public function addChunk(type:Int,x:Int,y:Int)
    {
        var tile = new Tile(type,TileType.Ground);
        tile.x = x * Static.GRID;
        tile.y = y * Static.GRID;
        addTile(tile);
    }
    public function addFloor(type:Int,x:Int,y:Int)
    {
        if(type > 0)
        {
            trace("type " + type);
        }
    }
    public function addObject(data:String,x:Int,y:Int)
    {
        var id = Std.parseInt(data);
        if(id == 0) return;
        if(id > 0)
        {
            //check if exists
            var exist = renderMap.get(id);
            if(exist != null)
            {
                //trace("exist");
                createTile(exist,x,y);
                return;
            }
            var data = new ObjectData(id);
            var array:Array<RenderType> = [];
            for(obj in data.spriteArray)
            {
                array.push({offset:{x:Std.int(obj.pos.x),y:Std.int(obj.pos.y)},graphic: obj.spriteID,inCenterXOffset: 0,inCenterYOffset: 0});
            }
            getSpriteData(array);
            renderMap.set(id,array);
            createTile(array,x,y);
        }else{
            trace("group " + data);
        }
    }
    public function getSpriteData(array:Array<RenderType>)
    {
        //get sprite data
        for(index in 0...array.length)
        {
        var input = File.read(Settings.assetPath + "sprites/" + array[index].graphic + ".txt",false);
        var i:Int = 0;
        var a = input.readLine().split(" ");
        for(string in a)
        {
            switch(i)
            {
                case 0:
                //name

                case 1:
                //multitag

                case 2:
                //centerX
                array[index].inCenterXOffset = Std.parseInt(string);
                case 3:
                //centerY
                array[index].inCenterYOffset = Std.parseInt(string);
                    
            }
            i++;
        }
        }
    }
    public function createTile(array:Array<RenderType>,x:Int,y:Int)
    {
        //set to grid
        x *= Static.GRID;
        y *= Static.GRID;
        //add array tiles
        for(obj in array)
        {
            var cache = cacheObject(obj.graphic);
            var rect = tileset.getRect(cache);
            var tile = new Tile(cache,Object);
            var w:Int = 1;
            var h:Int = 1;
            while (w < rect.width)w *= 2;
            while (h < rect.height) h *= 2;
            //rect.width = w;
            //rect.height = h;
            //trace("width " + w + " height " + h + " center x " + obj.inCenterXOffset + " y " + obj.inCenterYOffset);
            tile.x = x + obj.offset.x - obj.inCenterXOffset - rect.width/2;
            tile.y = y + -obj.offset.y - obj.inCenterYOffset - rect.height/2;
            addTile(tile);
        }
    }
    public function cacheBiome(id:Int)
    {
        var data:{bytes:ByteArray,header:Header} = Static.tgaBytes(Settings.assetPath + 
        "groundTileCache/biome_" + id + "_x" + 1 + "_y" + 1 + "_square.tga");
        var rect:Rectangle = new Rectangle(tileX,0,Static.GRID,Static.GRID);
        tileset.bitmapData.setPixels(rect,data.bytes);
        tileset.addRect(rect);
        tileX += Static.GRID;
        tileHeight = Static.GRID;
    }
    public function cacheObject(id:Int):Int
    {
        if(cacheMap.exists(id))
        {
            return cacheMap.get(id);
        }
        var data = Static.tgaBytes(Settings.assetPath + "sprites/" + id + ".tga");
        var rect = new Rectangle(tileX,tileY,data.header.width,data.header.height);
        var bytes:ByteArray = ByteArray.fromBytes(haxe.io.Bytes.alloc(data.bytes.length));
        data.bytes.readBytes(bytes,0,data.bytes.length);
        data.bytes.position = 0;
        var color:UInt;
        var minX:Int = Std.int(rect.width) - 1;
        var minY:Int = Std.int(rect.height) - 1;
        var maxX:Int = 0;
        var maxY:Int = 0;
        for(y in 0...Std.int(rect.height))
        {
            for(x in 0...Std.int(rect.width))
            {
                color = bytes.readUnsignedInt();
                if(color >> 24 & 255 == 0) continue;
                if(x < minX) minX = x;
                if (y < minY) minY = y;
                if (x > maxX) maxX = x;
                if (y > maxY) maxY = y;
            }
        }
        tileset.bitmapData.setPixels(rect,data.bytes);
        if(rect.x + rect.width > tileset.bitmapData.width)
        {
            tileX = 0;
            tileY += tileHeight;
            tileHeight = 0;
        }else{
            tileX += Std.int(rect.width);
            tileHeight = Std.int(Math.max(rect.height,tileHeight));
        }
        //crop transparent unused area
        rect.x += minX;
        rect.y += minY;
        rect.width = maxX - minX;
        rect.height = maxY - minY;
        //add tileset rect
        var i = tileset.addRect(rect);
        cacheMap.set(id,i);
        return i;
    }
}
class Tile extends openfl.display.Tile
{
    var type:TileType;
    public function new(id:Int,type:TileType)
    {
        super(id);
        this.type = type;
    }
}
enum TileType 
{
    Ground;
    Object;
    Floor;
    Player;
}
class Player extends Group
{
    public var head:Int = 0;
    public var body:Int = 0;
    public var id:Int = 0;
    public static var active:Map<Int,Player> = new Map<Int,Player>();
    public function new(id:Int)
    {
        super();
        active.set(id,this);
        this.id = id;
    }
    public function unactive()
    {
        active.remove(id);
    }
}
class Group
{
    var childern:Array<Tile> = [];
    @:isVar public var x(default,set):Int = 0;
    function set_x(value:Int):Int
    {
        var change = value - x;
        for(child in childern)
        {
            child.x += change;
        }
        return x = value;
    }
    @:isVar public var y(default,set):Int = 0;
    function set_y(value:Int):Int
    {
        var change = value - y;
        for (child in childern)
        {
            child.y += change;
        }
        return y = value;
    }
    public function new()
    {
        
    }
    public function add(child:Tile):Int
    {
        return childern.push(child);
    }
    public function remove(child:Tile)
    {
        childern.remove(child);
    }
}