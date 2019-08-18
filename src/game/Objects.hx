package game;
import data.GameData;
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

class Objects extends TileDisplay
{
    public var object:TileContainer;
    var cacheMap:Map<Int,Int> = new Map<Int,Int>();
    var objectMap:Map<Int,ObjectData> = new Map<Int,ObjectData>();
    var animationMap:Map<Int,AnimationData> = new Map<Int,AnimationData>();
    //for tileset
    var tileX:Float = 0;
    var tileY:Float = 0;
    //ground
    public var numGround:Int = 0;
    //last player to be loaded in 
    public var player:Player = null;
    //used for reading
    var tileHeight:Int = 0;
    public var velocityX:Float = 0;
    public var velocityY:Float = 0;
    public var setX:Float = 0;
    public var setY:Float = 0;
    public var group:TileContainer;
    public var data:GameData = null;
    //scale used for zoom in and out
    public var scale(get, set):Float;
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
    public function new()
    {
        smoothing = true;
        super(4000,4000);
        //add base
        group = new TileContainer();
        addTile(group);
    }
    public function addPlayer(data:PlayerInstance)
    {
        player = this.data.playerMap.get(data.p_id);
        if (player == null)
        {
            //new
            add(data.po_id,data.x,data.y,true);
            player = cast object;
            this.data.playerMap.set(data.p_id,player);
            player.set(data);
            player.pos();
            return;
        }else{
            //exists
            player.set(data);
        }
    }
    public function getObjectData(id:Int):ObjectData
    {
        var data = objectMap.get(id);
        if (data == null)
        {
            //create
            data = new ObjectData(id);
            objectMap.set(id,data);
        }
        return data;
    }
    public function add(id:Int,x:Int=0,y:Int=0,container:Bool=false,push:Bool=true):Bool
    {
        if(id <= 0) return false;
        var data = getObjectData(id);
        object = null;
        //data
        if (data.blocksWalking == 1)
        {
            this.data.blocking.set(x + "." + y,true);
        }else{
            this.data.blocking.remove(x + "." + y);
        }
        //create new objects
        if (container) object = new TileContainer();
        if(data.person > 0)
        {
            object = new Player(this.data);
            container = true;
        }
        if (container)
        {
            group.addTileAt(object,0);
            //set local position
            object.x = (x) * Static.GRID;
            object.y = (Static.tileHeight - y) * Static.GRID;
        }
        if (!this.data.map.loaded)
        {
            //add data into map data if not loaded in (for testing)
            this.data.map.object.set(x,y,id);
        }
        var r:Rectangle;
        var sprite:Tile = null;
        var sprites:Array<Tile> = [];
        for(i in 0...data.numSprites)
        {
            sprite = new Tile();
            sprite.id = cacheSprite(data.spriteArray[i].spriteID);
            //check if cache sprite fail
            if (sprite.id == -1) 
            {
                //trace("cache sprite fail");
                continue;
            }
            r = tileset.getRect(sprite.id);
            //todo setup inCenterOffset
            //rot
            if (data.spriteArray[i].rot > 0)
            {
                //object.rotation = data.spriteArray[i].rot * 365;
            }
            //flip
            if (data.spriteArray[i].hFlip != 0)
            {
                sprite.scaleX = data.spriteArray[i].hFlip;
            }
            //pos
            sprite.x = data.spriteArray[i].pos.x - data.spriteArray[i].inCenterXOffset * 1 - r.width/2;
            sprite.y = -data.spriteArray[i].pos.y - data.spriteArray[i].inCenterYOffset * 1 - r.height/2;
            //color
            sprite.colorTransform = new ColorTransform();
            sprite.colorTransform.redMultiplier = data.spriteArray[i].color[0];
            sprite.colorTransform.greenMultiplier = data.spriteArray[i].color[1];
            sprite.colorTransform.blueMultiplier = data.spriteArray[i].color[2];
            if(data.person > 0)
            {
                //player data set
                cast(object,Player).ageRange[i] = {min:data.spriteArray[i].ageRange[0],max:data.spriteArray[i].ageRange[1]};
            }
            if (container)
            {
                //parent
                if (push) object.addTile(sprite);
            }else{
                //group
                if (push) 
                {
                    group.addTileAt(sprite,0);
                    sprites.push(sprite);
                }
                sprite.x += x * Static.GRID;
                sprite.y += (Static.tileHeight - y) * Static.GRID;
            }
        }
        if (push)
        {
            if (data.floor == 0)
            {
                this.data.tileData.object.set(x,y,sprites);
            }else{
                this.data.tileData.floor.set(x,y,sprites);
            }
        }
        return true;
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