package states.game;
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
    var game:Game;
    //old pos
    var ox:Int = 0;
    var oy:Int = 0;
    var tile:Tile;
    var cacheMap:Map<Int,Int> = new Map<Int,Int>();
    var tileX:Float = 0;
    var tileY:Float = 0;
    public function new(game:Game)
    {
        this.game = game;
        smoothing = true;
        super(4096,4096);
    }
    //when map has changed
    public function update()
    {
        
    }
    public function addFloor(id:Int,x:Int,y:Int)
    {
        var floor = add(id,x,y);
        if (floor != null)
        {
            floor.type = FLOOR;
        }
    }
    public function addObject(vector:Vector<Int>=null,x:Int,y:Int)
    {
        if (vector == null) return;
        for (id in vector)
        {
            //single object
            add(id,x,y);
        }
    }
    public function sort()
    {
        @:privateAccess __group.__tiles.sort(function(a:Tile,b:Tile)
        {
            if(a.y > b.y)
            {
                return 1;
            }else{
                return -1;
            }
        });
    }
    public function add(id:Int,x:Int,y:Int,player:Bool=false):Object
    {
        if(id == 0) return null;
        var data = new ObjectData(id);
        var obj:Object = null;
        //data
        //trace("blcoking " + data.blocksWalking);
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
        }else{
            obj = new Object();
        }
        addTile(obj);
        obj.oid = data.id;
        obj.loadAnimation();
        //global cords used to refrence
        obj.tileX = x;
        obj.tileY = y;
        //add data into map data if not loaded in
        if (!game.data.map.loaded && !player)
        {
            trace("add data x " + Std.string(obj.tileX) + " y " + Std.string(obj.tileY));
            if (game.data.map.object[obj.tileY] == null) 
            {
                game.data.map.object[obj.tileY] = [];
            }
            game.data.map.object[obj.tileY][obj.tileX] = Vector.fromArrayCopy([obj.oid]);
        }
        //set to display postion
        obj.x = (obj.tileX + game.data.map.x + game.cameraX) * Static.GRID * 1;
        obj.y = (obj.tileY + game.data.map.y + game.cameraY) * Static.GRID * 1;
        trace("obj x " + obj.x + " y " + obj.y);
        var r:Rectangle;
        var parents:Array<Int> = [];
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
            trace("e " + e);
            return null;
        }
        //set dimensions
        rect.width = reader.rect.width;
        rect.height = reader.rect.height;
        //move down column
        if(rect.x + rect.width > tileset.bitmapData.width)
        {
            tileX = 0;
            tileY += tileHeight;
            rect.x = tileX;
            rect.y = tileHeight;
            tileHeight = 0;
        }
        tileset.bitmapData.setPixels(rect,reader.bytes);
        //move tilesystem
        tileX += Std.int(rect.width);
        if (rect.height > tileHeight) tileHeight = rect.height;
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