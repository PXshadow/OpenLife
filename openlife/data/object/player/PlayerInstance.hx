package openlife.data.object.player;
import openlife.settings.ServerSettings;
import openlife.data.map.MapData;
@:expose("PlayerInstance")
class PlayerInstance
{
    // Arcurus: remember that y is counted from bottom not from top
    public var gx:Int = 400; //global x offset from birth
    public var gy:Int = 300; //global y offset from birth 

    /**
     * Player ID, given by server
     */
    public var p_id:Int = 0;
    /**
     * Player's object ID, from objects
     */
    public var po_id:Int = 19;
    /**
     * facing direction 1 (right) or -1 (left)
     */
    public var facing:Int = 0;
    /**
     * N/A
     */
    public var action:Int = 0;
    /**
     * action x
     */
    public var action_target_x:Int = 0;
    /**
     * action y
     */
    public var action_target_y:Int = 0;
    /**
     * object id array from container format
     */
    public var o_id:Array<Int> = [0]; // replaced with heldObject in GlobalPlayerInstance be sure to still set it if changing heldObject
    /**
     * N/A
     */
    public var o_origin_valid:Int = 0;
    /**
     * x origin 
     */
    public var o_origin_x:Int = 0;
    /**
     * y origin
     */
    public var o_origin_y:Int = 0;
    /**
     * transition source id of object
     */
    public var o_transition_source_id:Int = -1;
    /**
     * heat value
     */
    public var heat:Float = 0;
    /**
     * sequence number of done moving
     */
    public var done_moving_seqNum:Int = 0;
    /**
     * forced set pos
     */
    public var forced:Bool = false;
    /**
     * tileX int
     */
    public var x:Int = 0;
    /**
     * tileY int
     */
    public var y:Int = 0;
    /**
     * age of player
     */
    public var age:Float = ServerSettings.StartingEveAge;
    /**
     * age rate of increase
     */
    public var age_r:Float = ServerSettings.AgingSecondsPerYear;

    //public static var initial_move_speed:Float = 3.75;

    /**
     * move speed of player
     */
    public var move_speed:Float = ServerSettings.InitialPlayerMoveSpeed;
    /**
     * clothing set string
     */
    public var clothing_set:String = "0;0;0;0;0;0";
    /**
     * just ate id
     */
    public var just_ate:Int = 0;
    /**
     * last ate id
     */
    public var last_ate_id:Int = 0;
    /**
     * responsible for player id
     */
    public var responsible_id:Int = -1;
    /**
     * held yum boolean
     */
    public var held_yum:Bool = false;
    /**
     * tool learned boolean
     */
    public var held_learned:Bool = false;
    /**
     * array of properties to generate PlayerType
     * @param a 
     */
    public var deleted:Bool = false;
    public var reason:String = "";
    var i:Int = 0;
    var a:Array<String>;

    public function new(a:Array<String>)
    {
        this.a = a;
        //var name = Reflect.fields(this);
        if (a.length < 23 + 1) return;
        p_id = int();
        po_id = int();
        facing = int();
        action = int();
        action_target_x = int();
        action_target_y = int();
        o_id = MapData.id(a[i++]);
        o_origin_valid = int();
        o_origin_x = int();
        o_origin_y = int();
        o_transition_source_id = int();
        heat = int();
        done_moving_seqNum = int();
        forced = int() == 1;
        if (a[i] == "X" && a[i+1] == "X")
        {
            deleted = true;
        }else{
            x = int();
            y = int();
        }
        age = float();
        age_r = float();
        move_speed = float();
        clothing_set = string();
        just_ate = int();
        responsible_id = int();
        if (a.length <= 23) return;
        held_yum = a[i++] == "1";
        if (a.length <= 24) return;
        held_learned = a[i++] == "1";
        if (deleted)
            reason = a[i];
    }
    private inline function int():Int
    {
        return Std.parseInt(a[i++]);
    }
    private inline function float():Float
    {
        return Std.parseFloat(a[i++]);
    }
    private inline function string():String
    {
        return a[i++];
    }
    public function update(instance:PlayerInstance)
    {
        p_id = instance.p_id;
        po_id = instance.po_id;
        facing = instance.facing;
        action_target_x = instance.action_target_x;
        action_target_y = instance.action_target_y;
        o_id = instance.o_id;
        o_origin_valid = instance.o_origin_valid;
        o_origin_x = instance.o_origin_x;
        o_origin_y = instance.o_origin_y;
        o_transition_source_id = instance.o_transition_source_id;
        heat = instance.heat;
        done_moving_seqNum = instance.done_moving_seqNum;
        forced = instance.forced;
        x = instance.x;
        y = instance.y;
        age = instance.age;
        age_r = instance.age_r;
        move_speed = instance.move_speed;
        clothing_set = instance.clothing_set;
        just_ate = instance.just_ate;
        responsible_id = instance.responsible_id;
        held_yum = instance.held_yum;
        held_learned = instance.held_learned;
    }
    public function clone():PlayerInstance
    {
        var instance = new PlayerInstance([]);
        instance.p_id = p_id;
        instance.po_id = po_id;
        instance.facing = facing;
        instance.action_target_x = action_target_x;
        instance.action_target_y = action_target_y;
        instance.o_id = o_id;
        instance.o_origin_valid = o_origin_valid;
        instance.o_origin_x = o_origin_x;
        instance.o_origin_y = o_origin_y;
        instance.o_transition_source_id = o_transition_source_id;
        instance.heat = heat;
        instance.done_moving_seqNum = done_moving_seqNum;
        instance.forced = forced;
        instance.x = x;
        instance.y = y;
        instance.age = age;
        instance.age_r = age_r;
        instance.move_speed = move_speed;
        instance.clothing_set = clothing_set;
        instance.just_ate = just_ate;
        instance.responsible_id = responsible_id;
        instance.held_yum = held_yum;
        instance.held_learned = held_learned;
        return instance;
    }
    /**
     * toString for debug
     * @return String output = "field: property"
     */
    public function toString():String
    {
        var string:String = "";
        for(field in Reflect.fields(this))
        {
            string += field + ": " + Reflect.getProperty(this,field) + "\n";
        }
        return string;
    }

    // TODO better use relative toData which transforms x,y to relative position
    public function toData():String
    {
        //o_origin_valid = 1;
        //441 2404 0 1 4 -6 33 1 4 -6 -1 0.26 8 0 4 -6 16.14 60.00 3.75 0;0;0;0;0;0 0 0 -1 0 1
        //return '$p_id $po_id $facing $action $action_target_x $action_target_y ${MapData.stringID(o_id)} $o_origin_valid $o_origin_x $o_origin_y $o_transition_source_id $heat $done_moving_seqNum ${(forced ? "1" : "0")} ${deleted ? 'X X' : '$x $y'} ${Std.int(age*100)/100} $age_r $move_speed $clothing_set $just_ate $last_ate_id $responsible_id ${(held_yum ? "1" : "0")} ${(held_learned ? "1" : "0")} ${deleted ? reason : ''}';

        return toRelativeData(this);
    }

    public function toRelativeData(forPlayer:PlayerInstance):String
    {
        var relativeX = this.gx - forPlayer.gx;
        var relativeY = this.gy - forPlayer.gy;

        var cutAge = Std.int(age * 100) / 100;
        var cutAge_r = Std.int(age_r * 100) / 100;
        var cutMove_speed = Std.int(move_speed * 100) / 100;
        var heldObject = o_id[0] < 0 ?  '${o_id[0]}' : MapData.stringID(o_id); 

        //441 2404 0 1 4 -6 33 1 4 -6 -1 0.26 8 0 4 -6 16.14 60.00 3.75 0;0;0;0;0;0 0 0 -1 0 1
        return '$p_id $po_id $facing $action ${action_target_x + relativeX}  ${action_target_y  + relativeY} ${heldObject} $o_origin_valid ${o_origin_x + relativeX} ${o_origin_y + relativeY} $o_transition_source_id $heat $done_moving_seqNum ${(forced ? "1" : "0")} ${deleted ? 'X X' : '${x + relativeX} ${y + relativeY}'} $cutAge $cutAge_r $cutMove_speed $clothing_set $just_ate $last_ate_id $responsible_id ${(held_yum ? "1" : "0")} ${(held_learned ? "1" : "0")} ${deleted ? reason : ''}';
    }
}
/*
Deleted players reported in update with
X X 
for x y
and a reason string at the tail end of the line.  Reason can be

reason_disconnected
reason_killed_id   (where id is the object that killed the player)
reason_hunger
reason_nursing_hunger  (starved while nursing a hungry baby)
reason_age
*/
//151 1628 0 0 0 0 0 0 0 0 -1 0.31 2 0 0 0 14.12 60.00 3.75 0;0;0;0;0;0 0 0 -1 0 0
//151 1628 0 0 0 0 0 0 0 -1 0 2 1 0 0 14 60 3.75 0;0;0;0;0;0 0 0 0 0 0
