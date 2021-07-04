package openlife.auto;

import openlife.data.object.ObjectHelper;
import openlife.data.object.ObjectData;
import openlife.data.transition.TransitionData;
import openlife.data.object.player.PlayerInstance;
import openlife.data.map.MapData;
import openlife.data.Pos;
import openlife.auto.Pathfinder.Coordinate;
import haxe.ds.Vector;

using StringTools;
using openlife.auto.AiHelper;


class Ai
{
    final RAD:Int = MapData.RAD; // search radius

    public var playerInterface:PlayerInterface;

    var done = false;

    var time:Float = 5;

    var foodTarget:ObjectHelper = null; 
    var dropTarget:ObjectHelper = null;
    var useTarget:ObjectHelper = null;

    var itemToCraft:IntemToCraft = new IntemToCraft();

    //var berryHunter:Bool = false;
    var isHungry = false;
    var doingAction = false;

    var playerToFollow:PlayerInstance;

    public function new(player:PlayerInterface) 
    {
        this.playerInterface = player;
    }

    public function say(player:PlayerInstance,curse:Bool,text:String) 
    {
        var myPlayer = playerInterface.getPlayerInstance();
        var world = playerInterface.getWorld();
        //trace('im a super evil bot!');

        //trace('ai3: ${myPlayer.p_id} player: ${player.p_id}');

        if (myPlayer.p_id == player.p_id) return;

        //trace('im a evil bot!');

        trace('AI ${text}');

        if (text.contains("TRANS")) 
        {
            trace('AI look for transitions: ${text}');

            var objectIdToSearch = 273; // 273 = Cooked Carrot Pie // 250 = Hot Adobe Oven

            AiHelper.SearchTransitions(playerInterface, objectIdToSearch);
        }

        if (text.contains("HELLO")) 
        {
            //HELLO WORLD

            //trace('im a nice bot!');

            playerInterface.say("HELLO WORLD");
        }
        if (text.contains("JUMP")) 
        {
            playerInterface.say("JUMP");
            playerInterface.jump();
        }
        if (text.contains("MOVE"))
        {
            playerInterface.Goto(player.tx() + 1 - myPlayer.gx, player.ty() - myPlayer.gy);
            playerInterface.say("YES CAPTAIN");
        }
        if (text.contains("FOLLOW ME"))
        {
            playerToFollow = player;
            playerInterface.Goto(player.tx() + 1 - myPlayer.gx, player.ty() - myPlayer.gy);
            playerInterface.say("SURE CAPTAIN");
        }
        if (text.contains("STOP"))
        {
            playerToFollow = null;
            playerInterface.say("YES CAPTAIN");
        }
        if (text.contains("EAT!"))
        {
            searchBestFood();
            searchFoodAndEat();
            playerInterface.say("YES CAPTAIN");
        }
        if (text.contains("MAKE!"))
        {
            craftItem(292); // basket
            playerInterface.say("YES CAPTAIN");
        }
    }

    public function searchFoodAndEat()
    {
        var myPlayer = playerInterface.getPlayerInstance();
        foodTarget = searchBestFood();
        if(foodTarget != null) playerInterface.Goto(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
    }

    public function dropHeldObject() 
    {
        var myPlayer = playerInterface.getPlayerInstance();
        
        if(myPlayer.heldObject.id == 0) return;

        var emptyTileObj = playerInterface.GetClosestObjectById(0); // empty
        dropTarget = emptyTileObj;

        trace('Drop ${emptyTileObj.tx} ${emptyTileObj.ty} $emptyTileObj');
        // x,y is relativ to birth position, since this is the center of the universe for a player
        //if(emptyTileObj != null) playerInterface.drop(emptyTileObj.tx - myPlayer.gx, emptyTileObj.ty - myPlayer.gy);
    }

    // do time stuff here is called from TimeHelper
    public function doTimeStuff(timePassedInSeconds:Float) 
    {
        // TODO put in again if testing AI
        // time -= timePassedInSeconds;

        if(time > 0) return;
        
        var world = playerInterface.getWorld();
        var myPlayer = playerInterface.getPlayerInstance();

        if(playerInterface.isMoving()) return;

        if(dropTarget != null && playerInterface.isMoving() == false)
        {
            var distance = playerInterface.CalculateDistanceToObject(dropTarget);

            if(distance > 1)
            {
                playerInterface.Goto(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);
            }
            else
            {
                playerInterface.drop(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);
                dropTarget = null;
                time = 0.5;
            }
            
            return;
        }       

        if(foodTarget != null && playerInterface.isMoving() == false)
        {
            var distance = playerInterface.CalculateDistanceToObject(foodTarget);

            if(distance > 1)
            {
                playerInterface.Goto(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
            }
            else
            {
                //trace('${foodTarget.tx} - ${myPlayer.tx()}, ${foodTarget.ty} - ${myPlayer.ty()}');

                // TODO eat food if held food is eatable
                var heldObjectIsEatable = myPlayer.heldObject.objectData.foodValue > 0;
                

                if(myPlayer.heldObject.id != 0)
                {
                    if(heldObjectIsEatable)
                    {
                        playerInterface.self(); // eat

                        trace('AI: Eat: held: ${ myPlayer.heldObject.description} food: ${foodTarget.description} foodTarget.numberOfUses ${foodTarget.numberOfUses} emptyFood: ${myPlayer.food_store_max - myPlayer.food_store} < 3)');

                        time = 0.2;
                        foodTarget = null;
                        return;
                    }
                    else
                    {
                        dropHeldObject();

                        trace('AI: drop held object to eat');
                        return;
                    }                                            
                }

                // pickup food from floor
                // x,y is relativ to birth position, since this is the center of the universe for a player
                var done = playerInterface.use(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy); 
                
                var oldNumberOfUses = foodTarget.numberOfUses;

                heldObjectIsEatable = myPlayer.heldObject.objectData.foodValue > 0;
                if(heldObjectIsEatable) playerInterface.self(); // eat

                trace('AI: Eat: held: ${ myPlayer.heldObject.description} food: ${foodTarget.description} foodTarget.numberOfUses ${foodTarget.numberOfUses} == oldNumberOfUses $oldNumberOfUses || emptyFood: ${myPlayer.food_store_max - myPlayer.food_store} < 3)');

                if(foodTarget.numberOfUses == oldNumberOfUses || myPlayer.food_store_max - myPlayer.food_store < 6)
                {
                    trace('AI: Eat: set foodTarget to null');
                    foodTarget = null;
                }

                dropHeldObject();

                time = 0.3;
            }       
            
            return;
        } 

        if(foodTarget == null && useTarget != null && playerInterface.isMoving() == false)
        {
            var distance = playerInterface.CalculateDistanceToObject(useTarget);

            trace('AI: ${useTarget.description} ${useTarget.tx} ${useTarget.ty} distance: $distance');

            if(distance > 1)
            {
                playerInterface.Goto(useTarget.tx - myPlayer.gx, useTarget.ty - myPlayer.gy);
            }
            else
            {
                // x,y is relativ to birth position, since this is the center of the universe for a player
                var done = playerInterface.use(useTarget.tx - myPlayer.gx, useTarget.ty - myPlayer.gy);

                trace('AI: ${useTarget.description} done: $done');

                time = 0.2;
                useTarget = null;
            }
        }

        /*if(time < 0 && foodTarget == null && useTarget == null && playerInterface.isMoving() == false)
        {
            if(myPlayer.heldObject.id == 33) // 33 Stone // 34 Sharp Stone
            {
                // make rock sharp
                useTarget = playerInterface.GetClosestObjectById(32); // 32 Big Hard Rock   
                if(useTarget != null) trace('AI: new useTarget ${useTarget.description}');
            }
            else if(myPlayer.heldObject.id != 34) // 33 Stone // 34 Sharp Stone
            {
                useTarget = playerInterface.GetClosestObjectById(34);
                
                if(useTarget != null) trace('AI: new useTarget ${useTarget.description}');

                if(useTarget == null)
                {
                    trace('AI: no new useTarget found! Sharp Stone');

                    if(myPlayer.heldObject.id != 33) // 33 Stone // 34 Sharp Stone
                    {
                        useTarget = playerInterface.GetClosestObjectById(33); // 33 Stone

                        if(useTarget == null)
                        {
                            trace('AI: no new useTarget found! Stone');
                        }
                    }
                }
            }
            //useTarget = null;
        }*/

        if(time < 0 && foodTarget == null && useTarget == null && itemToCraft != null && itemToCraft.transActor == null && playerInterface.isMoving() == false)
        {
            craftItem(292);
        }

        if(itemToCraft.transActor != null)
        {
            var isHoldingTransActor = itemToCraft.transActor.parentId == myPlayer.heldObject.parentId;

            if(time < 0 && foodTarget == null && isHoldingTransActor && itemToCraft.transTarget != null && playerInterface.isMoving() == false)
            {
                // is holding transActor, so set use target to trans target
                useTarget = itemToCraft.transTarget;
                itemToCraft.transActor = null;
                trace('AI: craft: set usetarget to: ${itemToCraft.transTarget.description}');
            }
        }

        if(time < 0 && foodTarget == null && useTarget == null && itemToCraft.transActor == null && playerInterface.isMoving() == false)
        {
        }

        if(time < 0 && foodTarget == null && playerInterface.isMoving() == false)
        {
            if(playerToFollow != null)
            {
                if(playerInterface.CalculateDistanceToPlayer(playerToFollow) > 2)
                {
                    playerInterface.Goto(playerToFollow.tx() + 1 - myPlayer.gx, playerToFollow.ty() - myPlayer.gy);
                }
            }
            else
            {
                playerToFollow = world.getClosestPlayer(20, true);

                if(playerToFollow != null) trace('AI: follow player ${playerToFollow.p_id}');
            }
        } 

        if(time < 0)
        {
            time = 1;

            isHungry = myPlayer.food_store < 15;

            if(isHungry && foodTarget == null) searchFoodAndEat();

            playerInterface.say('${playerInterface.getPlayerInstance().food_store}');
        }

        // TODO if hungry
        // look for close berrybush

        
        // move to berrybush
        // if close to berrybush eat

        //if(done) return;

        //done = true;

        //var transitionsForObject = searchTransitions(273);



        // TODO look if any of the objects you see is in transitionsForObject
        // TODO if none object is found or if the needed steps from the object you found are too high, search for a better object
        // TODO consider too look for a natural spawned object with the fewest steps on the list
        // TODO how to handle if you have allready some of the needed objects ready... 
    }

    private function craftItem(objId:Int)
    {
        var player = playerInterface.getPlayerInstance();
        if(player.heldObject.id != 0) return dropHeldObject();
        
        var transitionsForObject = playerInterface.SearchTransitions(292); // 292 basket
        var itemToCraft = searchBestObjectForCrafting(transitionsForObject);

        if(player.heldObject.parentId == itemToCraft.transActor.parentId)
        {

        }

        useTarget = itemToCraft.transActor;

        var trans = transitionsForObject[useTarget.parentId];
        var transition = trans.bestTransition;
        trace('AI: craft' + transition);


    }

    private function searchBestObjectForCrafting(transitionsForObject:Map<Int, TransitionForObject>) : IntemToCraft
    {
        var itemToCraft = new IntemToCraft();
        var player = playerInterface.getPlayerInstance();
        var baseX = player.tx();
        var baseY = player.ty();
        var world = playerInterface.getWorld();
        //var bestObj = null;
        var bestDistance = 0.0;
        var bestSteps = 0;
        var radius = RAD;

        for(ty in baseY - radius...baseY + radius)
        {
            for(tx in baseX - radius...baseX + radius)
            {
                var objData = world.getObjectDataAtPosition(tx, ty);

                if(objData.id == 0) continue;            
                if(objData.dummyParent != null) objData = objData.dummyParent; // use parent objectdata

                // check if object can be used to craft item
                var trans = transitionsForObject[objData.id];
                if(trans == null) continue;  

                var steps = trans.steps;        
                var obj = world.getObjectHelper(tx, ty);
                var targetObj = playerInterface.GetClosestObjectById(trans.bestTransition.targetID, obj);

                if(targetObj == null)
                    trace('ai: craft: steps: $steps actor: ${obj.description} target: not found!!!'); // + trans.bestTransition);
                else 
                    trace('ai: craft: steps: $steps actor: ${obj.description} target: ${targetObj.id} ${targetObj.description}'); // + trans.bestTransition);

                if(targetObj == null) continue; // TODO check if target can be crafted
                
                
                
                var distance = AiHelper.CalculateDistance(baseX, baseY, obj.tx, obj.ty);
                //trace('search food: best $bestDistance dist $distance ${obj.description}');

                if(itemToCraft.transActor == null || steps < bestSteps || (steps == bestSteps && distance < bestDistance))
                {
                    itemToCraft.transActor = obj;
                    itemToCraft.transTarget = targetObj;                    
                    bestSteps = steps;
                    bestDistance = distance;
                }
            }
        }

        if(itemToCraft.transActor !=null) trace('ai: craft: steps: $bestSteps bestActor: ${itemToCraft.transActor.description} target: ${itemToCraft.transTarget.id} ${itemToCraft.transTarget.description}');

        return itemToCraft;
    }

    private function searchBestFood() : ObjectHelper
    {
        var player = playerInterface.getPlayerInstance();
        var baseX = player.tx();
        var baseY = player.ty();
        var world = playerInterface.getWorld();
        var bestFood = null;
        var bestDistance = 0.0;

        var radius = RAD;
        
        // TODO consider current food vlaue cravings

        for(ty in baseY - radius...baseY + radius)
        {
            for(tx in baseX - radius...baseX + radius)
            {
                var objData = world.getObjectDataAtPosition(tx, ty);

                if(objData.dummyParent !=null) objData = objData.dummyParent; // use parent objectdata

                //var distance = calculateDistance(baseX, baseY, obj.tx, obj.ty);
                //trace('search food $tx, $ty: foodvalue: ${objData.foodValue} bestdistance: $bestDistance distance: $distance ${obj.description}');

                //var tmp = ObjectData.getObjectData(31);
                //trace('berry food: ${tmp.foodValue}');

                if(objData.foodValue > 0 || objData.foodFromTarget != null)                    
                {
                    var obj = world.getObjectHelper(tx, ty);
                    var distance = AiHelper.CalculateDistance(baseX, baseY, obj.tx, obj.ty);
                    //trace('search food: best $bestDistance dist $distance ${obj.description}');

                    if(bestFood == null || distance < bestDistance)
                    {
                        bestFood = obj;
                        bestDistance = distance;
                    }
                }
            }
        }

        if(bestFood !=null) trace('bestfood: $bestDistance ${bestFood.description}');

        return bestFood;
    }

    // is called once a movement is finished (client side it must be called manually after a PlayerUpdate)
    public function finishedMovement()
    {

    }

    public function emote(player:PlayerInstance,index:Int)
    {

    }

    public function playerUpdate(player:PlayerInstance)
    {
        
    }

    public function mapUpdate(targetX:Int,targetY:Int,isAnimal:Bool=false) 
    {
        
    }

    public function playerMove(player:PlayerInstance,targetX:Int,targetY:Int)
    {

    }
    public function dying(sick:Bool)
    {

    }
}

class IntemToCraft
{
    public var transActor:ObjectHelper = null;
    public var transTarget:ObjectHelper = null;

    public function new() {}
}