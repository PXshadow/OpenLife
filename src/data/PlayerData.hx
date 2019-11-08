package data;
import game.Player;
#if openfl
import motion.easing.Quad;
import motion.MotionPath;
import motion.actuators.GenericActuator;
import motion.Actuate;
import openfl.display.Sprite;
import openfl.display.Bitmap;
import openfl.display.Tile;
import openfl.display.DisplayObjectContainer;
import openfl.display.Shape;
#end
class PlayerData
{
    public var key:Map<Int,PlayerType> = new Map<Int,PlayerType>();
    public var array:Array<PlayerType> = [];
    public var primary:Int = -1;
    public var update:Void->Void;
    public function new()
    {
        
    }
}
class PlayerType 
{
    //id
    public var p_id:Int = 0;
    //object of player
    public var po_id:Int = 0;
    public var facing:Int = 0;
    public var action:Int = 0;
    public var action_target_x:Int = 0;
    public var action_target_y:Int = 0;
    //object id
    public var o_id:Int = 0;
    public var o_origin_valid:Int = 0;
    public var o_origin_x:Int = 0;
    public var o_origin_y:Int = 0;
    public var o_transition_source_id:Int = 0;
    public var heat:Int = 0;
    public var done_moving_seqNum:Int = 0;
    public var forced:Int = -1;
    public var x:Int = 0;
    public var y:Int = 0;
    public var age:Float = 0;
    public var age_r:Float = 0;
    public var move_speed:Float = 0;
    public var clothing_set:String = "";
    public var just_ate:Int = 0;
    public var last_ate_id:Int = 0;
    public var responsible_id:Int = 0;
    public var held_yum:Bool = false;
    public var held_learned:Bool = false;
    public function new()
    {

    }
    public function toString():String
    {
        var string:String = "";
        for(field in Reflect.fields(this))
        {
            string += field + ": " + Reflect.getProperty(this,field) + "\n";
        }
        return string;
    }
}
class PlayerInstance extends PlayerType
{
    public function new(a:Array<String>)
    {
        super();
        var index:Int = 0;
        //var name = Reflect.fields(this);
        for(value in a)
        {
            //index
            switch(index++)
            {
                case 0:
                p_id = Std.parseInt(value);
                case 1:
                po_id = Std.parseInt(value);
                case 2:
                //facing override
                facing = Std.parseInt(value);
                case 3:
                //action attempt
                action = Std.parseInt(value);
                case 4:
                action_target_x = Std.parseInt(value);
                case 5:
                action_target_y = Std.parseInt(value);
                case 6:
                o_id = Std.parseInt(value);
                case 7:
                o_origin_valid = Std.parseInt(value);
                case 8:
                o_origin_x = Std.parseInt(value);
                case 9:
                o_origin_y = Std.parseInt(value);
                case 10:
                o_transition_source_id = Std.parseInt(value);
                case 11:
                heat = Std.parseInt(value);
                case 12:
                done_moving_seqNum = Std.parseInt(value);
                case 13:
                ///forced
                forced = Std.parseInt(value);
                case 14:
                x = Std.parseInt(value);
                case 15:
                y = Std.parseInt(value);
                case 16:
                age = Std.parseFloat(value);
                case 17:
                age_r = Std.parseFloat(value);
                case 18:
                move_speed = Std.parseFloat(value);
                case 19:
                clothing_set = value;
                case 20:
                just_ate = Std.parseInt(value);
                case 21:
                responsible_id = Std.parseInt(value);
                case 22:
                held_yum = value == "1" ? true : false;
                case 24:
                held_learned = value == "1" ? true : false;
            }
            //trace(name[index - 1] + ": " + value);
        }
    }
}
#if full
import console.Program.Pos;
class PlayerMove 
{
    public var id:Int = 0;
    var xs:Int = 0;
    var ys:Int = 0;
    var total:Float = 0;
    var current:Float = 0;
    var trunc:Bool = false;
    var moves:Array<{x:Int,y:Int}> = [];
    public function  new(a:Array<String>)
    {
        var index:Int = 0;
        for(value in a)
        {
            switch(index++)
            {
                case 0:
                id = Std.parseInt(value);
                case 1:
                xs = Std.parseInt(value);
                case 2:
                //flip
                ys = Std.parseInt(value);
                case 3:
                total = Std.parseFloat(value);
                case 4:
                current = Std.parseFloat(value);
                case 5:
                trunc = value == "1" ? true : false;
                default:
                if(index > 6)
                {
                    if(index%2 == 0)
                    {
                        moves[moves.length - 1].y = Std.parseInt(value);
                    }else{
                        moves.push({x:Std.parseInt(value),y:0});
                    }
                }else{
                    throw("Player move parsing moves failed");
                }
            }
        }
    }
    public function movePlayer(player:Player)
    {
        /*if(player.instance.x == xs + moves[moves.length - 1].x && player.instance.y == ys + moves[moves.length - 1].y)
        {
            //same move
            trace("same move");
            return;
        }*/
        //set pos
        player.ix = xs;
        player.iy = ys;
        var currentX:Int = 0;
        var currentY:Int = 0;
        player.moves = [];
        var pos:Pos;
        for(move in moves)
        {
            pos = new Pos();
            pos.x = move.x - currentX;
            pos.y = move.y - currentY;
            player.moves.push(pos);
            currentX = move.x;
            currentY = move.y;
        }
        player.motion();
    }
}
#end