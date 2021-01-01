package openlife.server;
import haxe.display.Server.HaxeModuleMemoryResult;
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

    public var trueAge:Float = ServerSettings.StartingEveAge;

    //food vars
    public var food_store:Float = ServerSettings.GrownUpFoodStoreMax / 2;
    public var food_store_max:Float = ServerSettings.GrownUpFoodStoreMax;
    var last_ate_fill_max:Int = 0;
    public var yum_bonus:Float = 0;
    public var yum_multiplier:Float = 0;

    var hasEatenMap = new Map<Int, Int>();

    // craving
    var currentlyCraving:Int = 0;
    var lastCravingIndex:Int = 0;
    var cravings = new Array<Int>();

    public function new(a:Array<String>)
    {
        super(a);

        this.heldObject = ObjectHelper.readObjectHelper(this, [0]);

        for(i in 0...this.clothingObjects.length)
        {
            this.clothingObjects[i] = ObjectHelper.readObjectHelper(this, [0]);
        }
    }

    public function tx() : Int {return x + gx;}
    public function ty() : Int {return y + gy;}

    // works with coordinates relative to the player
    public function isClose(x:Int, y:Int, distance:Int = 1):Bool
    {    
        return (((this.x - x) * (this.x - x) <= distance * distance) && ((this.y - y) * (this.y - y) <= distance * distance));
    }

    public function getPackpack() : ObjectHelper
    {
        return this.clothingObjects[5];
    }

    public function hasBothShoes() : Bool
    {
        return (this.clothingObjects[2].id != 0 && this.clothingObjects[3].id != 0) ;   
    }

    public function addFood(foodValue:Float)
    {
        this.food_store += foodValue;

        if (food_store > food_store_max)
        {
            this.yum_bonus = food_store - food_store_max;
            food_store = food_store_max;
        } 
    }

    public function CalculateHealthFactor(forSpeed:Bool) : Float
    {
        var health:Float = this.yum_multiplier - this.trueAge  * ServerSettings.MinHealthPerYear;

        var healthFactor:Float; 

        var maxBoni = forSpeed ? 1.2 : 2;
        var maxMali = forSpeed ? 0.8 : 0.5;

        if(health >= 0) healthFactor = (maxBoni  * health + ServerSettings.HealthFactor) / (health + ServerSettings.HealthFactor);
        else healthFactor = (health - ServerSettings.HealthFactor) / ( (1 / maxMali) * health - ServerSettings.HealthFactor);

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

        if(clothingSlot < 0)
        {
            if(doEating(this,this)) return;
        }

        if(doSwitchCloths(clothingSlot)) return;

        doPlaceObjInClothing(clothingSlot);
    }

    //UBABY x y i id#
    /*
    UBABY is a special case of SELF applied to a baby (to feed baby food
	  or add/remove clothing from baby).  Also works on elderly.
      Note that server currently allows UBABY to feed anyone food, but
      only putting clothing on babies and elderly.
      ALSO:  UBABY is used for healing wounded players.
      Essentially, any action where held item is used on another player.
      Should be called UOTHER, but UBABY is used for historical reasons.
      NOTE the alternate call for UBABY with extra id parameter.
      this specifies a specific person to do the action on, if more than one is
	  close to the target tile.
    */
    public function doOnOther(x:Int, y:Int, clothingSlot:Int, playerId:Int)
    {
        this.mutex.acquire();

        if(ServerSettings.debug)
        {
            doOnOtherHelper(x,y,clothingSlot, playerId);
        }
        else{
            try{
                doOnOtherHelper(x,y,clothingSlot, playerId);
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

    public function doOnOtherHelper(x:Int, y:Int, clothingSlot:Int, playerId:Int) : Bool
    {
        trace('doOnOtherHelper: playerId: ${playerId} ${this.o_id[0]} ${heldObject.objectData.description} clothingSlot: $clothingSlot');

        // 838 Dont feed dam drugs! Wormless Soil Pit with Mushroom // 837 Psilocybe Mushroom
        if(heldObject.objectData.isDrugs()) return false;

        var targetPlayer = getPlayerAt(x,y, playerId);

        if(targetPlayer == null)
        {
            trace('doOnOtherHelper: could not find target player!');
        }

        if(this.isClose(targetPlayer.tx() - this.gx , targetPlayer.ty() - this.gy) == false)
        {
            trace('doOnOtherHelper: Targt position is too far away player: ${this.tx()},${this.ty()} target: ${targetPlayer.tx},${targetPlayer.ty}');
            return false; 
        }

        if(clothingSlot < 0)
        {
            if(doEating(this, targetPlayer)) return true;
        }

        return false;

        // TODO
        //if(doSwitchCloths(clothingSlot)) return true;

        //return doPlaceObjInClothing(clothingSlot);
    }

    public static function getPlayerAt(x:Int, y:Int, playerId:Int) : GlobalPlayerInstance
    {
        for(c in Server.server.connections)
        {
            if(c.player.p_id == playerId) return c.player;

            if(playerId <= 0)
            {
                if(c.player.x == x && c.player.y == y) return c.player;
            }
        }

        return null;
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
        var cut_move_speed = Std.int(move_speed * 100) / 100;

        this.connection.send(FOOD_CHANGE,['${Math.ceil(food_store)} ${Std.int(food_store_max)} $last_ate_id $last_ate_fill_max $cut_move_speed $responsible_id ${Math.ceil(yum_bonus)} ${Math.ceil(yum_multiplier)}'], isPlayerAction);
    }

    public static function doEating(playerFrom:GlobalPlayerInstance, playerTo:GlobalPlayerInstance) : Bool
    {
        if (playerFrom.o_id[0] == 0) return false;

        if(playerFrom.age < ServerSettings.MinAgeToEat)
        {
            trace('too young to eat player.age: ${playerFrom.age} < ServerSettings.MinAgeToEat: ${ServerSettings.MinAgeToEat} ');
            return false;
        }

        var heldObjData = playerFrom.heldObject.objectData;
        if(heldObjData.dummyParent != null) heldObjData = heldObjData.dummyParent;

        var foodValue:Float = heldObjData.foodValue;

        trace('FOOD: food_store_max: ${playerTo.food_store_max} food_store: ${playerTo.food_store} foodValue: ${foodValue}');

        if(foodValue < 1)
        {
            trace('cannot eat this stuff no food value!!! ${heldObjData.description}');
            return false;
        }
        
        if(playerTo.food_store_max - playerTo.food_store < Math.ceil(foodValue / 3))
        {
            trace('too full to eat: food_store_max: ${playerTo.food_store_max} - food_store: ${playerTo.food_store} < foodValue: $foodValue  / 3');
            return false;
        }

        var countEaten = playerTo.hasEatenMap[heldObjData.id]; 
        if(countEaten < 0) countEaten = 0;    

        foodValue += ServerSettings.YumBonus;
        foodValue -= countEaten;

        var isCravingEatenObject = heldObjData.id == playerTo.currentlyCraving;
        if(isCravingEatenObject) foodValue += 1; // craved food give more boni

        var isSuperMeh = foodValue < playerFrom.heldObject.objectData.foodValue / 2;

        if(isSuperMeh) foodValue = playerFrom.heldObject.objectData.foodValue / 2;

        /*
        if(isSuperMeh && food_store > 0)
        {
            trace('when food value is less then halve it can only be eaten if starving to death: foodValue: $foodValue original food value: ${heldObject.objectData.foodValue} food_store: $food_store');
            return;
        }*/

        var isHoldingYum = playerFrom.isHoldingYum();

        if(isSuperMeh == false)
        {
            playerTo.hasEatenMap[heldObjData.id] += 1;
            playerTo.doIncreaseFoodValue(heldObjData.id);
        }

        // eating YUM increases prestige / score while eating MEH reduces it
        if(isHoldingYum)
        {
            if(isCravingEatenObject) playerTo.yum_multiplier += 2;
            else playerTo.yum_multiplier += 1;            
        }
        else playerTo.yum_multiplier -= 1;
             
        //else if(isHoldingMeh()) yum_multiplier -= 1;

        trace('YUM: ${heldObjData.description} foodValue: $foodValue countEaten: $countEaten');

        // food_store food_capacity last_ate_id last_ate_fill_max move_speed responsible_id
       
        playerTo.last_ate_fill_max = Math.ceil(playerTo.food_store);
        trace('last_ate_fill_max: ${playerTo.last_ate_fill_max}');
        //this.food_store += foodValue;
        playerTo.just_ate = 1;
        playerTo.last_ate_id = heldObjData.id;
        playerTo.responsible_id = playerFrom.p_id; // -1; // self???
        //this.o_transition_source_id = -1;

        playerTo.addFood(foodValue);

        playerTo.move_speed = MoveHelper.calculateSpeed(playerTo, playerTo.tx(), playerTo.ty());

        playerTo.sendFoodUpdate();

        // check if there is a player transition like:
        // 2143 + -1 = 2144 + 0 Banana
        // 1251 + -1 = 1251 + 0 lastUseActor: false Bowl of Stew
        // 1251 + -1 = 235 + 0 lastUseActor: true Bowl of Stew
        if(TransitionHelper.DoChangeNumberOfUsesOnActor(playerFrom.heldObject, false, false) == false)
        {
            trace('FOOD: set held object null');
            playerFrom.setHeldObject(null);
        }

        playerTo.SetTransitionData(playerTo.x, playerTo.y);

        Connection.SendUpdateToAllClosePlayers(playerTo);

        if(playerFrom != playerTo)
        {
            Connection.SendUpdateToAllClosePlayers(playerFrom);
        }

        playerTo.just_ate = 0;
        playerTo.action = 0;

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
    public function SetTransitionData(x:Int, y:Int, objOriginValid = false)
    {
        var player = this;

        player.forced = false;
        player.action = 1;        
        player.o_id = this.heldObject.toArray();

        //player.o_transition_source_id = this.newTransitionSource; TODO ??????????????????????????
        player.o_transition_source_id = objOriginValid ? -1 : this.heldObject.id;

        // this changes where the client moves the objec from on display
        player.o_origin_x = objOriginValid ? x : 0;
        player.o_origin_y = objOriginValid ? y : 0;
        
        player.o_origin_valid = objOriginValid ? 1 : 0; // if set to 0 no animation is displayed to pick up hold obj from o_origin_x o_origin_y
        
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
        trace('IncreaseFoodValue: ${eatenFoodId}');
        
        if(hasEatenMap[eatenFoodId] > 0) cravings.remove(eatenFoodId);

        var hasEatenKeys = [for(key in hasEatenMap.keys()) key];

        trace('IncreaseFoodValue: hasEatenKeys.length: ${hasEatenKeys.length}');

        // restore one food pip if eaten not super meh
        if(hasEatenKeys.length < 1) return;

        var random = WorldMap.calculateRandomInt(hasEatenKeys.length -1);
        var key = hasEatenKeys[random];

        //trace('IncreaseFoodValue: random: $random hasEatenKeys.length: ${hasEatenKeys.length}');

        var newHasEatenCount = hasEatenMap[key];
        var cravingHasEatenCount = hasEatenMap[currentlyCraving];
        
        if(key != eatenFoodId && WorldMap.calculateRandomFloat() < ServerSettings.YumFoodRestore)
        {
            hasEatenMap[key] -= 1;
            newHasEatenCount = hasEatenMap[key];
            trace('IncreaseFoodValue: craving: hasEaten YES!!!: key: $key, ${newHasEatenCount}');

            if(newHasEatenCount < 1 && cravings.contains(key) == false)
            {
                trace('IncreaseFoodValue: added craving: key: $key');
                cravings.push(key);
            }
        }
        else
        {
            trace('IncreaseFoodValue: craving hasEaten: NO!!!: key: $key, heldObject.id(): ${eatenFoodId}');
        }
            
        newHasEatenCount--;  // A food with full YUM is displayed as +1 craving 
        cravingHasEatenCount--; // A food with full YUM is displayed as +1 craving

        //if(newHasEatenCount >= 0) cravings.remove(eatenFoodId);
        //if(cravingHasEatenCount >= 0) cravings.remove(currentlyCraving);

        if(cravingHasEatenCount < 0 && currentlyCraving != 0 && currentlyCraving == eatenFoodId)
        {            
            trace('IncreaseFoodValue: craving: currentlyCraving: $currentlyCraving ${-cravingHasEatenCount}');

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
                trace('IncreaseFoodValue: no new craving / choose random new: Eaten: ${eatenFoodId}');

                currentlyCraving = 0;

                // chose random new craving
                // TODO sort cravinglist by how difficult they are

                var index = 0;
                var foundNewCraving = false;

                for(i in 0...31)
                {
                    index = lastCravingIndex + WorldMap.calculateRandomInt(6 + i) - 3;

                    if(index == lastCravingIndex) index++;

                    if(index < 0) continue;
            
                    if(index >= ObjectData.foodObjects.length) continue;

                    var newObjData = ObjectData.foodObjects[index];

                    if(hasEatenMap[newObjData.id] > 0) continue;

                    foundNewCraving = true;

                    break;
                }

                if(foundNewCraving == false)
                {
                    trace('WARNING: No new random craving found!!!');
                    this.connection.send(ClientTag.CRAVING, ['${currentlyCraving} 0']); 
                    return;
                }

                var newObjData = ObjectData.foodObjects[index];

                if(hasEatenMap.exists(newObjData.id) == false) hasEatenMap[newObjData.id] = -1; // make sure to add it to the cravins and give a little boni

                newHasEatenCount = hasEatenMap[newObjData.id];
                newHasEatenCount--;

                trace('IncreaseFoodValue; new random craving: ${newObjData.description} ${newObjData.id} lastCravingIndex: $lastCravingIndex index: $index  newHasEatenCount: ${-newHasEatenCount}');

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

                trace('IncreaseFoodValue: new craving: cravingHasEatenCount: $cravingHasEatenCount currentlyCraving: $currentlyCraving ${-newHasEatenCount}');
            }
        }            
    }


    private function doSwitchCloths(clothingSlot:Int) : Bool
    {
        var objClothingSlot = calculateClothingSlot();
        trace('self:o_id: ${this.o_id[0]} helobj.id: ${this.heldObject.id} clothingSlot: $clothingSlot objClothingSlot: $objClothingSlot');

        if(objClothingSlot < 0 && this.heldObject.id != 0) return false;

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

        var tmpObj = this.clothingObjects[clothingSlot];
        this.clothingObjects[clothingSlot] = this.heldObject;
        this.setHeldObject(tmpObj);

        // switch clothing if there is a clothing on this slot
        //var tmp = Std.parseInt(array[clothingSlot]);
        array[clothingSlot] = '${clothingObjects[clothingSlot].toString()}';
        this.clothing_set = '${array[0]};${array[1]};${array[2]};${array[3]};${array[4]};${array[5]}';
        trace('this.clothing_set: ${this.clothing_set}');

        this.action = 0;
 
        SetTransitionData(x, y);
        
        Connection.SendUpdateToAllClosePlayers(this);

        //this.action = 0;

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
                case "h": objClothingSlot = 0;      // head
                case "t": objClothingSlot = 1;      // torso
                case "s": objClothingSlot = 2;      // shoes
                //case "s": objClothingSlot = 3;    // shoes
                case "b": objClothingSlot = 4;      // skirt / trouser
                case "p": objClothingSlot = 5;      // backpack
            }

            trace('objectData.clothing: ${objectData.clothing}');
            trace('objClothingSlot:  ${objClothingSlot}');
            //trace('clothingSlot:  ${clothingSlot}');
        }

        return objClothingSlot;
    }

    public function doPlaceObjInClothing(clothingSlot:Int, isDrop:Bool = false) : Bool
    {
        if(clothingSlot < 0 ||  clothingSlot >= this.clothingObjects.length) return false;

        var clothing = this.clothingObjects[clothingSlot];

        if(TransitionHelper.DoContainerStuffOnObj(this, clothing, isDrop) == false) return false;

        setInClothingSet(clothingSlot);

        if(isDrop) return true; // currently flase if called from drop

        SetTransitionData(this.x, this.y);

        Connection.SendUpdateToAllClosePlayers(this);

        return true;
    }

    private function setInClothingSet(clothingSlot:Int)
    {
        var array = this.clothing_set.split(";");

        if(array.length < 6)
        {
            trace('Clothing string missing slots: ${this.clothing_set}' );
        }  

        array[clothingSlot] = '${clothingObjects[clothingSlot].toString()}';
        this.clothing_set = '${array[0]};${array[1]};${array[2]};${array[3]};${array[4]};${array[5]}';
    }

    /*
    SREMV x y c i#

    SREMV is special case of removing an object contained in a piece of worn 
      clothing.
      c specifies the clothing slot to remove from:  0=hat, 1=tunic, 
         2=frontShoe, 3=backShoe, 4=bottom, 5=backpack
      i specifies the index of the container item to remove, or -1 to
	  remove top of stack.
    */
    // SREMV -5 6 5 -1 remnove from backpack
    public function specialRemove(x:Int,y:Int,clothingSlot:Int,index:Null<Int>) : Bool
    {
        // TODO implement
        trace("SPECIAL REMOVE:");

        if(clothingSlot < 0) return false;

        var container = this.clothingObjects[clothingSlot];

        if(container.containedObjects.length < 1) return false;

        this.mutex.acquire();

        this.setHeldObject(container.removeContainedObject(index));

        setInClothingSet(clothingSlot);

        SetTransitionData(x,y);
        
        /*
        this.action = 1;
        this.action_target_x = x;
        this.action_target_y = y;
        this.o_origin_x = x;
        this.o_origin_y = y;
        this.o_origin_valid = 0; // TODO ???
        */

        trace('this.clothing_set: ${this.clothing_set}');

        this.mutex.release();

        Connection.SendUpdateToAllClosePlayers(this);

        //this.connection.send(FRAME);

        return true;
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
        this.heldObject = obj;

        MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed();
    }

    public function MakeSureHoldObjIdAndDummyIsSetRightAndNullObjUsed()
    {
        var obj = this.heldObject;

        if(obj == null)
        {
            obj = ObjectHelper.readObjectHelper(this, [0]);
            this.heldObject = obj;
        } 

        obj.TransformToDummy();
        this.o_id = obj.toArray();
        this.held_yum = isHoldingYum(); 
    }

    public function transformHeldObject(id:Int)
    {
        var toObjData = ObjectData.getObjectData(id);
        if(toObjData.dummyParent != null) toObjData = toObjData.dummyParent;

        var fromObjData = heldObject.objectData;
        if(fromObjData.dummyParent != null) fromObjData = fromObjData.dummyParent;

        
        if(toObjData.id != fromObjData.id)
        {
            heldObject.numberOfUses = 1;
            //TODO set to max numberOfUses??? heldObject.numberOfUses = heldObject.objectData

            trace('transformHeldObject: ${fromObjData.id} --> ${toObjData.id} / numberOfUses set to 1');
        }

        trace('transformHeldObject: heldObject.numberOfUses: ${heldObject.numberOfUses}');

        heldObject.id = id;
        setHeldObject(heldObject);
    }

    public function doDeath(deathReason:String)
    {
        // TODO calculate score
        // TODO set coordinates player based
        ServerSettings.startingGx = this.tx();
        ServerSettings.startingGy = this.ty();

        this.age = this.trueAge; // bad health and starving can influence health, so setback true time a player lifed so that he sees in death screen
        this.reason = deathReason;
        this.deleted = true;

        //this.connection.die();

        placeGrave();
    }

    public function placeGrave()
    {
        var world = WorldMap.world;
        var grave = new ObjectHelper(this, 87); // 87 = Fresh Grave

        if(this.heldObject != null)
        {
            grave.containedObjects.push(this.heldObject);
            this.setHeldObject(null);
        }

        // place the clothings in the grave, but not need to remove them from the player, since he is dead... //clothing_set:String = "0;0;0;0;0;0";
        for(obj in this.clothingObjects)
        {
            if(obj.id == 0) continue;

            grave.containedObjects.push(obj);
        }

        if(tryPlaceGrave(this.tx(), this.ty(), grave)) return; 

        var distance = 1;
        
        for(i in 1...10000)
        {
            distance = Math.ceil(i / (20 * distance * distance)); 

            trace('rand: ${world.randomInt(distance * 2) - distance}');

            var tmpX = this.tx() + world.randomInt(distance * 2) - distance;
            var tmpY = this.ty() + world.randomInt(distance * 2) - distance;

            if(tryPlaceGrave(tmpX, tmpY, grave)) return; 
        }

        trace('WARNING: could not place any grave for player: ${this.p_id}');
    }

    function tryPlaceGrave(x:Int, y:Int, grave:ObjectHelper) : Bool
    {
        var world = Server.server.map;

        var obj = world.getObjectHelper(x, y);

        if(obj.id == 0)
        {
            world.setObjectHelper(x, y, grave);

            //Connection.SendUpdateToAllClosePlayers(this);

            Connection.SendMapUpdateToAllClosePlayers(x, y, grave.toArray());

            return true;
        }
        else if(obj.objectData.containable)
        {
            grave.containedObjects.push(obj);

            world.setObjectHelper(x, y, grave);

            //Connection.SendUpdateToAllClosePlayers(this);

            Connection.SendMapUpdateToAllClosePlayers(x, y, grave.toArray());

            return true;
        }

        //trace('Do death: could not place grave at: ${obj.description()}');

        return false;
    }

    // insulation reaches from 0 to 2
    public function calculateClothingInsulation() : Float
    {
        var clothingInsulation:Float = 0;
        
        for(clothing in this.clothingObjects)
        {
            if(clothing.id == 0) continue;
             
            clothingInsulation += clothing.objectData.getInsulation();
            
            //trace('insulation: ${clothing.description} ${clothing.objectData.getInsulation()}');
        }

        //trace('clothingInsulation: $clothingInsulation');

        return clothingInsulation;
    }
}