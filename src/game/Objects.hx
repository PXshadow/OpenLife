package game;
#if openfl
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
    public var cacheMap:Map<Int,Int> = new Map<Int,Int>();
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
    //scale used for zoom in and out
    public var scale(get, set):Float;
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
        player = Main.data.playerMap.get(data.p_id);
        if (player == null)
        {
            //new
            player = new Player();
            add([data.po_id],data.x,data.y,player);
            group.addTile(player);
            player.set(data);
            player.force();
            Main.data.playerMap.set(data.p_id,player);
            
        }else{
            //exists
            player.set(data);
        }
    }
    public function visibleSprites(id:Int,sprites:Array<Tile>,age:Int=20)
    {
        var data = Main.data.objectMap.get(id);
        if (data != null)
        {
            for (i in 0...sprites.length)
            {
                sprites[i].visible = true;
                if (data.useVanishIndex[i] == -1 && data.numUses > 1) sprites[i].visible = false;
                if ((data.spriteArray[i].ageRange[0] > -1 || data.spriteArray[i].ageRange[1] > -1) && (data.spriteArray[i].ageRange[0] > age || data.spriteArray[i].ageRange[1] < age)) sprites[i].visible = false;
            }
        }
    }
    public function add(array:Array<Int>,x:Int=0,y:Int=0,container:TileContainer=null):Bool
    {
        if (array == null || array.length == 0 || array[0] <= 0) return false;
        var data:ObjectData = Main.data.objectMap.get(array[0]);
        if (data == null)
        {
            trace("add fail id: " + array[0]);
            return false;
        }
        //blocking
        if (data.blocksWalking)
        {
            Main.data.blocking.set(x + "." + y,true);
        }else{
            Main.data.blocking.remove(x + "." + y);
        }
        //tile position
        var tx:Float = x * Static.GRID;
        var ty:Float = (Static.tileHeight - y) * Static.GRID;
        //create
        var sprites:Array<Tile> = [];
        if (container == null)
        {
            sprites = create(data,tx,ty);
        }else{
            sprites = create(data,0,0);
        }
        //conainted
        for (i in 1...array.length) 
        {

        }
        //fill container if present
        if (container != null)
        {
            container.addTiles(sprites);
        }else{
            group.addTiles(sprites);
        }
        //push data
        if (container == null)
        {
            if (data.floor)
            {
                Main.data.tileData.floor.set(x,y,sprites);
            }else{
                Main.data.tileData.object.set(x,y,sprites);
            }
        }else{
            //age system
            visibleSprites(array[0],sprites,20);
        }
        return true;
    }
    private function create(data:ObjectData,x:Float=0,y:Float=0):Array<Tile>
    {
        var sprite:Tile = null;
        var r:Rectangle;
        var sprites:Array<Tile> = [];
        for (i in 0...data.numSprites)
        {
            sprite = new Tile();
            sprite.id = cacheSprite(data.spriteArray[i].spriteID);
            r = tileset.getRect(sprite.id);
            //rotation
            sprite.rotation = data.spriteArray[i].rot * 365;
            //flip
            if (data.spriteArray[i].hFlip != 0) sprite.scaleX = data.spriteArray[i].hFlip;
            //pos
            //trace("width " + r.width + " height " + r.height);
            sprite.originX = r.width/2 + data.spriteArray[i].inCenterXOffset;
            sprite.originY = r.height/2 + data.spriteArray[i].inCenterYOffset;
            sprite.x = data.spriteArray[i].pos.x;
            sprite.y = -data.spriteArray[i].pos.y;
            //color
            sprite.colorTransform = new ColorTransform();
            sprite.colorTransform.redMultiplier = data.spriteArray[i].color[0];
            sprite.colorTransform.greenMultiplier = data.spriteArray[i].color[1];
            sprite.colorTransform.blueMultiplier = data.spriteArray[i].color[2];
            //offset
            sprite.x += x;
            sprite.y += y;
            //array
            sprites.push(sprite);
        }
        return sprites;
    }
    public function clear()
    {
        Main.data.tileData.object.clear();
        Main.data.tileData.floor.clear();
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
#end