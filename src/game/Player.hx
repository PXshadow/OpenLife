package game;
import console.Program.Pos;
import console.Program;
import data.GameData;
#if openfl
import openfl.geom.Point;
import openfl.display.TileContainer;
import motion.easing.Quad;
import motion.easing.Linear;
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
    #if openfl
    //statics
    private static inline var babyHeadDownFactor:Float = 0.6;
    private static inline var babyBodyDownFactor:Float = 0.75;
    private static inline var oldHeadDownFactor:Float = 0.35;
    private static inline var oldHeadForwardFactor:Float = 2;

    public var ageRange:Array<{min:Float,max:Float}> = [];
    public var sprites:Array<Tile> = [];
    public var object:Tile;
    var objects:Objects;
    //clothing hat;tunic;front_shoe;back_shoe;bottom;backpack
    var clothing:Array<TileContainer> = [];
    #end
    public var moves:Array<Pos> = [];
    public var instance:PlayerInstance;
    var clothingInt:Array<Int> = [];
    //pathing
    public var lastMove:Int = 1;
    public var moveTimer:Timer;
    public var moving:Bool = false;
    public var goal:Bool = false;
    public var program:Program = null;
    public var follow:Bool = true;
    var multi:Float = 1;
    //locally used instance pos
    public var ix:Int = 0;
    public var iy:Int = 0;
    //locally used object
    public var oid:Int = 0;
    public var held:Bool = false;
    //name
    public var firstName:String = "";
    public var lastName:String = "";
    public var text:String = "";
    public function new(#if openfl objects:Objects #end)
    {
        #if openfl
        this.objects = objects;
        #end
        #if openfl
        super();
        #end
    }
    public function motion()
    {
        //use another move
        if(moves.length > 0 && !moving)
        {
            var point = moves.pop();
            moving = true;
            var time = Std.int(Static.GRID/(Static.GRID * (instance.move_speed) * computePathSpeedMod()) * 60 * multi);
            #if !openfl
            sys.thread.Thread.create(() -> {
                Sys.sleep(time/60);
                trace("finish move");
                ix += point.x;
                iy += point.y;
                moving = false;
                if (goal)
                {
                    path();
                }else{
                    motion();
                }
            });
            #else
            //facing direction
            if (point.x != 0)
            {
                if (point.x > 0)
                {
                    scaleX = 1;
                }else{
                    scaleX = -1;
                }
            }
            Actuate.tween(this,time/60,{x:(ix + point.x) * Static.GRID,y:(Static.tileHeight - (iy + point.y)) * Static.GRID}).onComplete(function(_)
            {
                ix += point.x;
                iy += point.y;
                moving = false;
                if (goal)
                {
                    path();
                }else{
                    motion();
                }
            }).ease(Linear.easeNone);
            #end
        }
    }
    #if openfl
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
    #end
    public function step(mx:Int,my:Int):Bool
    {
        //no other move is occuring, and player is not moving on blocked
        if (moving || Main.data.blocking.get(Std.string(ix + mx) + "." + Std.string(iy + my))) return false;
        //send data
        lastMove++;
        //trace("step move " + mx + " " + my);
        Main.client.send("MOVE " + ix + " " + iy + " @" + lastMove + " " + mx + " " + my);
        var pos = new Pos();
        pos.x = mx;
        pos.y = my;
        moves = [pos];
        motion();
        return true;
    }
    public function computePathSpeedMod():Float
    {
        var floorData = Main.data.objectMap.get(Main.data.map.floor.get(ix,iy));
        var objectData = Main.data.objectMap.get(oid);
        var multiple:Float = 1;
        if (floorData != null) multiple *= floorData.speedMult;
        if (objectData != null) multiple *= objectData.speedMult;
        return multiple;
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
    public var px:Int = 0;
    public var py:Int = 0;
    public function path()
    {
        if (!setPath())
        {
            //complete
            program.end();
            return;
        }
        pathfind(px,py);
    }
    public function setPath():Bool
    {
        px = program.goal.x - ix;
        py = program.goal.y - iy;
        var bool:Bool = false;
        if (px != 0) 
        {
            px = px > 0 ? 1 : -1;
            bool = true;
        }
        if (py != 0)
        {
            py = py > 0 ? 1 : -1;
            bool = true;
        }
        return bool;
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
        if (instance == null)
        {
            ix = data.x;
            iy = data.y;
        }
        instance = data;
        //pos and age
        if (instance.forced == 1) 
        {
            trace("force!");
            if (held)
            {
                //added back to stage
                #if openfl
                objects.group.addTile(this);
                #end
                held = false;
            }
            ix = instance.x;
            iy = instance.y;
            Main.client.send("FORCE " + ix + " " + iy);
            if (program != null) program.clean();
            //force movement
            force();
        }
        //remove moves
        moves = [];
        #if openfl
        age();
        #end
        hold();
        cloths();
    }
    public function cloths()
    {
        var temp:Array<Int> = [];
        var array:Array<Array<String>> = [];
        var sub:Array<String> = [];
        for (string in instance.clothing_set.split(";"))
        {
            sub = string.split(",");
            temp.push(Std.parseInt(sub[0]));
            for (i in 1...sub.length)
            {
                temp.push(Std.parseInt(sub[i]) * -1);
            }
        }
        if (temp != clothingInt)
        {
            clothingInt = temp;
            #if openfl
            var index:Int = 0;
            clothing = [];
            for(i in clothingInt)
            {
                if (i == 0) 
                {
                    clothing.push(null);
                    continue;
                }
                if (i > 0)
                {
                    //new clothing
                    objects.add(i,0,0,true,false);
                    object = objects.object;
                    addTile(object);
                }else{
                    /*//add to preexisting clothing
                    var data = data.objectMap.get(clothingInt[clothing.length - 1]);
                    objects.add(i,0,0,true,false);
                    clothing[clothing.length - 1].addTiles(objects.sprites);*/
                }
            }
            #end
        }
    }
    public function hold()
    {
        #if !openfl
        if (instance.o_id != oid)
        {
            //change
            oid = instance.o_id;
        }
        #else
        if (instance.o_id != oid)
        {
            //remove previous
            if (object != null)
            {
                removeTile(object);
                if (oid < 0) 
                {
                    //player add back to stage
                    objects.group.addTile(object);
                    if (Std.is(object,Player)) cast(object,Player).held = false;
                }
                object = null;
            }
            //set oid
            oid = instance.o_id;
            if (oid == 0) return;
            var objectData:ObjectData = null;
            //object
            if (oid > 0)
            {
                //object coming from the world
                objectData = Main.data.objectMap.get(oid);
                if (instance.o_origin_valid == 1)
                {
                    var array = Main.data.tileData.object.get(instance.o_origin_x,instance.o_origin_y);
                    if (array != null)
                    {
                        //trace("array " + array);
                        var mo = Main.data.map.object.get(instance.o_origin_x,instance.o_origin_y);
                        var index = -1;
                        if (mo != null) index = mo.indexOf(oid);
                        if (index > -1 && index < array.length)
                        {
                            object = array[index];
                            addTile(object);
                            //remove tiles and data
                            Main.data.map.object.set(instance.o_origin_x,instance.o_origin_y,[]);
                            Main.data.tileData.object.set(instance.o_origin_x,instance.o_origin_y,[]);
                        }
                    }
                }else{
                    //new object not being pulled from
                    objects.add(instance.o_id,0,0,true,false);
                    object = objects.object;
                }
            }
            //player
            if (oid < 0)
            {
                var player = Main.data.playerMap.get(oid * -1);
                if (player != null)
                {
                    objectData = Main.data.objectMap.get(player.instance.po_id);
                    objects.group.removeTile(player);
                    player.held = true;
                    //same facing as mother
                    player.scaleX = scaleX;
                    //add to mother's display list
                    addTile(player);
                    object = player;
                }
            }
            if (objectData != null && object != null && objectData.heldOffset != null)
            {
                object.x = 20 + objectData.heldOffset.x;
                object.y = -Static.GRID/2 + objectData.heldOffset.y - 18;
            }
        }
        #end
    }
    public function force() 
    {
        moves = [];
        moving = false;
        #if openfl
        Actuate.pause(this);
        //local position
        x = instance.x * Static.GRID;
        y = (Static.tileHeight - instance.y) * Static.GRID;
        #end
    }
    #if openfl
    public function sort()
    {
        var diff:Int = 0;
        var object:Array<Tile> = Main.data.tileData.object.get(instance.x,instance.y);
        //floor
        if (object == null) 
        {
            object = Main.data.tileData.floor.get(instance.x,instance.y);
            diff = -1;
        }
        /*if (object == null) 
        {
            object = Main.data.tileData.object.get(instance.x,instance.y + 1);
            diff = 1;
        }*/
        if (object == null || object[0] == null) return;
        objects.group.setTileIndex(this,objects.group.getTileIndex(object[0]) + diff);
    }
    public function age()
    {
        objects.visibleSprites(instance.po_id,sprites,Std.int(instance.age));
    }
    #end
}