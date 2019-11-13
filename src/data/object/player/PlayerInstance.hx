package data.object.player;
import data.map.MapData;
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
                o_id = MapData.id(value);
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
                forced = value == "1" ? true : false;
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