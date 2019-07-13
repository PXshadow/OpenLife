package states.game;
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
    var game:Game;
    //old pos
    var ox:Int = 0;
    var oy:Int = 0;
    var tile:Tile;
    var cacheMap:Map<Int,Int> = new Map<Int,Int>();
    var tileX:Int = 0;
    var tileY:Int = 0;
    public var player:Player;
    public function new(game:Game)
    {
        this.game = game;
        super(3200,3200);
    }
    //when map has changed
    public function update()
    {
        for(i in 0...numTiles)
        {
            tile = getTileAt(i);
            tile.x += x;
            tile.y += y;
            //if (tile.x > width || tile.x < -Static.GRID || tile.y > height || tile.y < -Static.GRID)
        }
        //reset pos
        x = 0;
        y = 0;
    }
    public function addFloor(id:Int,x:Int,y:Int)
    {
        add(id,x,y);
    }
    public function addObject(string:String,x:Int,y:Int)
    {
        var id:Null<Int> = Std.parseInt(string);
        if (id != null)
        {
            //single object
            add(id,x,y);
        }else{
            //group
        }
    }
    public function addPlayer(data:PlayerInstance)
    {
        player = game.data.playerMap.get(data.p_id);
        if (player == null)
        {
            //new
            player = cast add(data.po_id,0,0,true);
            game.data.playerMap.set(data.p_id,player);
        }
        //set to player object
        player.set(data);
        addTile(player);
    }
    private function add(id:Int,x:Int,y:Int,player:Bool=false):Object
    {
        if(id == 0) return null;
        var data = new ObjectData(id);
        //data
        if (data.blocksWalking == 1)
        {
            game.data.blocking.set(x + "." + y,true);
        }else{
            game.data.blocking.remove(x + "." + y);
        }
        //obj
        var obj:Object;
        if(player)
        {
            obj = new Player(game);
        }else{
            obj = new Object();
        }
        obj.oid = data.id;
        obj.x = x * Static.GRID * 1;
        obj.y = y * Static.GRID * 1;
        addTile(obj);
        var r:Rectangle;
        //trace("numSprites " + data.numSprites);
        for(i in 0...data.numSprites)
        {
            var tile = new Tile();
            tile.id = cacheSprite(data.spriteArray[i].spriteID);
            //check if cache sprite fail
            if (tile.id == -1) continue;
            r = tileset.getRect(tile.id);
            //todo setup inCenterOffset
            //rot
            if (data.spriteArray[i].rot > 0)
            {
                tile.rotation = data.spriteArray[i].rot * 365;
            }
            //flip
            if (data.spriteArray[i].hFlip != 0)
            {
                tile.scaleX = data.spriteArray[i].hFlip;
            }
            //parent
            if (data.spriteArray[i].parent >= 0)
            {
                
            }
            //pos
            tile.x = data.spriteArray[i].pos.x - data.spriteArray[i].inCenterXOffset * 1 - r.width/2;
            tile.y = -data.spriteArray[i].pos.y - data.spriteArray[i].inCenterYOffset * 1 - r.height/2;
            //color
            tile.colorTransform = new ColorTransform();
            tile.colorTransform.redMultiplier = data.spriteArray[i].color[0];
            tile.colorTransform.greenMultiplier = data.spriteArray[i].color[1];
            tile.colorTransform.blueMultiplier = data.spriteArray[i].color[2];
            obj.addTile(tile);

            if(player)
            {
                //player data set
                cast(obj,Player).ageRange[i] = {min:data.spriteArray[i].ageRange[0],max:data.spriteArray[i].ageRange[1]};
                
            }
        }
        obj.animate();
        return obj;
    }
    private function cacheSprite(id:Int):Int
    {
        if(cacheMap.exists(id))
        {
            return cacheMap.get(id);
        }
        var path = Static.dir + "sprites/" + id + ".tga";
        if (!FileSystem.exists(path))
        {
            trace("cacheSprite fail " + path);
            return -1;
        }
        reader.read(File.read(path).readAll());
        var rect = new Rectangle(tileX,tileY,reader.rect.width,reader.rect.height);
        
        var color:UInt;
        var minX:Int = Std.int(rect.width) - 1;
        var minY:Int = Std.int(rect.height) - 1;
        var maxX:Int = 0;
        var maxY:Int = 0;
        for(y in 0...Std.int(rect.height))
        {
            for(x in 0...Std.int(rect.width))
            {
                color = reader.bytes.readUnsignedInt();
                if(color >> 24 & 255 == 0) continue;
                if(x < minX) minX = x;
                if (y < minY) minY = y;
                if (x > maxX) maxX = x;
                if (y > maxY) maxY = y;
            }
        }
        reader.bytes.position = 0;
        tileset.bitmapData.setPixels(rect,reader.bytes);
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