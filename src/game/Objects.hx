package game;
import openfl.display.BitmapData;
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
    public var containing:Int = 0;
    public var sprites:Array<Tile> = [];
    public var object:TileContainer;
    public var cacheMap:Map<Int,Int> = new Map<Int,Int>();
    public var objectMap:Map<Int,ObjectData> = new Map<Int,ObjectData>();
    //for tileset
    public var tileX:Float = 0;
    public var tileY:Float = 0;
    //last player to be loaded in 
    public var player:Player = null;
    //used for reading
    public var tileHeight:Int = 0;
    public var velocityX:Float = 0;
    public var velocityY:Float = 0;
    public var setX:Float = 0;
    public var setY:Float = 0;
    public var group:TileContainer;
    public var data:GameData = null;
    //scale used for zoom in and out
    public var scale(get, set):Float;
    public var range:Int = 16;
    //clear
    public var clearBool:Bool = false;
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
        super();
        //trace(list);
        //add base
        group = new TileContainer();
        addTile(group);
    }
    public function addPlayer(data:PlayerInstance)
    {
        if (data == null) return;
        player = this.data.playerMap.get(data.p_id);
        if (player == null)
        {
            //new
            add(data.po_id,data.x,data.y,true,false);
            player = cast object;
            if (player == null) return;
            trace("player " + player);
            this.data.playerMap.set(data.p_id,player);
            player.set(data);
            player.force();
            return;
        }else{
            //exists
            player.set(data);
        }
    }
    public function add(id:Int,x:Int=0,y:Int=0,container:Bool=false,push:Bool=true,index:Int=0):Bool
    {
        //return false;
        if(id <= 0) return false;
        //trace("unit test");
        UnitTest.inital();
        //trace("inital");
        var data = objectMap.get(id);
        if (data == null) return false;
        //data
        if (data.blocksWalking == 1)
        {
            this.data.blocking.set(x + "." + y,true);
        }else{
            this.data.blocking.remove(x + "." + y);
        }
        //create new objects
        if (data.containable == 1) container = true;
        if (containing == 0) object = null;
        if (container && containing == 0) object = new TileContainer();
        //moving object
        //if (data.)
        if(data.person > 0)
        {
            object = new Player(this.data,this);
            container = true;
            push = false;
        }
        if (container && containing == 0)
        {
            //set local position
            object.x = (x) * Static.GRID;
            object.y = (Static.tileHeight - y) * Static.GRID;
            group.addTileAt(object,0);
        }
        if (!this.data.map.loaded)
        {
            //add data into map data if not loaded in (for testing)
            this.data.map.object.set(x,y,[id]);
        }
        var r:Rectangle;
        var sprite:Tile = null;
        sprites = [];
        var parents:Map<Int,TileContainer> = new Map<Int,TileContainer>();
        //trace("inital " + UnitTest.stamp());
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
                sprite.rotation = data.spriteArray[i].rot * 365;
            }
            //flip
            if (data.spriteArray[i].hFlip != 0)
            {
                sprite.scaleX = data.spriteArray[i].hFlip;
            }
            //pos
            sprite.originX = r.width/2 + data.spriteArray[i].inCenterXOffset;
            sprite.originY = r.height/2 + data.spriteArray[i].inCenterYOffset;
            sprite.x = data.spriteArray[i].pos.x;
            sprite.y = -data.spriteArray[i].pos.y;
            //color
            sprite.colorTransform = new ColorTransform();
            sprite.colorTransform.redMultiplier = data.spriteArray[i].color[0];
            sprite.colorTransform.greenMultiplier = data.spriteArray[i].color[1];
            sprite.colorTransform.blueMultiplier = data.spriteArray[i].color[2];
            //rotation
            sprite.rotation = data.spriteArray[i].rot * 365;
            if(data.person > 0)
            {
                //player data set
                cast(object,Player).ageRange[i] = {min:data.spriteArray[i].ageRange[0],max:data.spriteArray[i].ageRange[1]};
            }
            //parent system
            if (data.spriteArray[i].parent > -1)
            {
                var p = parents.get(data.spriteArray[i].parent);
                if (p == null)
                {
                    p = new TileContainer();
                    parents.set(data.spriteArray[i].parent,p);
                    if (container)
                    {
                        object.addTile(p);
                    }else{
                        p.x = x * Static.GRID;
                        p.y = (Static.tileHeight - y) * Static.GRID;
                        //group.addTileAt(p,0);
                        group.addTile(p);
                    }
                }
                p.addTile(sprite);
                if (i == data.spriteArray[i].parent)
                {
                    sprites.push(p);
                }else{
                    sprites.push(sprite);
                }
            }else{
                if (container)
                {
                    if (containing > 0)
                    {
                        //pos
                        var pos = objectMap.get(containing).slotPos[index];
                        sprite.x += pos.x;
                        sprite.y += pos.y;
                    }
                    object.addTile(sprite);
                    sprites.push(sprite);
                }else{
                    sprite.x += x * Static.GRID;
                    sprite.y += (Static.tileHeight - y) * Static.GRID;
                    group.addTile(sprite);
                    //group.addTileAt(sprite,0);
                    sprites.push(sprite);
                }
            }
        }
        //trace("for " + UnitTest.stamp() + " person " + data.person + " container " + container);
        //person
        if (data.person > 0)
        {
            cast(object,Player).sprites = sprites;
        }
        //finish for loop, push data into tileData
        if (push)
        {
            if (container) sprites = [object];
            if (data.floor == 0)
            {
                this.data.tileData.object.set(x,y,sprites);
            }else{
                this.data.tileData.floor.set(x,y,sprites);
            }
        }
        //trace("add " + UnitTest.stamp());
        return true;
    }
    public function clear()
    {
        data.tileData.object.clear();
        data.tileData.floor.clear();
        group.removeTiles();
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
        tileX += Math.ceil(rect.width) + 2;
        //set to bitmapData
        tileset.bitmapData.setPixels(rect,reader.bytes);
        if (rect.height > tileHeight) tileHeight = Math.ceil(rect.height) + 2;
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