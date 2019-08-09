package states.game;
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
class Player #if openfl extends Object #end
{
    public var lastMove:Int = 1;
    public var moveTimer:Timer;
    public static var main:Player;
    public var instance:PlayerInstance;
    public var ageRange:Array<{min:Float,max:Float}> = [];
    public var game:Game;
    #if openfl
    public var object:Object;
    #end
    public var moves:Array<Pos> = [];
    public var velocityX:Float = 0;
    public var velocityY:Float= 0;
    //how many frames till depletion
    public var delay:Int = 0;
    public var time:Int = 0;
    var timeInt:Int = 0;
    //pathing
    public var goal:Bool = false;
    public var restricted:Array<console.Program.Pos> = [];
    public function new(game:Game)
    {
        this.game = game;
        #if openfl
        super();
        type = PLAYER;
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
            //move camera if main player
            game.objects.px = velocityX;
            game.objects.py = velocityY;
            if (this == Player.main) game.objects.update(); //game.move(velocityX,velocityY);
            //remove time per frame
            timeInt--;
        }
        if (delay > 0) delay--;
        #end
    }
    private function restrict(pos:console.Program.Pos):Bool
    {
        if (restricted.indexOf(pos) == -1)  return false;
        return true;
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
            if (goal)
            {
                var pos = new console.Program.Pos();
                pos.x = instance.x;
                pos.y = instance.y;
                restricted.push(pos);
            }
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
        return false;
        #end
    }
    public function step(mx:Int,my:Int):Bool
    {
        //no other move is occuring, and player is not moving on blocked
        if (timeInt > 0 || game.data.blocking.get(Std.string(instance.x + mx) + "." + Std.string(instance.y + my))) return false;
        //restriction of previous path
        if (restricted.length > 0)
        {
            var pos = new console.Program.Pos();
            pos.x = instance.x + mx;
            pos.y = instance.y + my;
            if (restrict(pos)) return false;
        }
        //send data
        lastMove++;
        Main.client.send("MOVE " + instance.x + " " + instance.y + " @" + lastMove + " " + mx + " " + my);
        #if openfl
        timeSpeed();
        var pos = new Pos();
        pos.x = mx;
        pos.y = my;
        moves = [pos];
        #end
        return true;
    }
    public function timeSpeed()
    {
        #if openfl
        //get floor speed
        //var time = Static.GRID/(Static.GRID * instance.move_speed);
        //trace("instance move speed " + instance.move_speed);
        this.time = Std.int(Static.GRID/(Static.GRID * instance.move_speed) * 60);
        timeInt = 0;
        #end
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
            restricted = [];
            game.program.stop();
        }else{
            if (!step(px,py))
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
                            //reset restricted area as all pathing is blocked
                            restricted = [];
                        }
                    }
                }
            }
        }
        timeInt = time;
    }
    public function set(data:PlayerInstance)
    {
        instance = data;
        //trace("force " + instance.forced);
        //pos and age
        pos();
        if (instance.forced == 1) 
        {
            trace("forced");
            Main.client.send("FORCE " + instance.x + " " + instance.y);
            //reset camera
            game.center();
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
        //object holding
        if (instance.o_id == 0)
        {
            if (object != null) removeTile(object);
        }else{
            if (object != null)
            {
                if (object.id == Std.int(instance.o_id)) return;
                removeTile(object);
            }
            if (instance.o_id > 0)
            {
                //object
                object = game.objects.add(instance.o_id);
            }else{
                //player
                object = game.objects.add(Std.int(instance.o_id),true);
            }
            //remove from main objects display
            game.objects.removeTile(object);
            //add into player
            addTile(object);

            object.x = instance.o_origin_x;
            object.y = instance.o_origin_y;
        }
        #end
    }
    #if openfl override #else public #end function pos() 
    {
        #if openfl
        //converts from global to local
        tileX = instance.x + game.cameraX;
        tileY = instance.y + game.cameraY;
        super.pos();
        #end
    }
    public function sort()
    {
        //did not move vertically
        if (tileY == instance.y + game.cameraY) return;

        var obj:Object = null;
        for (i in 0...game.objects.numTiles)
        {
            obj = cast game.objects.getTileAt(i);
            if (obj.tileY == instance.y + game.cameraY - 1)
            {
                break;
            }else{
                obj = null;
            }
        }
        if (obj != null) 
        {
            game.objects.removeTile(this);
            game.objects.addTileAt(this,game.objects.getTileIndex(obj));
        }
    }
    public function age()
    {
        #if openfl
        var tile:Tile;
        for(i in 0...numSprites)
        {
            tile = get(i);
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