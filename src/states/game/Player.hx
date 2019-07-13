package states.game;
#if openfl
import motion.MotionPath;
import openfl.geom.Point;
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
    public function new(game:Game)
    {
        this.game = game;
        #if openfl
        super();
        #end
        trace("new player");
    }
    public function step(mx:Int,my:Int)
    {
        //movement timer
        if (moveTimer != null) return;
        //instance.move_speed/Static.GRID
        var time = instance.move_speed * Static.GRID;
        moveTimer = new Timer(time);
        moveTimer.run = function()
        {
            //change data pos
            instance.x += mx;
            instance.y += my;
            pos();
            moveTimer.stop();
            moveTimer = null;
        }
        //send data
        lastMove++;
        Main.client.send("MOVE " + instance.x + " " + instance.y + " @" + lastMove + " " + mx + " " + my);
        //tween
        #if openfl
        Actuate.tween(this,time/1000,{x: x + mx * Static.GRID,y: y + my * Static.GRID});
        #end
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