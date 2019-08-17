package states.game;
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
    public static var main:Player;
    public var instance:PlayerInstance;
    public var ageRange:Array<{min:Float,max:Float}> = [];
    public var game:Game;
    #if openfl
    public var object:TileContainer;
    #end
    public var moves:Array<Pos> = [];
    public var velocityX:Float = 0;
    public var velocityY:Float= 0;
    //how many frames till depletion
    public var delay:Int = 0;
    public var time:Int = 0;
    public var timeInt:Int = -1;
    //pathing
    public var goal:Bool = false;
    public var refine:Bool = false;
    public function new(game:Game)
    {
        this.game = game;
        #if openfl
        super();
        #end
    }
    
    public function update()
    {
        #if openfl
        if (timeInt == 0)
        {
            if (goal) path();
            move();
        }
        if (timeInt > 0)
        {
            //add to pos
            x += velocityX;
            y += velocityY;
            if (Player.main == this)
            {
                //move camera only if main player
                game.objects.group.x += -velocityX;
                game.objects.group.y += -velocityY;
                game.ground.x += -velocityX;
                game.ground.y += -velocityY;
            }
            //remove time per frame
            timeInt--;
        }
        if (delay > 0) delay--;
        #end
    }
    public function move():Bool
    {
        #if openfl
        //grab another move
        if(moves.length > 0)
        {
            var point = moves.pop();
            pos();
            instance.x += point.x;
            instance.y += point.y;
            sort();
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
            velocityX = (point.x * Static.GRID) / time;
            velocityY = -(point.y * Static.GRID) / time;
            timeInt = time;
            return true;
        }
        timeInt = -1;
        return false;
        #end
    }
    public function step(mx:Int,my:Int):Bool
    {
        //no other move is occuring, and player is not moving on blocked
        if (timeInt > 0 || game.data.blocking.get(Std.string(instance.x + mx) + "." + Std.string(instance.y + my))) return false;
        timeInt = 0;
        //send data
        lastMove++;
        Main.client.send("MOVE " + instance.x + " " + instance.y + " @" + lastMove + " " + mx + " " + my);
        #if openfl
        updateMoveSpeed();
        var pos = new Pos();
        pos.x = mx;
        pos.y = my;
        moves = [pos];
        #end
        return true;
    }
    public function updateMoveSpeed()
    {
        time = Std.int(Static.GRID /(Static.GRID * instance.move_speed) * 50 * 1);
        /*var moveLeft = measurePathLength();
        var numTurns:Int = 0;
        if (moves.length > 1)
        {
            var lastDir = sub(moves[0],moves[1]);
            var dir = new Pos();
            for (i in 0...moves.length - 1)
            {
                dir = sub(moves[i + 1],moves[i]);
                if (dir != lastDir)
                {
                    numTurns++;
                    lastDir = dir;
                }
            }
        }
        time = Std.int(Static.GRID /(Static.GRID * instance.move_speed) * 60);
        //boost when turninig
        time += Std.int((0.08 * numTurns) * 60);
        if (time < 0.1 * 60)
        {
            time = Std.int(0.1 * 60);
        }*/
    }
    public function measurePathLength():Float
    {
        var diagLength:Float = 1.4142356237;
        var totalLength:Float = 0;
        if (moves.length < 2)
        {
            return totalLength;
        }
        var lastPos = new Pos();
        lastPos = moves[0];
        for (i in 0...moves.length)
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
    public function path()
    {
        var px:Int = game.program.goal.x - instance.x;
        var py:Int = game.program.goal.y - instance.y;
        if (px != 0) px = px > 0 ? 1 : -1;
        if (py != 0) py = py > 0 ? 1 : -1;
        if (px == 0 && py == 0)
        {
            //complete 
            game.program.stop();
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
            /*if (!step(px,py))
            {
                //path blocked
                //x
                px *= -1;
                if (!step(px,py))
                {
                    //y
                    px *= -1;
                    py *= -1;
                    if (!step(px,py))
                    {
                        //x and y
                        px *= -1;
                        if (!step(px,py))
                        {
                            //all paths are blocked
                        }
                    }
                }
            }*/
        }
        timeInt = time;
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
            pos();
        }
        //remove moves
        timeInt = 0;
        moves = [];
        age();
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
            game.objects.add(instance.o_id,0,0,true,false);
        }else{
            //player
            trace("player");
            game.objects.add(instance.o_id * -1,0,0,true,false);
        }
        object = game.objects.object;
        object.x = -instance.o_origin_x + Static.GRID/4;
        object.y = -instance.o_origin_y - Static.GRID/1.5;
        #end
    }
    public function pos() 
    {
        #if openfl
        //local position
        x = instance.x * Static.GRID;
        y = (Static.tileHeight - instance.y) * Static.GRID;
        #end
    }
    public function sort()
    {
        
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