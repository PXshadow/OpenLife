package data.object.player;

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
    public var o_id:Array<Int>;
    public var o_origin_valid:Int = 0;
    public var o_origin_x:Int = 0;
    public var o_origin_y:Int = 0;
    public var o_transition_source_id:Int = 0;
    public var heat:Int = 0;
    public var done_moving_seqNum:Int = 0;
    public var forced:Bool = false;
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