package openlife.server;
import openlife.client.ClientTag;
import openlife.data.object.ObjectData;
import openlife.settings.ServerSettings;
import haxe.ds.Vector;
import openlife.data.object.ObjectHelper;
import openlife.data.map.MapData;
import openlife.data.transition.TransitionData;
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;
import sys.thread.Mutex;

using openlife.server.MoveHelper;

class GlobalPlayerInstance extends PlayerInstance {
    // holds additional ObjectInformation for the object held in hand / null if there is no additional object data
    public var heldObject:ObjectHelper; 

    // additional ObjectInformation for the object stored in backpack or other clothing. The size is 6 since 6 clothing slots
    public var clothingObjects:Vector<ObjectHelper> = new Vector(6); 

    // handles all the movement stuff
    public var moveHelper = new MoveHelper();
    // is used since move and move update can change the player at the same time
    public var mutex = new Mutex();

    public var connection:Connection; 

    // remember that y is counted from bottom not from top
    public var gx:Int = 400; //global x offset from birth
    public var gy:Int = 300; //global y offset from birth 

    public var trueAge:Float = ServerSettings.StartingEveAge;

    //food vars
    public var food_store:Float = ServerSettings.GrownUpFoodStoreMax / 2;
    public var food_store_max:Float = ServerSettings.GrownUpFoodStoreMax;
    var last_ate_fill_max:Int = 0;
    public var yum_bonus:Float = 0;
    public var yum_multiplier = 0;

    var hasEatenMap = new Map<Int, Int>();

    // craving
    var currentlyCraving:Int = 0;
    var lastCravingIndex:Int = 0;
    var cravings = new Array<Int>();

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
        return '$p_id $po_id $facing $action ${action_target_x + relativeX}  ${action_target_y  + relativeY} ${MapData.stringID(o_id)} $o_origin_valid ${o_origin_x + relativeX} ${o_origin_y + relativeY} $o_transition_source_id $heat $done_moving_seqNum ${(forced ? "1" : "0")} ${deleted ? 'X X' : '${x + relativeX} ${y + relativeY}'} ${Std.int(age*100)/100} $age_r $move_speed $clothing_set $just_ate $last_ate_id $responsible_id ${(held_yum ? "1" : "0")} ${(held_learned ? "1" : "0")} ${deleted ? reason : ''}';
    }

    // works with coordinates relative to the player
    public function isClose(x:Int, y:Int, distance:Int = 1):Bool
    {    
        return (((this.x - x) * (this.x - x) <= distance * distance) && ((this.y - y) * (this.y - y) <= distance * distance));
    }

    public function CalculateHealthFactor(forSpeed:Bool) : Float
    {
        //var health:Float = forSpeed ? this.yum_multiplier : this.yum_multiplier + ServerSettings.HealthStart;
        var health:Float = this.yum_multiplier + ServerSettings.HealthStart;

        var healthFactor:Float; 

        var maxBoni = forSpeed ? 1.2 : 2;

        if(health >= 0) healthFactor = (maxBoni  * health + ServerSettings.HealthFactor) / (health + ServerSettings.HealthFactor);
        else healthFactor = (health - ServerSettings.HealthFactor) / ( 2 * health - ServerSettings.HealthFactor);

        return healthFactor;
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
            }
        }

        // send always PU so that player wont get stuck
        this.connection.send(PLAYER_UPDATE,[this.toData()]);
        this.connection.send(FRAME);

        this.mutex.release();
    }    

    private function doSelf(x:Int, y:Int, clothingSlot:Int)
    {
        trace('self: ${this.o_id[0]} ${heldObject.objectData.description} clothingSlot: $clothingSlot');

        if(doEating()) return;

        doSwitchCloths(clothingSlot);
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

    public function doEating() : Bool
    {
        if (this.o_id[0] == 0) return false;

        if(this.age < ServerSettings.MinAgeToEat)
        {
            trace('too young to eat player.age: ${this.age} < ServerSettings.MinAgeToEat: ${ServerSettings.MinAgeToEat} ');
            return false;
        }

        var heldObjData = heldObject.objectData;
        if(heldObjData.dummyParent != null) heldObjData = heldObjData.dummyParent;

        var foodValue = heldObjData.foodValue;

        if(foodValue < 1)
        {
            trace('cannot eat this stuff no food value!!! ${heldObjData.description}');
            return false;
        }

        if(food_store_max - food_store < (foodValue + 1) / 2)
        {
            trace('too full to eat: food_store_max: $food_store_max - food_store: $food_store < ( foodValue: $foodValue + 1 ) / 2');
            return false;
        }

        var countEaten = hasEatenMap[heldObjData.id]; 
        if(countEaten < 0) countEaten = 0;    

        foodValue += ServerSettings.YumBonus;
        foodValue -= countEaten;

        var isCravingEatenObject = heldObjData.id == currentlyCraving;
        if(isCravingEatenObject) foodValue += 1; // craved food give more boni

        var isSuperMeth = foodValue < heldObject.objectData.foodValue / 2;

        if(isSuperMeth) foodValue = Math.ceil(heldObject.objectData.foodValue / 2);

        /*
        if(isSuperMeth && food_store > 0)
        {
            trace('when food value is less then halve it can only be eaten if starving to death: foodValue: $foodValue original food value: ${heldObject.objectData.foodValue} food_store: $food_store');
            return;
        }*/

        var isHoldingYum = isHoldingYum();

        hasEatenMap[heldObjData.id] += 1;

        // eating YUM increases prestige / score while eating MEH reduces it
        if(isHoldingYum)
        {
            if(isCravingEatenObject) yum_multiplier += 2;
            else yum_multiplier += 1;
            doIncreaseFoodValue(heldObjData.id);
        }
        else yum_multiplier -= 1;
        //else if(isHoldingMeh()) yum_multiplier -= 1;

        trace('YUM: ${heldObjData.description} foodValue: $foodValue countEaten: $countEaten');

        // food_store food_capacity last_ate_id last_ate_fill_max move_speed responsible_id
       
        this.last_ate_fill_max = Math.ceil(this.food_store);
        trace('last_ate_fill_max: $last_ate_fill_max');
        this.food_store += foodValue;
        this.just_ate = 1;
        this.last_ate_id = heldObjData.id;
        this.responsible_id = -1; // self
        //this.o_transition_source_id = -1;

        if (food_store > food_store_max)
        {
            this.yum_bonus = food_store - food_store_max;
            food_store = food_store_max;
        } 

        sendFoodUpdate();

        // check if there is a player transition like:
        // 2143 + -1 = 2144 + 0 Banana
        // 1251 + -1 = 1251 + 0 lastUseActor: false Bowl of Stew
        // 1251 + -1 = 235 + 0 lastUseActor: true Bowl of Stew
        if(TransitionHelper.DoChangeNumberOfUsesOnActor(this.heldObject, false, false) == false)
        {
            trace('FOOD: set held object null');
            setHeldObject(null);
        }
        else{
            setHeldObject(this.heldObject);
        }

        SetTransitionData(this.x, this.y);

        Connection.SendUpdateToAllClosePlayers(this);

        this.just_ate = 0;
        this.action = 0;

        return true;    
    }

    /**
        PU
        List of player ids with their display object ids, facing direction, action
        attempt flag, action attempt target position,
        held object ids (in CONTAINER OBJECT FORMAT, see above), 
        whether held origin is valid (1 or 0), origin position on map of that held 
        object (where it was picked up from), 
        transition source object id (or -1) if held object is result of a transition,
        player's current heat value, 
        done_moving_seqNum (to signal destination reached), force flag (to signal
        a move truncated unexpectedly), x,y grid positions of player,
        floating point age in "years", floating point aging rate in sec/year (how many
        seconds it takes the player to age 1 year), and
        floating point move speeds (in grid square widths per second) and clothing
        set, just_ate = 1 or 0 to indicate whether the player just ate what they were 
        holding, the ID of the object they just ate, and the player responsible for this update.

        If facing is 0, then the player's facing direction doesn't change.
        If facing is 1, then they should face right, and -1 to face left.

        action flag is 1 if player is attempting an action, 0 otherwise;

        Heat is the player's warmth between 0 and 1, where 0 is coldest, 1 is hottest,
        and 0.5 is ideal.

        If done_moving_seqNum is > 0, this means the player is stationary at this position (and this is the sequence number of their last move).
        Otherwise, player may still be in the middle of a move (for example, if what
        they are holding decays while they are moving, a PU will be sent with
        done_moving_seqNum set to 0).

        force is usually 0 except in special cases of move truncation where it is 1.
        A player receiving force for itself must snap back to that location
        before continuing to move.
    **/
    public function SetTransitionData(x:Int, y:Int)
    {
        var player = this;

        player.forced = false;
        player.action = 1;        
        player.o_id = this.heldObject.toArray();

        //player.o_transition_source_id = this.newTransitionSource; TODO ??????????????????????????
        player.o_transition_source_id = -1;

        // TODO set right
        // this changes where the client moves the objec from on display
        player.o_origin_x = x;
        player.o_origin_y = y;
        
        player.o_origin_valid = 1; // what is this for???
        
        player.action_target_x = x;
        player.action_target_y = y;
    }

    /*
    CR
    food_id bonus
    #

    Tells player about which food they're currently craving, and how much their
    YUM multiplier will increase when they eat it.
    */

    private function doIncreaseFoodValue(eatenFoodId:Int)
    {
        trace('YUM: ${eatenFoodId}');
        
        if(hasEatenMap[eatenFoodId] > 0) cravings.remove(eatenFoodId);

        var hasEatenKeys = [for(key in hasEatenMap.keys()) key];

        trace('YUM: hasEatenKeys.length: ${hasEatenKeys.length}');

        // restore one food pip if eaten YUM
        if(hasEatenKeys.length < 1) return;

        var random = WorldMap.calculateRandomInt(hasEatenKeys.length -1);
        var key = hasEatenKeys[random];

        //trace('YUM: random: $random hasEatenKeys.length: ${hasEatenKeys.length}');

        var newHasEatenCount = hasEatenMap[key];
        var cravingHasEatenCount = hasEatenMap[currentlyCraving];
        
        if(key != eatenFoodId && WorldMap.calculateRandomFloat() < ServerSettings.YumFoodRestore)
        {
            hasEatenMap[key] -= 1;
            newHasEatenCount = hasEatenMap[key];
            trace('YUM: craving: hasEaten YES!!!: key: $key, ${newHasEatenCount}');

            if(newHasEatenCount < 1 && cravings.contains(key) == false)
            {
                trace('YUM: added craving: key: $key');
                cravings.push(key);
            }
        }
        else
        {
            trace('YUM: craving hasEaten: NO!!!: key: $key, heldObject.id(): ${eatenFoodId}');
        }
            
        newHasEatenCount--;  // A food with full YUM is displayed as +1 craving 
        cravingHasEatenCount--; // A food with full YUM is displayed as +1 craving

        //if(newHasEatenCount >= 0) cravings.remove(eatenFoodId);
        //if(cravingHasEatenCount >= 0) cravings.remove(currentlyCraving);

        if(cravingHasEatenCount < 0 && currentlyCraving != 0 && currentlyCraving == eatenFoodId)
        {            
            trace('YUM: craving: currentlyCraving: $currentlyCraving ${-cravingHasEatenCount}');

            this.connection.send(ClientTag.CRAVING, ['${currentlyCraving} ${-cravingHasEatenCount}']);
        }      
        else
        {
            /*else if(newHasEatenCount < 0)
            {
                this.connection.send(ClientTag.CRAVING, ['$key ${-newHasEatenCount}']);
                currentlyCraving = key;
            }*/
            

            if(cravings.length < 1 || WorldMap.calculateRandomFloat() < ServerSettings.YumNewCravingChance)
            {
                trace('YUM: no new craving / choose random new: Eaten: ${eatenFoodId}');

                currentlyCraving = 0;

                // chose random new craving
                // TODO sort cravinglist by how difficult they are

                var index = 0;
                var iterations = 0;

                for(i in 0...31)
                {
                    iterations = i;
                    
                    index = lastCravingIndex + WorldMap.calculateRandomInt(6 + i) - 3;

                    if(index == lastCravingIndex) index++;

                    if(index < 0) continue;
            
                    if(index >= ObjectData.foodObjects.length) continue;

                    var newObjData = ObjectData.foodObjects[index];

                    if(hasEatenMap[newObjData.id] > 0) continue;

                    break;
                }

                if(iterations >= 30)
                {
                    trace('WARNING: No new random craving found!!!');
                    this.connection.send(ClientTag.CRAVING, ['${currentlyCraving} 0']); 
                    return;
                }

                var newObjData = ObjectData.foodObjects[index];

                if(hasEatenMap.exists(newObjData.id) == false) hasEatenMap[newObjData.id] = -1; // make sure to add it to the cravins and give a little boni

                newHasEatenCount = hasEatenMap[newObjData.id];
                newHasEatenCount--;

                trace('YUM; new random craving: ${newObjData.description} ${newObjData.id} lastCravingIndex: $lastCravingIndex index: $index  newHasEatenCount: ${-newHasEatenCount}');

                lastCravingIndex = index;
                currentlyCraving = newObjData.id;

                this.connection.send(ClientTag.CRAVING, ['${currentlyCraving} ${-newHasEatenCount}']); 
            }
            else
            {
                // chose craving from known craving list
                var random = WorldMap.calculateRandomInt(cravings.length -1);
                var key = cravings[random];
                newHasEatenCount = hasEatenMap[key];
                newHasEatenCount--;
                this.connection.send(ClientTag.CRAVING, ['$key ${-newHasEatenCount}']);
                currentlyCraving = key;

                trace('YUM: new craving: cravingHasEatenCount: $cravingHasEatenCount currentlyCraving: $currentlyCraving ${-newHasEatenCount}');
            }
        }            
    }


    private function doSwitchCloths(clothingSlot:Int) : Bool
    {
        var objClothingSlot = calculateClothingSlot();

        if(objClothingSlot < 0 && clothingSlot < 0) return false;

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

        if(clothingSlot < 0) return false;

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
        
        Connection.SendUpdateToAllClosePlayers(this);

        this.action = 0;

        return true;
    }

    private function calculateClothingSlot() : Int
    {
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
            //trace('clothingSlot:  ${clothingSlot}');
        }

        return objClothingSlot;
    }

    public function specialRemove(x:Int,y:Int,clothing:Int,id:Null<Int>)
    {
        // TODO implement

        Connection.SendUpdateToAllClosePlayers(this);
    }

    public function isHoldingYum() : Bool
    {
        var heldObjData = heldObject.objectData;
        if(heldObjData.dummyParent != null) heldObjData = heldObjData.dummyParent;

        if(heldObjData.foodValue < 1) return false;

        var countEaten = hasEatenMap[heldObjData.id];

        return countEaten < ServerSettings.YumBonus; 
    }

    public function isHoldingMeh() : Bool
    {
        var heldObjData = heldObject.objectData;
        if(heldObjData.dummyParent != null) heldObjData = heldObjData.dummyParent;

        if(heldObjData.foodValue < 1) return false;

        var countEaten = hasEatenMap[heldObjData.id];

        return countEaten > ServerSettings.YumBonus; 
    }

    public function setHeldObject(obj:ObjectHelper)
    {
        if(obj == null) obj = ObjectHelper.readObjectHelper(this, [0]);

        obj.TransformToDummy();

        this.heldObject = obj;
        this.o_id = obj.toArray();
        this.held_yum = isHoldingYum();    
    }

    public function transformHeldObject(id:Int)
    {
        heldObject.setId(id);
        setHeldObject(heldObject);
    }
}