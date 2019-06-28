import openfl.display.Shader;
import motion.Actuate;
import openfl.geom.ColorTransform;
import ObjectData.SpriteData;
import haxe.ds.Vector;
import PlayerData.PlayerType;
import openfl.display.BitmapDataChannel;
import format.tga.Data.Header;
import openfl.utils.ByteArray;
import haxe.Timer;
import lime.app.Future;
import openfl.display.Bitmap;
#if sys
import sys.FileSystem;
import sys.io.File;
#end
import openfl.display.BitmapData;
import openfl.display.Tileset;
import openfl.display.Tilemap;
import openfl.geom.Rectangle;
class Display extends Tilemap
{
    //static vars
    inline private static var BASE_SPEED:Float = 3.75;
    //tile vars
    var groundShader:Shader;
    var tileX:Int = 0;
    var tileY:Int = 0;
    var tileHeight:Int = 0;
    var cacheMap:Map<Int,Int> = new Map<Int,Int>();
    var biomeMap:Map<Int,Vector<Int>> = new Map<Int,Vector<Int>>();
    public static var renderMap:Map<Int,Vector<SpriteData>> = new Map<Int,Vector<SpriteData>>();
    //map
    public var objectMap:Map<String,Group> = new Map<String,Group>();
    //animation bank
    public var animationArray:Array<Animation> = [];

    public var inital:Bool = true;
    //entire display
    public var setX:Int = 0;
    public var setY:Int = 0;
    public var sizeX:Int = 0;
    public var sizeY:Int = 0;
    public function new()
    {
        super(Static.GRID * 32,Static.GRID * 32);
        //shader = 
        groundShader = null;
        tileset = new Tileset(new BitmapData(1600 * 4,1600 * 4));
        //return;
        for(i in 0...6 + 1) cacheBiome(i);
    }
    public function updatePlayer(data:PlayerType):Bool
    {
        trace("update player " + data.forced);
        //update player
        var player:Player = null;
        player = Player.active.get(data.p_id);
        if(player == null) return false;
        //set tile int pos
        motion.Actuate.pause(this);
        player.tileX = data.x;
        player.tileY = data.y;
        player.x = player.tileX * Static.GRID;
        player.y = -player.tileY * Static.GRID;
        trace("player tileX " + player.tileX + " Y " + player.tileY);
        player.speed = data.move_speed;
        //set age
        player.age = data.age;
        //p.ageSystem(data.age_r);
        player.speed = data.move_speed;
        //object
        player.oid = data.po_id;
        //age
        player.agePlayer();
        //force
        if (data.forced == 1)
        {
            if(Player.main == player)
            {
                trace("FORCE");
                Main.client.send("FORCE " + player.tileX + " " + player.tileY);
            }
        }
        return true;
    }
    public function addPlayer(data:PlayerType)
    {
        //return;
        trace("age " + data.age);
        //trace("add player x " + data.o_origin_x + " y " + data.o_origin_y);
        var p = new Player(data.p_id,data.x,data.y);
        var obj = new ObjectData(data.po_id);
        //trace("obj fail " + obj.fail);
        if(obj.fail) return;
        //trace("create1");
        var length:Int = numTiles;
        //draw
        renderMap.set(obj.id,obj.spriteArray);
        //trace("tiles num " + obj.spriteArray.length);
        createTile(obj.spriteArray,data.x,data.y);
        //player as object
        p.poid = obj.id;
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
        var tile:Tile;

        for(i in length...length + obj.spriteArray.length)
        {
            tile = cast getTileAt(i);
            if(tile == null) throw("tile null " + i);
            p.add(tile);
        }
        //update player
        updatePlayer(data);
    }
    public function addChunk(type:Int,x:Int,y:Int)
    {
        //I will one day figure out how I wrote this, haha.
        var ix:Int = x;
        var iy:Int = y;
        if (ix > 0) 
        {
            while(ix > 2) ix += -3;
        }else{
            while(ix < 0) ix += 3;
        }
        if (iy > 0) 
        {
            while(iy > 2) iy += -3;
        }else{
            while(iy < 0) iy += 3;
        }
        trace("ix " + ix + " iy " + iy);
        var index:Int = ix + iy * 3;
        trace("index " + index + " type " + type);
        var tile = new Tile(biomeMap.get(type)[index],TileType.Ground);
        tile.x = (x - setX) * Static.GRID;
        tile.y = (y - setY) * Static.GRID;
        //shader
        tile.shader = groundShader;
        addTileAt(tile,numTiles);
    }
    public function addFloor(id:Int,x:Int,y:Int)
    {
        if(id > 0)
        {
            //trace("type " + type);
            //check if exists
            var exist = renderMap.get(id);
            if(exist != null)
            {
                //trace("exist");
                createTile(exist,x,y);
                return;
            }
            var data = new ObjectData(id);
            if(data.fail) return;
            //saves spriteData section
            renderMap.set(id,data.spriteArray);
            //draw
            createTile(data.spriteArray,x,y);
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
            if(data.fail) return;
            //add to animation bank 
            var anim = new Animation(id);
            if(!anim.fail) animationArray.push(anim);

            //todo: save entire object data without spriteData

            //saves spriteData section
            renderMap.set(id,data.spriteArray);
            //draw
            createTile(data.spriteArray,x,y);
        }else{
            //trace("group " + data);
        }
    }
    public function createTile(array:Vector<SpriteData>,x:Int,y:Int):Group
    {
        var group = new Group();
        objectMap.set(x + "-" + y, group);
        //shift to pos
        x += -setX;
        y += -setY;
        //set to grid
        x *= Static.GRID;
        y *= Static.GRID;
        //add array tiles
        var length:Int = numTiles;
        for(obj in array)
        {
            var cache = cacheObject(obj.spriteID);
            var rect = tileset.getRect(cache);
            var tile = new Tile(cache,Object);

            if (obj.rot > 0)
            {
                tile.rotation = obj.rot * 360;
            }
            if (obj.hFlip != 0)
            {
                tile.scaleX = obj.hFlip;
            }
            if (obj.parent >= 0)
            {
                tile.parentID = obj.parent;
            }
            //color
            tile.colorTransform = new ColorTransform();
            tile.colorTransform.redMultiplier = obj.color[0];
            tile.colorTransform.greenMultiplier = obj.color[1];
            tile.colorTransform.blueMultiplier = obj.color[2];
            //pos
            tile.x = x + obj.pos.x - obj.inCenterXOffset * 1 - rect.width/2;
            tile.y = y + -obj.pos.y - obj.inCenterYOffset * 1 - rect.height/2;
            addTile(tile);
            group.add(tile);
        }
        return group;
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
    public function reload()
    {
        
    }
}
class Tile extends openfl.display.Tile
{
    public var type:TileType;
    public var parentID:Int = 0;
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
}
class Group
{
    public var children:Array<Tile> = [];
    @:isVar public var alpha(default,set):Float = 0;
    function set_alpha(value:Float):Float
    {
        alpha = value;
        for(child in children)
        {
            child.alpha = alpha;
        }
        return alpha;
    }
    function get_alpha():Float
    {
        return alpha;
    }
    @:isVar public var x(default,set):Float = 0;
    function set_x(value:Float):Float
    {
        var change = value - x;
        for(child in children)
        {
            child.x += change;
        }
        return x = value;
    }
    @:isVar public var y(default,set):Float = 0;
    function set_y(value:Float):Float
    {
        var change = value - y;
        for (child in children)
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
        return children.push(child);
    }
    public function remove(child:Tile)
    {
        children.remove(child);
    }
}