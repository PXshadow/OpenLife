package game;
import resources.Resource;
import data.object.player.PlayerInstance;
import data.animation.AnimationPlayer;
import data.object.SpriteData;
#if openfl
import openfl.display.BitmapData;
import openfl.display.TileContainer;
import lime.utils.ObjectPool;
import openfl.display.Tileset;
import haxe.io.Path;
import openfl.utils.ByteArray;
import haxe.ds.Vector;
import data.animation.AnimationData;
import openfl.geom.ColorTransform;
import openfl.geom.Rectangle;
import graphics.TgaData;
import openfl.display.Tile;
import data.object.ObjectData;
#if nativeGen @:nativeGen #end
class Objects extends TileDisplay
{
    public var animate:AnimationPlayer;
    public var containing:Int = 0;
    public var sprites:Array<Tile> = [];
    public var cacheMap:Map<Int,Int> = new Map<Int,Int>();
    //for tileset
    public var tileX:Float = 0;
    public var tileY:Float = 0;
    //last player to be loaded in
    public var player:game.Player = null;
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
        animate = new AnimationPlayer(this);
        smoothing = true;
        //alpha = 0.5;
        //trace(list);
        //add base
        group = new TileContainer();
        //group.x = Static.GRID * 32/2;
        //group.y = Static.GRID * 30/2;
        addTile(group);
    }
    public function addPlayer(data:data.object.player.PlayerInstance)
    {
        if (data == null) return;
        player = Game.data.playerMap.get(data.p_id);
        if (player == null)
        {
            //new
            player = new game.Player();
            //tileHeight - tileHeight = 0 for Y
            add([data.po_id],0,data.y,player);
            //set to very front
            group.addTile(player);
            player.objects = this;
            player.set(data);
            player.force();
            Game.data.playerMap.set(data.p_id,player); 
        }else{
            //exists
            player.set(data);
        }
    }
    public function visibleSprites(id:Int,sprites:Array<Tile>,age:Int=20)
    {
        var data = get(id);
        if (data != null)
        {
            for (i in 0...sprites.length)
            {
                sprites[i].visible = true;
                if (data.useVanishIndex[i] == -1 && data.numUses > 1) sprites[i].visible = false;
                if ((data.spriteArray[i].ageRange[0] > -1 || data.spriteArray[i].ageRange[1] > -1) && (data.spriteArray[i].ageRange[0] >= age || data.spriteArray[i].ageRange[1] < age)) sprites[i].visible = false;
            }
        }
    }
    public function remove(x:Int,y:Int,floor:Bool=false)
    {
        var tiles:Array<Tile> = null;
        if (floor)
        {
            tiles = Game.data.tileData.floor.get(x,y);
            Game.data.map.floor.set(x,y,0);
            Game.data.tileData.floor.set(x,y,[]);
        }else{
            tiles = Game.data.tileData.object.get(x,y);
            Game.data.map.object.set(x,y,[]);
            Game.data.tileData.object.set(x,y,[]);
        }
        if (tiles != null) for (tile in tiles) group.removeTile(tile);
    }
    public function get(id:Int):ObjectData
    {
        var data = Game.data.objectMap.get(id);
        if (data == null)
        {
            //retrieve from file
            data = new ObjectData(id);
            Game.data.objectMap.set(id,data);
        }
        return data;
    }
    public function add(array:Array<Int>,x:Int=0,y:Int=0,container:TileContainer=null):Bool
    {
        if (array == null || array.length == 0 || array[0] == 0) return false;
        //data is main container
        var data:ObjectData = get(array[0]);
        var main = data;
        var sub:ObjectData;
        //container int
        var conInt:Int = 0;
        var mainInt:Int = 0;
        //sub is used for all sub data props
        var sub:ObjectData = data;
        if (data == null)
        {
            trace("failed object id: " + array[0]);
            return false;
        }
        //blocking
        if (data.blocksWalking)
        {
            Game.data.blocking.set(x + "." + y,true);
        }else{
            Game.data.blocking.remove(x + "." + y);
        }
        //tile position
        var tx:Float = x * Static.GRID;
        var ty:Float = (Static.tileHeight - y) * Static.GRID;
        //create
        //var sprites:Array<Tile> = [];
        if (container != null)
        {
            tx = 0;
            ty = 0;
        }
        sprites = create(data,tx + 0,ty - 0);
        //age system
        if (container != null && data.person == 0) visibleSprites(array[0],sprites,20);
        if (sprites.length == 0) trace(data.id);
        //conainted
        var id:Int = 0;
        //id of container
        var set:Bool = false;
        for (i in 1...array.length) 
        {
            set = false;
            if (array[i] > 0)
            {
                //container
                sub = get(array[i] * 1);
                id = mainInt++;
                conInt = 0;
                set = true;
                data = main;
            }else{
                //add into container
                sub = get(array[i] * -1);
                id = conInt++;
            }
            if (data.slotPos[id] == null || data.slotVert == null || sub == null || data.slotPos == null) continue;
            for (sprite in create(sub,
            tx + data.slotPos[id].x + Static.GRID/4,
            ty - data.slotPos[id].y - Static.GRID/4 * 0,
            sub.vertSlotRot * 365 + (data.slotVert[id] ? 1 : 0) * 90)
            )
            {
                sprites.insert(data.slotParent[id],sprite);
            }
            if (set) data = sub;
        }
        //animation
        if (data.animation != null && data.person == 0)
        {
            animate.play(data.id,0,sprites,x,y);
        }
        //fill container if present
        if (container != null)
        {
            container.addTiles(sprites);
        }else{
            group.addTiles(sprites);
            //for (sprite in sprites) group.addTileAt(sprite,0);
            //data set
            if (data.floor)
            {
                Game.data.tileData.floor.set(x,y,sprites);
                //sprites[0].data = {};
            }else{
                Game.data.tileData.object.set(x,y,sprites);
            }
        }
        return true;
    }
    public function getObjectHeight(id:Int):Int
    {
        var obj:ObjectData = get(id);
        if (obj == null) return 0;
        if (obj.cacheHeight > 0) return obj.cacheHeight;
        var sprite:SpriteData;
        var rect:Rectangle;
        var height:Int;
        for (i in 0...obj.numSprites)
        {
            sprite = obj.spriteArray[i];
            rect = tileset.getRect(cacheSprite(sprite.spriteID));
            height = Std.int(sprite.pos.y - rect.height/2 - sprite.inCenterYOffset);
            if (height > obj.cacheHeight) obj.cacheHeight = height;
        }
        return 0;
    }
    public function create(data:ObjectData,x:Float=0,y:Float=0,rotation:Float=0,worn:Bool=false,held:Bool=false,inDrawBehindSlots:Int=2):Array<Tile>
    {
        var sprite:Tile = null;
        var sprites:Array<Tile> = [];
        for (i in 0...data.numSprites)
        {
            sprite = new Tile();
            sprite.data = {floor:data.floor};
            sprite.id = cacheSprite(data.spriteArray[i].spriteID);
            setSprite(sprite,data.spriteArray[i],x,y);
            //worn
            if (data.clothing != "n" && data.spriteArray[i].invisWorn != 0)
            {
                if (worn && data.spriteArray[i].invisWorn == 1)
                {
                    sprite.visible = false;
                }else if (!worn && data.spriteArray[i].invisWorn == 2)
                {
                    sprite.visible = false;
                }
            }
            //draw behind slots
            if (inDrawBehindSlots != 2)
            {
                if (inDrawBehindSlots == 0 && !data.spriteArray[i].behindSlots)
                {
                    sprite.visible = false;
                }else if (inDrawBehindSlots == 1 && data.spriteArray[i].behindSlots)
                {
                    sprite.visible = false;
                }
            }
            sprites.push(sprite);
            //sprites.unshift(sprite);
        }
        //rotation change
        if (rotation > 0)
        {
            for (i in 0...sprites.length)
            {
                sprite = sprites[i];
                sprite.matrix.translate(-x,-y);
                sprite.matrix.rotate((rotation) * (Math.PI/180));
                sprite.matrix.translate(x,y);
                sprite.rotation += rotation;
            }
        }
        return sprites;
    }
    /**
     * sort tiles when they move
     */
    public function sort(object:Tile,oldX:Int,oldY:Int)
    {

    }
    public function setSprite(sprite:Tile,data:SpriteData,x:Float,y:Float)
    {
        if (data == null) return;
        var r = tileset.getRect(sprite.id);
        if (r == null) return;
        //center
        sprite.originX = r.width/2 + data.inCenterXOffset;
        sprite.originY = r.height/2 + data.inCenterYOffset;
        //pos offset
        sprite.x = data.pos.x;
        sprite.y = -data.pos.y;
        //grid offset
        sprite.x += x;
        sprite.y += y;
        //color
        sprite.colorTransform = new ColorTransform();
        sprite.colorTransform.redMultiplier = data.color[0];
        sprite.colorTransform.greenMultiplier = data.color[1];
        sprite.colorTransform.blueMultiplier = data.color[2];
        //rotation
        sprite.rotation = data.rot * 365;
        //flip
        if (data.hFlip != 0) sprite.scaleX = data.hFlip * -1;
    }
    /**
     * Set this up to postion clothing
     * @param sprite 
     * @param data 
     * @param parent 
     */
    public function setClothing(sprite:Tile,data:SpriteData,parent:Tile)
    {

    }
    public function clear()
    {
        Game.data.tileData.object.clear();
        Game.data.tileData.floor.clear();
        player = null;
        group.removeTiles();
    }
    public function cacheSprite(id:Int):Int
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
    var spacing:Int = 0;
    //fit rectangle within spacing
    private function drawSprite(id:Int,rect:Rectangle):Rectangle
    {
        /*try {
            reader.read(File.getBytes(Game.dir + "sprites/" + id + ".tga"),false);
        }catch(e:Dynamic)
        {
            //trace("e " + e);
            return null;
        }*/
        reader.read(Resource.spriteImage(id));
        reader.crop();
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