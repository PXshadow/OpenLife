package states.game;
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
    var clean:Bool = false;
    var cleanArray:Array<Tile> = [];
    var tileX:Int = 0;
    var tileY:Int = 0;
    public function new(game:Game)
    {
        this.game = game;
        super(3200,3200);
    }
    //when map has changed
    public function update()
    {
        clean = false;
        if (ox != game.tileX || oy != game.tileY)
        {
            ox = game.tileX;
            oy = game.tileY;
            clean = true;
        }
        for(i in 0...numTiles)
        {
            tile = getTileAt(i);
            tile.x += x;
            tile.y += y;
            if (clean) if (tile.x > width || tile.x < -Static.GRID || tile.y > height || tile.y < -Static.GRID)
            {
                cleanArray.push(tile);
            }
        }
        clean = false;
        for (tile in cleanArray) removeTile(tile);
        //reset pos
        x = 0;
        y = 0;
    }
    private function cleanAction()
    {
        for(tile in cleanArray)
        {
            removeTile(tile);
        }
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
        var player:Player;
        player = 
        player = cast add(data.po_id,data.x - game.tileX,data.y - game.tileY,true);
        player.set(data);
        game.data.playerMap.set(data.p_id,player);
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
            obj = new Player();
        }else{
            obj = new Object();
        }
        obj.x = x * Static.GRID * 1;
        obj.y = y * Static.GRID * 1;
        addTile(obj);
        var r:Rectangle;
        for(i in 0...data.numSprites)
        {
            var tile = new Tile();
            tile.id = cacheSprite(data.spriteArray[i].spriteID);
            r = tileset.getRect(tile.id);
            //todo setup inCenterOffset
            //rot
            if (data.spriteArray[i].rot > 0)
            {
                trace("rotation " + data.description + " value " + data.spriteArray[i].rot);
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
        return obj;
    }
    private function cacheSprite(id:Int):Int
    {
        if(cacheMap.exists(id))
        {
            return cacheMap.get(id);
        }
        reader.read(File.read(Static.dir + "assets/sprites/" + id + ".tga").readAll());
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