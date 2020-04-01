package data.object.player;
import data.map.MapData;
#if nativeGen @:nativeGen #end
class PlayerInstance
{
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
    public var o_id:Array<Int> = [];
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
    public var o_transition_source_id:Int = 0;
    /**
     * heat value
     */
    public var heat:Int = 0;
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
    public var age:Float = 0;
    /**
     * age rate of increase
     */
    public var age_r:Float = 0;
    /**
     * move speed of player
     */
    public var move_speed:Float = 0;
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
    public var responsible_id:Int = 0;
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
    public function new(a:Array<String>)
    {
        //var name = Reflect.fields(this);
        if (a.length < 23) return;
        var i:Int = 0;
        p_id = Std.parseInt(a[i++]);
        po_id = Std.parseInt(a[i++]);
        facing = Std.parseInt(a[i++]);
        action = Std.parseInt(a[i++]);
        action_target_x = Std.parseInt(a[i++]);
        action_target_y = Std.parseInt(a[i++]);
        o_id = MapData.id(a[i++]);
        o_origin_valid = Std.parseInt(a[i++]);
        o_origin_x = Std.parseInt(a[i++]);
        o_origin_y = Std.parseInt(a[i++]);
        o_transition_source_id = Std.parseInt(a[i++]);
        heat = Std.parseInt(a[i++]);
        done_moving_seqNum = Std.parseInt(a[i++]);
        forced = a[i++] == "1";
        x = Std.parseInt(a[i++]);
        y = Std.parseInt(a[i++]);
        age = Std.parseFloat(a[i++]);
        age_r = Std.parseFloat(a[i++]);
        move_speed = Std.parseFloat(a[i++]);
        clothing_set = a[i++];
        just_ate = Std.parseInt(a[i++]);
        responsible_id = Std.parseInt(a[i++]);
        if (a.length <= 23) return;
        held_yum = a[i++] == "1";
        if (a.length <= 24) return;
        held_learned = a[i++] == "1";
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
    public function toData():String
    {
        return '$p_id $po_id $facing $action_target_x $action_target_y ${MapData.stringID(o_id)}$o_origin_valid $o_origin_x $o_origin_y $o_transition_source_id $heat $done_moving_seqNum ${(forced ? "1" : "0")} $x $y $age $age_r $move_speed $just_ate $last_ate_id $responsible_id ${(held_yum ? "1" : "0")} ${(held_learned ? "1" : "0")}';
    }
}