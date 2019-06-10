import openfl.geom.ColorTransform;
import ObjectData.SpriteData;
import haxe.ds.Vector;
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
class Display extends Tilemap
{
    //static vars
    inline private static var BASE_SPEED:Float = 3.75;
    //tile vars
    var tileX:Int = 0;
    var tileY:Int = 0;
    var tileHeight:Int = 0;
    var cacheMap:Map<Int,Int> = new Map<Int,Int>();
    var biomeMap:Map<Int,Vector<Int>> = new Map<Int,Vector<Int>>();
    var renderMap:Map<Int,Vector<SpriteData>> = new Map<Int,Vector<SpriteData>>();
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
        if(p == null) p = new Display.Player(data.p_id);
        var obj = new ObjectData(data.po_id);
        var length:Int = numTiles;
        //ids
        p.pid = data.po_id;
        p.oid = data.o_id;
        //set age
        trace("data age " + data.age);
        p.age = data.age;
        p.speed = data.move_speed;
        //draw
        renderMap.set(obj.id,obj.spriteArray);
        createTile(obj.spriteArray,data.o_origin_x + 4,data.o_origin_y + 4);
        //clothing
        var i = data.clothing_set.split(",");
        p.hat = Std.parseInt(i[0]);
        p.tunic = Std.parseInt(i[1]);
        p.front_shoe = Std.parseInt(i[2]);
        p.back_shoe = Std.parseInt(i[3]);
        p.bottom = Std.parseInt(i[4]);
        p.backpack = Std.parseInt(i[5]);
        //set body parts
        p.head = obj.headIndex + length;
        p.body = obj.bodyIndex + length;
        p.frontFoot = obj.frontFootIndex;
        p.backFoot = obj.backFootIndex;
        for(value in p.frontFoot) value += length;
        for(value in p.backFoot) value += length;
        //set section of player tiles
        p.setSection(length, numTiles - length);
        setPlayerAge(p);
        /*for(i in p.index...p.index+p.length)
        {
            getTileAt(i).visible = false;
        }
        //show basic player
        getTileAt(p.head).visible = true;
        getTileAt(p.body).visible = true;
        for(i in p.frontFoot) getTileAt(i).visible = true;
        for (i in p.backFoot) getTileAt(i).visible = true;*/
        //create box for testing
        var obj = new ObjectData(434);
        length = numTiles;
        createTile(obj.spriteArray,4,4);
        var box = new Group();
        for(i in length...numTiles)
        {
            box.add(cast(getTileAt(i),Tile));
        }
        box.y += -90;
    }
    public function setPlayerAge(p:Player)
    {
        p.age = 2;
        if(renderMap.exists(p.pid))
        {
            var index:Int = 0;
            var array = renderMap.get(p.pid);
            var sprite:SpriteData;
            for(i in p.index...p.index + p.length)
            {
                sprite = array[index++];
                if((sprite.ageRange[0] > p.age || sprite.ageRange[1] < p.age) && sprite.ageRange[0] > 0)
                {
                    //outside of range of age
                    getTileAt(i).visible = false;
                }
            }
        }else{
            throw("player rendermap object not found");
        }
    }
    public function addChunk(type:Int,x:Int,y:Int)
    {
        var index:Int = x % 3 + (y % 3) * 3;
        var tile = new Tile(biomeMap.get(type)[index],TileType.Ground);
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
            //todo: save entire object data without spriteData

            //saves spriteData section
            renderMap.set(id,data.spriteArray);
            //draw
            createTile(data.spriteArray,x,y);
        }else{
            //trace("group " + data);
        }
    }
    public function createTile(array:Vector<SpriteData>,x:Int,y:Int)
    {
        //set to grid
        x *= Static.GRID;
        y *= Static.GRID;
        //add array tiles
        for(obj in array)
        {
            var cache = cacheObject(obj.spriteID);
            var rect = tileset.getRect(cache);
            var tile = new Tile(cache,Object);
            /*var w:Int = 1;
            var h:Int = 1;
            while (w < rect.width)w *= 2;
            while (h < rect.height) h *= 2;*/
            if(obj.rot > 0)
            {
                tile.rotation = obj.rot * 180 * 2;
            }
            //color
            tile.colorTransform = new ColorTransform();
            tile.colorTransform.redMultiplier = obj.color[0];
            tile.colorTransform.greenMultiplier = obj.color[1];
            tile.colorTransform.blueMultiplier = obj.color[2];
            //pos
            tile.x = x + obj.pos.x - obj.inCenterXOffset - rect.width/2;
            tile.y = y + -obj.pos.y - obj.inCenterYOffset - rect.height/2;
            addTile(tile);
        }
    }
    public function cacheBiome(id:Int)
    {
        var data:{bytes:ByteArray,header:Header};
        var vector = new Vector<Int>(3 * 3);
        for(y in 0...3 + 1)
        {
            for(x in 0...3 + 1)
            {
                //trace("x " + x + " y " + y);
                data = Static.tgaBytes(Settings.assetPath + 
                "groundTileCache/biome_" + id + "_x" + x + "_y" + y + "_square.tga");
                var rect:Rectangle = new Rectangle(tileX + x * Static.GRID,y * Static.GRID,Static.GRID,Static.GRID);
                tileset.bitmapData.setPixels(rect,data.bytes);
                vector[x + y * 3] = tileset.addRect(rect);
            }
        }
        biomeMap.set(id,vector);
        tileHeight = Static.GRID * 3;
        tileX += Static.GRID * 3;
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
    public var backFoot:Array<Int> = [];
    public var frontFoot:Array<Int> = [];
    public var index:Int = 0;
    public var length:Int = 0;
    public var id:Int = 0;
    //object id, what is being held
    public var oid:Int = 0;
    //id refrence to renderMap
    public var pid:Int = 0;
    public var age:Int = 0;
    public var speed:Float = 0;
    //clothing
    public var hat:Int = 0;
    public var tunic:Int = 0;
    public var front_shoe:Int = 0;
    public var back_shoe:Int = 0;
    public var bottom:Int = 0;
    public var backpack:Int = 0;
    public static var active:Map<Int,Player> = new Map<Int,Player>();
    public function new(id:Int)
    {
        super();
        this.id = id;
        active.set(id,this);
    }
    public function setSection(index:Int,length:Int)
    {
        this.index = index;
        this.length = length;
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