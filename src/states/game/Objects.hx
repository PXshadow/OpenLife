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
    //game ref
    var game:Game;
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
    public function addGround(id:Int,x:Int,y:Int):Tile
    {
        var object = new Tile();
        object.id = id * 16 + ci(x) + ci(y) * 3;
        object.data = {type:GROUND,x:x,y:y};
        object.x = object.data.x * Static.GRID - Static.GRID/2;
        object.y = (Static.tileHeight - object.data.y) * Static.GRID - Static.GRID/2;
        group.addTileAt(object,0);
        //add to chunk
        game.data.chunk.latest.ground.set(x,y,object);
        return object;
    }
    public function addPlayer(data:PlayerInstance)
    {
        player = game.data.playerMap.get(data.p_id);
        if (player == null)
        {
            //new
            add(data.po_id,data.x,data.y,true);
            player = cast object;
            game.data.playerMap.set(data.p_id,player);
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
    public function add(id:Int,x:Int=0,y:Int=0,container:Bool=false,chunk:Bool=true):Bool
    {
        if(id <= 0) return false;
        var data = getObjectData(id);
        object = null;
        //data
        if (data.blocksWalking == 1)
        {
            game.data.blocking.set(x + "." + y,true);
        }else{
            game.data.blocking.remove(x + "." + y);
        }
        //create new objects
        if (container) object = new TileContainer();
        if(data.person > 0)
        {
            object = new Player(game);
            container = true;
        }
        if (container)
        {
            group.addTile(object);
            //set local position
            object.x = (x) * Static.GRID;
            object.y = (Static.tileHeight - y) * Static.GRID;
        }
        if (!game.data.map.loaded)
        {
            //add data into map data if not loaded in (for testing)
            game.data.map.object.set(x,y,id);
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
                if (chunk) object.addTile(sprite);
            }else{
                //group
                if (chunk) 
                {
                    group.addTile(sprite);
                    sprites.push(sprite);
                }
                sprite.x += x * Static.GRID;
                sprite.y += (Static.tileHeight - y) * Static.GRID;
            }
        }
        //chunk bool loads in the elements into a chunk
        if (chunk)
        {
            //add to chunk
            if (data.person == 0)
            {
                if (container)
                {
                    sprites = [object];
                }
                if (data.floor == 1)
                {
                    game.data.chunk.latest.floor.set(x,y,sprites);
                }else{
                    game.data.chunk.latest.object.set(x,y,sprites);
                }
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