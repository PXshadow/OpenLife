import openfl.display.Sprite;
import openfl.display.Bitmap;
import openfl.display.Tile;
import openfl.display.DisplayObjectContainer;
import openfl.display.Shape;
class PlayerData
{
    public var key:Map<Int,PlayerType> = new Map<Int,PlayerType>();
    public var primary:Int = -1;
    public var update:Void->Void;
    public function new()
    {

    }
    public function set() 
    {
        for(player in PlayerInstance.array)
        {
            if(primary == -1) primary = player.p_id;
            key.set(player.p_id,cast(player,PlayerType));
        }
        //clear
        PlayerInstance.array = [];
        if(update != null) update();
    }
}
class PlayerType 
{
    public var p_id:Int = 0;
    public var po_id:Int = 0;
    public var facing_action:Int = 0;
    public var action_target_x:Int = 0;
    public var action_target_y:Int = 0;
    public var o_id:Int = 0;
    public var o_origin_valid:Int = 0;
    public var o_origin_x:Int = 0;
    public var o_origin_y:Int = 0;
    public var o_transition_source_id:Int = 0;
    public var heat:Int = 0;
    public var done_moving_seqNum:Int = 0;
    public var forceX:Int = 0;
    public var forceY:Int = 0;
    public var age:Int = 0;
    public var age_r:Int = 0;
    public var move_speed:Float = 0;
    public var clothing_set:String = "";
    public var just_ate:Int = 0;
    public var last_ate_id:Int = 0;
    public var responsible_id:Int = 0;
    public var held_yum:Int = 0;
}
class PlayerInstance extends PlayerType
{
    public function new()
    {
        array.push(this);
    }
    public static var array:Array<PlayerInstance> = [];
}   