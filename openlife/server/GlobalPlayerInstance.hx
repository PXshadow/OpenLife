package openlife.server;
import openlife.data.object.ObjectData;
import openlife.settings.ServerSettings;
import haxe.ds.Vector;
import openlife.data.object.ObjectHelper;
import openlife.data.map.MapData;
import openlife.data.transition.TransitionData;
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;
import sys.thread.Mutex;

using openlife.server.MoveExtender;

class GlobalPlayerInstance extends PlayerInstance {
    // holds additional ObjectInformation for the object held in hand / null if there is no additional object data
    public var heldObject:ObjectHelper; 

    // additional ObjectInformation for the object stored in backpack or other clothing. The size is 6 since 6 clothing slots
    public var clothingObjects:Vector<ObjectHelper> = new Vector(6); 

    // handles all the movement stuff
    public var me:MoveExtender = new MoveExtender();
    // is used since move and move update can change the player at the same time
    public var mutex = new Mutex();

    public var connection:Connection; 

    // remember that y is counted from bottom not from top
    public var gx:Int = 400; //global x offset from birth
    public var gy:Int = 300; //global y offset from birth 

    //food vars
    public var food_store:Float = ServerSettings.GrownUpFoodStoreMax / 2;
    public var food_store_max:Float = ServerSettings.GrownUpFoodStoreMax;
    var last_ate_fill_max:Int = 0;
    public var yum_bonus:Float = 0;
    var yum_multiplier = 0;

    var hasEatenMap = new Map<Int, Int>();

    public function new(a:Array<String>)
    {
        super(a);

        this.heldObject = ObjectHelper.readObjectHelper(this, [0]);
    }

    public function tx() : Int {return x + gx;}
    public function ty() : Int {return y + gy;}

    public function toRelativeData(forPlayer:GlobalPlayerInstance):String
    {
        var relativeX = this.gx - forPlayer.gx;
        var relativeY = this.gy - forPlayer.gy;

        o_origin_valid = 1; // TODO ???
        //441 2404 0 1 4 -6 33 1 4 -6 -1 0.26 8 0 4 -6 16.14 60.00 3.75 0;0;0;0;0;0 0 0 -1 0 1
        return '$p_id $po_id $facing $action ${action_target_x + relativeX}  ${action_target_y  + relativeY} ${MapData.stringID(o_id)} $o_origin_valid ${o_origin_x + relativeX} ${o_origin_y + relativeY} $o_transition_source_id $heat $done_moving_seqNum ${(forced ? "1" : "0")} ${deleted ? 'X X' : '${x + relativeX} ${y + relativeY}'} $age $age_r $move_speed $clothing_set $just_ate $last_ate_id $responsible_id ${(held_yum ? "1" : "0")} ${(held_learned ? "1" : "0")} ${deleted ? reason : ''}';
    }

    // works with coordinates relative to the player
    public function isClose(x:Int, y:Int, distance:Int = 1):Bool
    {    
        return (((this.x - x) * (this.x - x) <= distance * distance) && ((this.y - y) * (this.y - y) <= distance * distance));
    }

    /*
    SELF x y i#

    SELF is special case of USE action taken on self (to eat what we're holding
     or add/remove clothing).
     This differentiates between use actions on the object at our feet
     (same grid cell as us) and actions on ourself.
     If holding food i is ignored.
	 If not holding food, then SELF removes clothing, and i specifies
	 clothing slot:
     0=hat, 1=tunic, 2=frontShoe, 3=backShoe, 4=bottom, 5=backpack
    */
    public function self(x:Int, y:Int, clothingSlot:Int)
    {
        this.mutex.acquire();

        if(ServerSettings.debug)
        {
            doSelf(x,y,clothingSlot);
        }
        else{
            try{
                doSelf(x,y,clothingSlot);
            }
            catch(e)
            {                
                trace(e);

                // send PU so that player wont get stuck
                this.connection.send(PLAYER_UPDATE,[this.toData()]);
                this.connection.send(FRAME);
            }
        }

        this.mutex.release();
    }
    /*
        FX

        food_store food_capacity last_ate_id last_ate_fill_max move_speed responsible_id
        yum_bonus yum_multiplier#

        food_store is integer amount of food left in body, capacity is the integer 
        maximum amount of food.

        last_ate_id is the object id of the last piece of food eaten, or 0 if nothing
        was eaten recently

        last_ate_fill_max is an integer number indicating how many slots were full
        before what was just eaten.  Amount that what was eaten filled us up is
        (food_store - last_ate_fill_max).

        move_speed is floating point speed in grid square widths per second.

        responsible_id is id of player that fed you if you're a baby, or -1

        yum_bonus is an integer indicating the current stored bonus food.

        yum_multiplier is an integer indicating how many yum bonus points are earned
        when the next yummy food is eaten.
    */

    public function sendFoodUpdate(isPlayerAction:Bool = true)
    {
        //trace('\n\tFX food_store: ${Math.ceil(food_store)} food_capacity: ${Std.int(food_capacity)} last_ate_id: $last_ate_id last_ate_fill_max: $last_ate_fill_max move_speed: $move_speed responsible_id: $responsible_id yum_bonus: $yum_bonus yum_multiplier: $yum_multiplier');
        this.connection.send(FOOD_CHANGE,['${Math.ceil(food_store)} ${Std.int(food_store_max)} $last_ate_id $last_ate_fill_max $move_speed $responsible_id ${Math.ceil(yum_bonus)} $yum_multiplier'], isPlayerAction);
    }

    public function doSelf(x:Int, y:Int, clothingSlot:Int)
    {
        trace('self: ${this.o_id[0]} ${heldObject.objectData.description} clothingSlot: $clothingSlot');

        var objClothingSlot = -1;

        if(this.o_id[0] != 0)
        {
            var objectData = ObjectData.getObjectData(this.o_id[0]);
            //trace("OD: " + objectData.toFileString());        

            switch objectData.clothing.charAt(0) {
                case "h": objClothingSlot = 0;
                case "t": objClothingSlot = 1;
                case "s": objClothingSlot = 2;
                //case "s": objClothingSlot = 3; 
                case "b": objClothingSlot = 4;
                case "p": objClothingSlot = 5;
            }

            trace('objectData.clothing: ${objectData.clothing}');
            trace('objClothingSlot:  ${objClothingSlot}');
            trace('clothingSlot:  ${clothingSlot}');
        }

        if (this.o_id[0] != 0 && objClothingSlot == -1)
        {
            if(this.age < ServerSettings.MinAgeToEat)
            {
                trace('too young to eat player.age: ${this.age} < ServerSettings.MinAgeToEat: ${ServerSettings.MinAgeToEat} ');
                this.connection.send(PLAYER_UPDATE,[this.toData()]);
                this.connection.send(FRAME);
                return;
            }

            var foodValue = heldObject.objectData.foodValue;

            if(foodValue < 1)
            {
                trace('cannot eat this stuff no food value!!! ${heldObject.objectData.description}');

                this.connection.send(PLAYER_UPDATE,[this.toData()]);
                this.connection.send(FRAME);
                return;
            }

            if(food_store_max - food_store < (foodValue + 1) / 2){

                trace('too full to eat: food_store_max: $food_store_max - food_store: $food_store < ( foodValue: $foodValue + 1 ) / 2');

                this.connection.send(PLAYER_UPDATE,[this.toData()]);
                this.connection.send(FRAME);
                return;
            }

            var countEaten = hasEatenMap[heldObject.id()];      

            

            foodValue += ServerSettings.YumBonus;
            foodValue -= countEaten;

            var isSuperMeth = foodValue < heldObject.objectData.foodValue / 2;

            if(isSuperMeth) foodValue = Math.ceil(heldObject.objectData.foodValue / 2);

            if(isSuperMeth && food_store > 0)
            {
                trace('when food value is less then halve it can only be eaten if starving to death: foodValue: $foodValue original food value: ${heldObject.objectData.foodValue} food_store: $food_store');

                this.connection.send(PLAYER_UPDATE,[this.toData()]);
                this.connection.send(FRAME);
                return;
            }

            hasEatenMap[heldObject.id()] += 1;

            // eating YUM increases prestige / score while eating MEH reduces it
            if(isHoldingYum()){
                yum_multiplier += 1;

                trace('YUM: ${heldObject.id()} foodValue: $foodValue');

                if(WorldMap.calculateRandomFloat() < ServerSettings.YumFoodRestore)
                {                    
                    var hasEatenKeys = [for(key in hasEatenMap.keys()) key];

                    trace('YUM: hasEatenKeys.length: ${hasEatenKeys.length}');

                    // restore one food pip if eaten YUM
                    if(hasEatenKeys.length > 0)
                    {
                        var random = WorldMap.calculateRandomInt(hasEatenKeys.length -1);
                        var key = hasEatenKeys[random];

                        trace('YUM: random: $random hasEatenKeys.length: ${hasEatenKeys.length}');
                        
                        if(key != heldObject.id())
                        {
                            hasEatenMap[key] -= 1;
                            trace('YUM: hasEaten YES!!!: key: $key, ${hasEatenMap[key]}');

                            if(hasEatenMap[key] <= 0) hasEatenMap.remove(key);
                        }
                        else{
                            trace('YUM: hasEaten: NO!!!: key: $key, heldObject.id(): ${heldObject.id()}');
                        }
                    }
                }
            }
            else if(isHoldingMeh()) yum_multiplier -= 1;


            trace('foodValue: $foodValue countEaten: $countEaten');

            

            // food_store food_capacity last_ate_id last_ate_fill_max move_speed responsible_id
            /*
                last_ate_fill_max is an integer number indicating how many slots were full
                before what was just eaten.  Amount that what was eaten filled us up is
                (food_store - last_ate_fill_max).
            */
            this.last_ate_fill_max = Math.ceil(this.food_store);
            trace('last_ate_fill_max: $last_ate_fill_max');
            this.food_store += foodValue;
            this.last_ate_id = heldObject.id();
            this.responsible_id = -1; // self

            if (food_store > food_store_max){
                this.yum_bonus = food_store - food_store_max;
                food_store = food_store_max;
            } 

            sendFoodUpdate();

            setHeldObject(null);

            this.connection.send(PLAYER_UPDATE,[this.toData()]);
            this.connection.send(FRAME);
            return;
        }    

        if(objClothingSlot >= 0 || clothingSlot >=0){
            var array = this.clothing_set.split(";");

            if(array.length < 6)
            {
                trace('Clothing string missing slots: ${this.clothing_set}' );
            }  

            // set  the index for shoes that come on the other feet
            // TODO setting shoes is not always working nice
            // TODO if the clothing are shoes and there are shoes allready on the first shoe but not on the second and if the index is not set

            if(objClothingSlot == 2 && clothingSlot == -1)
            {
                clothingSlot = 3;
            }
            else
            {
                // always use clothing slot from the hold object if it has
                if(objClothingSlot > -1) clothingSlot = objClothingSlot;
            }

            trace('self: ${this.o_id[0]} clothingSlot: $clothingSlot objClothingSlot: $objClothingSlot');

            if(clothingSlot >= 0){
                // switch clothing if there is a clothing on this slot
                var tmp = Std.parseInt(array[clothingSlot]);
                array[clothingSlot] = '${this.o_id[0]}';
                this.clothing_set = '${array[0]};${array[1]};${array[2]};${array[3]};${array[4]};${array[5]}';

                this.heldObject = ObjectHelper.readObjectHelper(this, [tmp]);

                //doaction = true;
                this.o_id = [tmp];
                this.action = 1;
                this.action_target_x = x;
                this.action_target_y = y;
                this.o_origin_x = x;
                this.o_origin_y = y;
                this.o_origin_valid = 0; // TODO ???

                trace('this.clothing_set: ${this.clothing_set}');
            }
        }
        
        Connection.SendUpdateToAllClosePlayers(this);

        this.action = 0;
    }

    public function specialRemove(x:Int,y:Int,clothing:Int,id:Null<Int>)
    {
        // TODO implement

        Connection.SendUpdateToAllClosePlayers(this);
    }

    public function isHoldingYum() : Bool
    {
        if(heldObject.objectData.foodValue < 1) return false;

        var countEaten = hasEatenMap[heldObject.id()];

        return countEaten < ServerSettings.YumBonus; 
    }

    public function isHoldingMeh() : Bool
    {
        if(heldObject.objectData.foodValue < 1) return false;

        var countEaten = hasEatenMap[heldObject.id()];

        return countEaten > ServerSettings.YumBonus; 
    }

    public function setHeldObject(obj:ObjectHelper)
    {
        if(obj == null) obj = ObjectHelper.readObjectHelper(this, [0]);

        this.heldObject = obj;
        this.o_id = obj.writeObjectHelper([]);
        this.held_yum = isHoldingYum();    
    }

    public function transformHeldObject(id:Int)
    {
        heldObject.setId(id);
        setHeldObject(heldObject);
    }
}