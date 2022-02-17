package openlife.data.object.player;
import openlife.server.ServerAi;
import openlife.server.GlobalPlayerInstance;
import openlife.client.ClientTag;
import openlife.server.Connection;
import openlife.settings.ServerSettings;
import openlife.data.map.MapData;
@:expose("PlayerInstance")
@:rtti
class PlayerInstance
{
    /**
     * Player ID, given by server
     */
     public var p_id:Int = 0;
     
    // holds additional ObjectInformation for the object held in hand / null if there is no additional object data
    public var heldObject:ObjectHelper; 
    
    //ARcurus: food vars
    public var food_store:Float = 0;
    public var food_store_max:Float = 0;
    public var last_ate_fill_max:Int = 0;
    public var yum_bonus:Float = 0;
    public var yum_multiplier:Float = 0;

    // Arcurus: remember that y is counted from bottom not from top
    public var gx:Int = 400; //global x offset from birth
    public var gy:Int = 300; //global y offset from birth 

    public var tx(get, null):Int;
    public function get_tx(){return x + gx;}

    public var ty(get, null):Int;
    public function get_ty(){return y + gy;}

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
    public var heat:Float = 0.5;
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
    public var age:Float = 14;
    /**
     * age rate of increase
     */
    public var age_r:Float = 60;

    //public static var initial_move_speed:Float = 3.75;

    /**
     * move speed of player
     */
    public var move_speed:Float = 3.75;
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

    public var name(get, set):String;
    public function get_name(){return '';}
    public function set_name(newName:String){return newName;}

    public function isMoving() : Bool {return false;}
    public function isHeld() : Bool {return false;}

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

    public function toData(?rx:Int,?ry:Int,?age:Float,?age_r:Float,?move_speed:Float,heldObject:String="", forPlayerOffsetX:Int = 0, forPlayerOffsetY:Int = 0):String
    {
        //o_origin_valid = 1;
        if (heldObject == "")
            heldObject = o_id[0] < 0 ?  '${o_id[0]}' : MapData.stringID(o_id);
        if (rx == null)
            rx = this.x;
        if (ry == null)
            ry = this.y;
        if (age == null)
            age = this.age;
        if (age_r == null)
            age_r = this.age_r;
        if (move_speed == null)
            move_speed = this.move_speed;
   
        var tmpHeat = Std.int(heat * 100) / 100;
        age = Std.int(age * 100) / 100;
        age_r = Std.int(age_r * 100) / 100;
        move_speed = Std.int(move_speed * 100) / 100;

        var seqNum = isHeld() || isMoving() ? 0 : done_moving_seqNum; 

        trace('TODATA: ${name} $x $y + $gx $gy = $tx $ty r: $rx $ry seqNum: ${seqNum} isHeld: ${isHeld()} isMoving: ${isMoving()} ');
        //trace('AAI: p$p_id $rx,$ry');

        
        // TODO currently AutoFollowAi supports only one connection
        if(ServerSettings.AutoFollowAi)
        {
            var player = GlobalPlayerInstance.AllPlayers[p_id];
            var isHuman = player != null && player.isHuman();
            
            if(isHuman)
            {
                if(player.connection.serverAi == null) player.connection.serverAi = new ServerAi(player);
                seqNum = 0; // set is held
                player.food_store = 10; 
                player.hits = 0;                 
                var putext = '1 $po_id $facing $action ${action_target_x + forPlayerOffsetX} ${action_target_y + forPlayerOffsetY} $heldObject $o_origin_valid ${o_origin_x + forPlayerOffsetX} ${o_origin_y + forPlayerOffsetY} $o_transition_source_id $tmpHeat $seqNum ${(forced ? "1" : "0")} ${deleted ? 'X X' : '$rx $ry'} ${Std.int(age*100)/100} $age_r $move_speed $clothing_set $just_ate $last_ate_id $responsible_id ${(held_yum ? "1" : "0")} ${(held_learned ? "1" : "0")} ${deleted ? reason : ''}';
                player.connection.send(ClientTag.PLAYER_UPDATE,[putext], false);

                heldObject = '${-1}';
                // predent there is a dog that is carring me arrond
                return '$p_id 1658 $facing $action ${action_target_x + forPlayerOffsetX} ${action_target_y + forPlayerOffsetY} $heldObject $o_origin_valid ${o_origin_x + forPlayerOffsetX} ${o_origin_y + forPlayerOffsetY} $o_transition_source_id $tmpHeat $seqNum ${(forced ? "1" : "0")} ${deleted ? 'X X' : '$rx $ry'} ${Std.int(age*100)/100} $age_r $move_speed $clothing_set $just_ate $last_ate_id $responsible_id ${(held_yum ? "1" : "0")} ${(held_learned ? "1" : "0")} ${deleted ? reason : ''}';
            }
        }

        var putext = '$p_id $po_id $facing $action ${action_target_x + forPlayerOffsetX} ${action_target_y + forPlayerOffsetY} $heldObject $o_origin_valid ${o_origin_x + forPlayerOffsetX} ${o_origin_y + forPlayerOffsetY} $o_transition_source_id $tmpHeat $seqNum ${(forced ? "1" : "0")} ${deleted ? 'X X' : '$rx $ry'} ${Std.int(age*100)/100} $age_r $move_speed $clothing_set $just_ate $last_ate_id $responsible_id ${(held_yum ? "1" : "0")} ${(held_learned ? "1" : "0")} ${deleted ? reason : ''}';
        return putext;
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
