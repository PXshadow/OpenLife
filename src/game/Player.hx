package game;
import console.Program;
import data.GameData;
import openfl.display.TileContainer;
import motion.easing.Quad;
import console.Program.Pos;
#if openfl
import motion.MotionPath;
import motion.Actuate;
import openfl.display.Tile;
#end
import data.PlayerData.PlayerType;
import data.PlayerData.PlayerInstance;
import haxe.Timer;
import data.SpriteData;
import data.ObjectData;
import data.AnimationData;
class Player #if openfl extends TileContainer #end
{
    public var lastMove:Int = 1;
    public var moveTimer:Timer;
    public var instance:PlayerInstance;
    public var ageRange:Array<{min:Float,max:Float}> = [];
    public var sprites:Array<Tile> = [];
    public var object:TileContainer;
    public var moves:Array<Pos> = [];
    public var velocityX:Float = 0;
    public var velocityY:Float= 0;
    //how many frames till depletion
    public var timeInt:Int = -1;
    //pathing
    public var goal:Bool = false;
    public var program:Program = null;
    var gdata:GameData;
    public var follow:Bool = true;
    var objects:Objects;
    var multi:Float = 1;
    public function new(data:GameData,objects:Objects)
    {
        this.objects = objects;
        this.gdata = data;
        #if openfl
        super();
        #end
    }
    
    public function update()
    {
        #if openfl
        timeInt--;
        if (timeInt == -1)
        {
            if (goal) path();
            move();
        }
        if (timeInt > 0)
        {
            x += velocityX;
            y += velocityY;
        }
        #end
    }
    public function move():Bool
    {
        #if openfl
        //grab another move
        if(moves.length > 0)
        {
            var point = moves.pop();
            //flip (change direction)
            if (point.x != 0)
            {
                if (point.x > 0)
                {
                    scaleX = 1;
                }else{
                    scaleX = -1;
                }
            }
            //0.08
            force();
            timeInt = Std.int(Static.GRID/(Static.GRID * (instance.move_speed) * computePathSpeedMod()) * 60 * multi);
            velocityX = (point.x * (Static.GRID))/timeInt;
            velocityY = -(point.y * (Static.GRID))/timeInt;
            instance.x += point.x;
            instance.y += point.y;
            sort();
            return true;
        }
        timeInt = -1;
        return false;
        #end
    }
    public function step(mx:Int,my:Int):Bool
    {
        //no other move is occuring, and player is not moving on blocked
        if (timeInt > 0 || gdata.blocking.get(Std.string(instance.x + mx) + "." + Std.string(instance.y + my))) return false;
        timeInt = 0;
        //send data
        lastMove++;
        Main.client.send("MOVE " + instance.x + " " + instance.y + " @" + lastMove + " " + mx + " " + my);
        #if openfl
        var pos = new Pos();
        pos.x = mx;
        pos.y = my;
        moves = [pos];
        #end
        return true;
    }
    public function computePathSpeedMod():Float
    {
        var floorData = objects.objectMap.get(gdata.map.floor.get(instance.x,instance.y));
        if (floorData != null)  return floorData.speedMult;
        return 1;
    }
    public function measurePathLength():Float
    {
        var diagLength:Float = 1.4142356237;
        var totalLength:Float = 0;
        if (moves.length < 2)
        {
            return totalLength;
        }
        var lastPos = moves[0];
        for (i in 1...moves.length)
        {
            if (moves[i].x != lastPos.x && moves[i].y != lastPos.y)
            {
                totalLength += diagLength;
            }else{
                //not diag
                totalLength += 1;
            }
            lastPos = moves[i];
        }
        return totalLength;
    }
    public function equal(pos:Pos,pos2:Pos):Bool
    {
        if (pos.x == pos2.x && pos.y == pos2.y) return true;
        return false;
    }
    public function sub(pos:Pos,pos2:Pos):Pos
    {
        var pos = new Pos();
        pos.x = pos.x - pos2.x;
        pos.y = pos.y - pos2.y;
        return pos;
    }
    public function path()
    {
        var px:Int = program.goal.x - instance.x;
        var py:Int = program.goal.y - instance.y;
        if (px != 0) px = px > 0 ? 1 : -1;
        if (py != 0) py = py > 0 ? 1 : -1;
        if (px == 0 && py == 0)
        {
            //complete 
            program.stop();
        }else{
            if (!step(px,py))
            {
                //non direct path
                if (px == py || px == py * -1)
                {
                    //diagnol
                    if (!step(px,0)) step(0,py);
                }else{
                    //non diagnol
                    if (px == 0)
                    {
                        //vetical
                        if (!step(1,py)) step(-1,py);
                    }else{
                        //horizontal
                        if (!step(px,1)) step(px,-1);
                    }
                }
            }
        }
    }
    public function set(data:PlayerInstance)
    {
        instance = data;
        //pos and age
        if (instance.forced == 1) 
        {
            trace("forced");
            Main.client.send("FORCE " + instance.x + " " + instance.y);
            //force movement
            force();
        }
        //remove moves
        timeInt = 0;
        moves = [];
        //age();
        hold();
    }
    public function hold()
    {
        #if openfl
        //remove previous if any
        if (object != null)
        {
            removeTile(object);
            object = null;
        }
        if (instance.o_id == 0) return;
        if (instance.o_id > 0)
        {
            //object
            objects.add(instance.o_id,0,0,true,false);
        }else{
            //player
            trace("player holding object");
            objects.add(instance.o_id * -1,0,0,true,false);
        }
        object = objects.object;
        object.x = -instance.o_origin_x + Static.GRID/4;
        object.y = -instance.o_origin_y - Static.GRID/1.5;
        if (!contains(object)) addTile(object);
        #end
    }
    public function force() 
    {
        #if openfl
        //local position
        x = instance.x * Static.GRID;
        y = (Static.tileHeight - instance.y) * Static.GRID;
        #end
    }
    public function sort()
    {
        var diff:Int = 0;
        var object:Array<Tile> = gdata.tileData.object.get(instance.x,instance.y);
        if (object == null) object = gdata.tileData.floor.get(instance.x,instance.y);
        if (object == null) 
        {
            object = gdata.tileData.object.get(instance.x,instance.y + 1);
            diff = 1;
        }
        if (object == null) 
        {
            object = gdata.tileData.object.get(instance.x,instance.y - 1);
            diff = -1;
        }
        if (object == null || object[0] == null) return;
        objects.group.setTileIndex(this,objects.group.getTileIndex(object[0]) + diff);
    }
    public function age()
    {
        #if openfl
        var tile:Tile;
        for(i in 0...numTiles)
        {
            tile = getTileAt(i);
            if (tile == null) continue;
            tile.visible = true;
            if((ageRange[i].min > instance.age || ageRange[i].max < instance.age) && ageRange[i].min > 0)
            {
                tile.visible = false;
            }
        }
        #end
    }
}