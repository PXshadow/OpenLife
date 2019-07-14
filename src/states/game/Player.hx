package states.game;
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
import data.Point;
class Player #if openfl extends Object #end
{
    public var lastMove:Int = 1;
    public var moveTimer:Timer;
    public static var main:Player;
    public var instance:PlayerInstance;
    public var ageRange:Array<{min:Float,max:Float}> = [];
    public var game:Game;

    public var moves:Array<Point> = [];
    public var velocityX:Float = 0;
    public var velocityY:Float= 0;
    //how many frames till depletion
    public var time:Int = 0;
    var timeInt:Int = 0;
    public function new(game:Game)
    {
        this.game = game;
        #if openfl
        super();
        #end
    }
    public function update()
    {
        if (timeInt <= 0) move();
        if (timeInt > 0)
        {
            x += velocityX;
            y += velocityY;
            timeInt--;
        }
    }
    public function move()
    {
        if(moves.length > 0)
        {
            var point = moves.pop();
            instance.x += Std.int(point.x);
            instance.y += Std.int(point.y);
            sort();
            velocityX = (point.x * Static.GRID) / time;
            velocityY = (-point.y * Static.GRID) / time;  
            timeInt = time;
        }
    }
    public function step(mx:Int,my:Int)
    {
        if (timeInt > 0) return;
        var time = Static.GRID/(Static.GRID * instance.move_speed);
        //send data
        lastMove++;
        Main.client.send("MOVE " + instance.x + " " + instance.y + " @" + lastMove + " " + mx + " " + my);
        this.time = Std.int(time * 60);
        moves = [new Point(mx,my)];
    }
    public function set(data:PlayerInstance)
    {
        instance = data;
        trace("force " + instance.forced);
        if (instance.forced == 1) 
        {
            trace("forced");
            Actuate.pause(this);
            Main.client.send("FORCE " + instance.x + " " + instance.y);
        }
        pos();
        age();
    }
    public function pos()
    {
        x = (-game.offsetX + instance.x) * Static.GRID;
        y = (-game.offsetY - instance.y) * Static.GRID;
        sort();
    }
    public function sort()
    {
        //keep player on z order
        
    }
    public function age()
    {
        #if openfl
        var tile:Tile;
        for(i in 0...numTiles)
        {
            tile = getTileAt(i);
            tile.visible = true;
            if((ageRange[i].min > instance.age || ageRange[i].max < instance.age) && ageRange[i].min > 0)
            {
                tile.visible = false;
            }
        }
        #end
    }
}