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
    public var animation:AnimationData;
    
    public function new()
    {
        #if openfl
        super();
        #end
        main = this;
    }
    public function move(mx:Int,my:Int)
    {
        //movement timer
        if (moveTimer != null) return;
        //instance.move_speed/Static.GRID
        var time = instance.move_speed * Static.GRID * 1000;
        moveTimer = new Timer(time);
        moveTimer.run = function()
        {
            //change data pos
            instance.x += mx;
            instance.y += my;
            moveTimer.stop();
            moveTimer = null;
        }
        //send data
        lastMove++;
        Main.client.send("MOVE " + instance.x + " " + instance.y + " @" + lastMove + " " + mx + " " + my);
        trace("time " + time);
        //tween
        #if openfl
        Actuate.tween(this,0.4,{x:(instance.x + mx) * Static.GRID,y:-(instance.y + my) * Static.GRID});
        #end
    }
    public function animate()
    {
        if (animation != null) return;
        animation = new AnimationData(instance.po_id);
        if(animation.fail)
        {
            trace("player animation fail " + instance.po_id);
            return;
        }
        trace("param " + animation.record[0].params);
    }
    public function set(data:PlayerInstance)
    {
        instance = data;
        //pos
        x = instance.x;
        y = -instance.y;
        age();
        animate();
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