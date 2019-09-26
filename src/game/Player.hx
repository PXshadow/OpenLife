package game;
import openfl.geom.Point;
import motion.easing.Linear;
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
    //statics
    private static inline var babyHeadDownFactor:Float = 0.6;
    private static inline var babyBodyDownFactor:Float = 0.75;
    private static inline var oldHeadDownFactor:Float = 0.35;
    private static inline var oldHeadForwardFactor:Float = 2;

    public var lastMove:Int = 1;
    public var moveTimer:Timer;
    public var instance:PlayerInstance;
    public var ageRange:Array<{min:Float,max:Float}> = [];
    public var sprites:Array<Tile> = [];
    public var object:TileContainer;
    public var moves:Array<Pos> = [];
    public var velocityX:Float = 0;
    public var velocityY:Float = 0;
    //clothing
    public var backShoe:TileContainer = null;
    public var tunic:TileContainer = null;
    public var bottom:TileContainer = null;
    public var backpack:TileContainer = null;
    public var frontShoe:TileContainer = null;
    public var hat:TileContainer = null;
    //public var 
    //pathing
    public var moving:Bool = false;
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
    public function motion()
    {
        if (goal) path();
        //grab another move
        if(moves.length > 0 && !moving)
        {
            var point = moves.pop();
            //speed
            var time = Std.int(Static.GRID/(Static.GRID * (instance.move_speed) * computePathSpeedMod()) * 60 * multi);
            moving = true;
            Actuate.tween(this,time/60,{x:(instance.x + point.x) * Static.GRID,y:(Static.tileHeight - (instance.y + point.y)) * Static.GRID}).onComplete(function(_)
            {
                instance.x += point.x;
                instance.y += point.y;
                moving = false;
                motion();
            }).ease(Linear.easeNone);
            sort();
            return;
        }
    }
    public function getAgeHeadOffset(inAge:Float,head:Point,body:Point,frontFoot:Point)
    {
        if (inAge == -1) return new Point();
        var maxHead = head.y - body.y;
        if (inAge < 20)
        {
            var yOffset = ( ( 20 - inAge ) / 20 ) * babyHeadDownFactor * maxHead;
            return new Point(0,Math.round(-yOffset));
        }
        if (inAge >= 40)
        {
            if (inAge > 60)
            {
                inAge = 60;
            }
            var vertOffset = ( ( inAge - 40) / 20 ) * oldHeadDownFactor * maxHead;
            var footOffset = frontFoot.x - head.x;
            var forwardOffset = ( ( inAge - 40 ) / 20 ) * oldHeadDownFactor * footOffset;
            return new Point(Math.round(forwardOffset),Math.round(-vertOffset));
        }
        return new Point();
    }
    public function getAgeBodyOffset(inAge:Float,pos:Point)
    {
        if (inAge == -1) return new Point();
        if (inAge < 20)
        {
            var maxBody = pos.y;
            var yOffset = ( ( 20 - inAge) / 20) * babyBodyDownFactor * maxBody;
            return new Point(0,Math.round(-yOffset));
        }
        return new Point();
    }
    public function move(mx:Int,my:Int)
    {
        //pathfind(mx,my);
        step(mx,my);
    }
    public function step(mx:Int,my:Int):Bool
    {
        //no other move is occuring, and player is not moving on blocked
        if (moving || gdata.blocking.get(Std.string(instance.x + mx) + "." + Std.string(instance.y + my))) return false;
        //send data
        lastMove++;
        Main.client.send("MOVE " + instance.x + " " + instance.y + " @" + lastMove + " " + mx + " " + my);
        #if openfl
        var pos = new Pos();
        pos.x = mx;
        pos.y = my;
        moves = [pos];
        #end
        motion();
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
            pathfind(px,py);
        }
    }
    public function pathfind(px:Int,py:Int)
    {
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
        moves = [];
        age();
        hold();
        cloths();
    }
    public function cloths()
    {
        var array:Array<Array<String>> = [];
        for (string in instance.clothing_set.split(";"))
        {
            array.push(string.split(","));
        }
    }
    public function hold()
    {
        #if openfl
        //remove previous if any
        if (object != null)
        {
            removeTile(object);
            if (Std.is(object,Player))
            {
                objects.group.addTile(object);
            }
            object = null;
        }
        if (instance.o_id == 0) return;
        if (instance.o_id > 0)
        {
            //object
            objects.add(instance.o_id,0,0,true,false);
            object = objects.object;
        }else{
            //player
            trace("instance.o_id");
            var player = gdata.playerMap.get(instance.o_id * -1);
            trace("player holding id " + instance.o_id + " player " + player);
            if (player != null)
            {
                objects.group.removeTile(player);
                object = player;
                //trace("player holding object " + instance.o_id);
                //objects.add(instance.o_id * -1,0,0,true,false);
            }
        }
        if (object != null)
        {
            trace("instance o " + instance.o_origin_x + " " + instance.o_origin_y);
            object.x = -instance.o_origin_x + Static.GRID/4;
            object.y = -instance.o_origin_y - Static.GRID/1.5;
            if (!contains(object)) addTile(object);
        }
        #end
    }
    public function force() 
    {
        Actuate.pause(this);
        moving = false;
        //local position
        x = instance.x * Static.GRID;
        y = (Static.tileHeight - instance.y) * Static.GRID;
        moves = [];
    }
    public function sort()
    {
        var diff:Int = 0;
        var object:Array<Tile> = gdata.tileData.object.get(instance.x,instance.y);
        //floor
        if (object == null) 
        {
            object = gdata.tileData.floor.get(instance.x,instance.y);
            diff = -1;
        }
        /*if (object == null) 
        {
            object = gdata.tileData.object.get(instance.x,instance.y + 1);
            diff = 1;
        }*/
        if (object == null || object[0] == null) return;
        objects.group.setTileIndex(this,objects.group.getTileIndex(object[0]) + diff);
    }
    public function age()
    {
        #if openfl
        var tile:Tile;
        for(i in 0...sprites.length)
        {
            sprites[i].visible = true;
            if (ageRange[i] == null) continue;
            if((ageRange[i].min > instance.age || ageRange[i].max < instance.age) && ageRange[i].min > 0)
            {
                sprites[i].visible = false;
            }
        }
        #end
    }
}