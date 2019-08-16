package states.game;
import openfl.display.TileContainer;
import lime.utils.ObjectPool;
import openfl.display.Tileset;
import haxe.io.Path;
import openfl.utils.ByteArray;
import haxe.ds.Vector;
import sys.FileSystem;
import data.PlayerData.PlayerInstance;
import sys.io.File;
import data.AnimationData;
import openfl.geom.ColorTransform;
import openfl.geom.Rectangle;
import data.TgaData;
import openfl.display.Tile;
import data.ObjectData;
import states.launcher.Launcher;

class Objects extends TileDisplay
{
    public var objectPool:ObjectPool<Object>;
    public var groundPool:ObjectPool<Tile>;
    //game ref
    var game:Game;
    var tile:Tile;
    var cacheMap:Map<Int,Int> = new Map<Int,Int>();
    //for tileset
    var tileX:Float = 0;
    var tileY:Float = 0;
    //ground
    public var numGround:Int = 0;
    //last player to be loaded in 
    public var player:Player = null;
    //used for reading
    var tileHeight:Int = 0;
    //scale used for zoom in and out
    public var scale(get, set):Float;
    public var velocityX:Float = 0;
    public var velocityY:Float = 0;
    public var setX:Float = 0;
    public var setY:Float = 0;
    public var group:TileContainer;
    function get_scale():Float 
    {
        return group.scaleX;
    }
    function set_scale(scale:Float):Float 
    {
        group.scaleX = scale;
        group.scaleY = scale;
        return scale;
    }
    public function new(game:Game)
    {
        this.game = game;
        smoothing = true;
        super(4096,4096);
        //add base
        group = new TileContainer();
        addTile(group);

        //add cached ground
        for (i in 0...6 + 1) cacheGround(i);
        //pools
        objectPool = new ObjectPool(function():Object
        {
            //object or floor
            var tile = new Object();
            group.addTile(tile);
            return tile;
        },function(tile:Object)
        {
            tile.removeTiles();
            tile.visible = false;
            removeTile(tile);
        });
        groundPool = new ObjectPool(function():Tile
        {
            //ground
            return new Tile();
        },function(tile:Tile)
        {
            tile.visible = false;
            removeTile(tile);
        });
    }
    //camera controls
    public function move(x:Float,y:Float)
    {
        group.x += x;
        group.y += y;
    }
    public function set(x:Float,y:Float)
    {
        group.x = x;
        group.y = y;
    }
    //cache ground tiles
    public function cacheGround(id:Int)
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
    public function addGround(id:Int,x:Int,y:Int,add:Int):Tile
    {
        var tile = groundPool.get();
        tile.id = id * 16 + ci(x) + ci(y) * 3;
        tile.data = {type:GROUND,x:x,y:y};
        tile.x = tile.data.x * Static.GRID;
        tile.y = (Static.tileHeight - tile.data.y) * Static.GRID;
        group.addTileAt(tile,add);
        return tile;
    }
    public function addFloor(id:Int,x:Int=0,y:Int=0):Bool
    {
        return add(id,x,y,false,true) != null ? true : false;
    }
    public function addObject(id:Int,x:Int=0,y:Int=0):Tile
    {
        //single object
        return add(id,x,y,false,false);
    }
    public function addPlayer(data:PlayerInstance)
    {
        player = game.data.playerMap.get(data.p_id);
        if (player == null)
        {
            //new
            player = cast add(data.po_id,data.x,data.y,true);
            game.data.playerMap.set(data.p_id,player);
            player.set(data);
            player.pos();
            return;
        }else{
            //exists
            player.set(data);
        }
    }
    public function add(id:Int,x:Int=0,y:Int=0,player:Bool=false,floor:Bool=false):Object
    {
        if(id == 0) return null;
        var data = new ObjectData(id);
        var obj:Object = null;
        //data
        if (data.blocksWalking == 1)
        {
            game.data.blocking.set(x + "." + y,true);
        }else{
            game.data.blocking.remove(x + "." + y);
        }
        //obj
        if(player)
        {
            obj = new Player(game);
            group.addTile(obj);
        }else{
            obj = objectPool.get();
            //global pos
            obj.data.x = x;
            obj.data.y = y;
            //local position
            obj.x = (obj.data.x) * Static.GRID;
            obj.y = (Static.tileHeight - (obj.data.y)) * Static.GRID;
        }
        if (floor) obj.data.type = FLOOR;
        obj.oid = data.id;
        //animation setup
        obj.loadAnimation();
        //add data into map data if not loaded in
        if (!game.data.map.loaded && !player)
        {
            game.data.map.object.set(obj.data.tileX,obj.data.tileY,obj.oid);
        }
        var r:Rectangle;
        var parents:Array<Int> = [];
        for(i in 0...data.numSprites)
        {
            var tile = new Tile();
            tile.id = cacheSprite(data.spriteArray[i].spriteID);
            //check if cache sprite fail
            if (tile.id == -1) 
            {
                //trace("cache sprite fail");
                continue;
            }
            r = tileset.getRect(tile.id);
            //todo setup inCenterOffset
            //rot
            if (data.spriteArray[i].rot > 0)
            {
                //tile.rotation = data.spriteArray[i].rot * 365;
            }
            //flip
            if (data.spriteArray[i].hFlip != 0)
            {
                tile.scaleX = data.spriteArray[i].hFlip;
            }
            //pos
            tile.x = data.spriteArray[i].pos.x - data.spriteArray[i].inCenterXOffset * 1 - r.width/2;
            tile.y = -data.spriteArray[i].pos.y - data.spriteArray[i].inCenterYOffset * 1 - r.height/2;
            //color
            tile.colorTransform = new ColorTransform();
            tile.colorTransform.redMultiplier = data.spriteArray[i].color[0];
            tile.colorTransform.greenMultiplier = data.spriteArray[i].color[1];
            tile.colorTransform.blueMultiplier = data.spriteArray[i].color[2];
            //parent
            obj.add(tile,i,data.spriteArray[i].parent);
            if(player)
            {
                //player data set
                cast(obj,Player).ageRange[i] = {min:data.spriteArray[i].ageRange[0],max:data.spriteArray[i].ageRange[1]};
            }
        }
        obj.animate(0);
        return obj;
    }
    private function cacheSprite(id:Int):Int
    {
        if(cacheMap.exists(id))
        {
            return cacheMap.get(id);
        }
        //add tileset rect
        var rect = drawSprite(id,new Rectangle(tileX,tileY,0,0));
        if (rect == null) return -1;
        var i = tileset.addRect(rect);
        cacheMap.set(id,i);
        return i;
    }
    private function drawSprite(id:Int,rect:Rectangle):Rectangle
    {
        try {
            reader.read(File.read(Static.dir + "sprites/" + id + ".tga").readAll());
        }catch(e:Dynamic)
        {
            //trace("e " + e);
            return null;
        }
        //set dimensions
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
        if (rect.height > tileHeight) tileHeight = Std.int(rect.height);
        return rect;
    }
    public function reload()
    {
        var id:Int = 0;
        var rect:Rectangle;
        var keys = cacheMap.keys();
        while(keys.hasNext())
        {
            id = keys.next();
            rect = tileset.getRect(cacheMap.get(id));
            tileset.bitmapData.fillRect(rect,0x00FFFFFF);
            drawSprite(id,rect);
        }
    }
}