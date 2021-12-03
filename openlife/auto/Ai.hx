package openlife.auto;

import openlife.settings.ServerSettings;
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

    var itemToCraft:IntemToCraft = null; // new IntemToCraft();

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
        time -= timePassedInSeconds;

        if(time > 0) return;
        time += 0.5; // minimum AI reacting time
        
        //trace('AI:1');
        if(playerInterface.isMoving()) return;

        checkIsHungry();

        //trace('AI:2');

        if(isMovingToPlayer()) return;

        if(playerToFollow == null) return; // Do stuff only if close to player TODO remove if testing AI without player

        //trace('AI:3');

        if(isDropingItem()) return;
        //trace('AI:4');
        if(isEating()) return;
        //trace('AI:5');
        if(isUsingItem()) return;
        //trace('AI:6');

        if(itemToCraft != null && itemToCraft.countDone >= itemToCraft.count) return;
        
        craftItem(292, 20); // 292 basket
        //craftItem(34,1); // 34 sharpstone
        //craftItem(124); // Reed Bundle
        //craftItem(224); // Harvested Wheat
        //craftItem(58); // Thread
    }

    // TODO consider held object / backpack / contained objects
    // TODO consider if object is reachable
    // TODO reconsider closest objects after reached first object
    // TODO store transitions for crafting to have faster lookup
    // TODO consider too look for a natural spawned object with the fewest steps on the list
    private function craftItem(objId:Int, count:Int = 1) : Bool
    {
        var player = playerInterface.getPlayerInstance();

        if(itemToCraft != null && itemToCraft.transActor != null && player.heldObject.parentId == itemToCraft.transActor.parentId)
        {
            itemToCraft.transActor = null; // actor is allready in the hand
            useTarget = itemToCraft.transTarget;
            return true;
        }

        if(player.heldObject.id != 0)
        {
             dropHeldObject();
             return true;
        }

        var countDone = 0;
        var countTransitionsDone = 0;
        var transitionsByObjectId = null;

        if(itemToCraft != null && itemToCraft.itemToCraft.parentId == objId)
        {
            countDone = itemToCraft.countDone;
            countTransitionsDone = itemToCraft.countTransitionsDone;
            transitionsByObjectId = itemToCraft.transitionsByObjectId;

            // reset objects so that it can be filled again
            for(trans in transitionsByObjectId)
            {
                trans.closestObject = null;
                trans.closestObjectDistance = -1;
                trans.secondObject = null;
                trans.closestObjectDistance = -1;
            }
        }
        
        if(transitionsByObjectId == null) transitionsByObjectId = playerInterface.SearchTransitions(objId); 
        
        itemToCraft = searchBestObjectForCrafting(objId, transitionsByObjectId);
        itemToCraft.itemToCraft = ObjectData.getObjectData(objId);
        itemToCraft.count = count;
        itemToCraft.countDone = countDone;
        itemToCraft.countTransitionsDone = countTransitionsDone;
        itemToCraft.transitionsByObjectId = transitionsByObjectId;

        useTarget = itemToCraft.transActor;

        if(itemToCraft.transActor == null)
        {
            trace('AI: craft: did not find any item in search radius for crafting!');
            return false;
        }

        trace(itemToCraft.transActor.description);

        //var trans = transitionsForObject[useTarget.parentId];
        //var transition = trans.bestTransition;
        trace('AI: craft');
        //trans.traceTransition("AI: craft: ", true, true);
        //transition.traceTransition("AI: best craft: ", true, true);

        return true;
    }

    private function searchBestObjectForCrafting(objToCraftId:Int, transitionsForObject:Map<Int, TransitionForObject>) : IntemToCraft
    {
        var itemToCraft = new IntemToCraft();
        var world = playerInterface.getWorld();
        var player = playerInterface.getPlayerInstance();
        var baseX = player.tx();
        var baseY = player.ty();
        var bestDistance = 0.0;
        var bestSteps = 0;
        var bestTrans = null; 
        var radius = ServerSettings.AiSearchRadius;

        // go through all close by objects and map them to the best transition
        for(ty in baseY - radius...baseY + radius)
        {
            for(tx in baseX - radius...baseX + radius)
            {
                var objData = world.getObjectDataAtPosition(tx, ty);

                if(objData.id == 0) continue;            
                if(objData.dummyParent != null) objData = objData.dummyParent; // use parent objectdata

                // check if object can be used to craft item
                var trans = transitionsForObject[objData.id];
                if(trans == null) continue;  // object is not useful for crafting wanted object 

                var steps = trans.steps;        
                var obj = world.getObjectHelper(tx, ty);                
                var objDistance = playerInterface.CalculateDistanceToObject(obj);
                
                if(trans.closestObject == null || trans.closestObjectDistance > objDistance)
                {
                    trans.secondObject = trans.closestObject;
                    trans.secondObjectDistance = trans.closestObjectDistance;

                    trans.closestObject = obj;
                    trans.closestObjectDistance = objDistance;
                    
                    continue;
                }
                
                if(trans.secondObject == null || trans.secondObjectDistance > objDistance)
                {
                    trans.secondObject = obj;
                    trans.secondObjectDistance = objDistance;

                    continue;
                }                        
            }
        }

        // search for the best doable transition with actor and target
        for(trans in transitionsForObject)
        {
            if(trans.closestObject == null) continue;

            var bestTargetTrans = null; 
            var bestTargetObject = null; 
            var bestTargetDistance = -1.0;
            var bestTargetSteps = -1;

            //  search for the best doable transition with target
            for(targetTrans in trans.transitions)                
            {
                var actorID = targetTrans.bestTransition.actorID;
                var targetID = targetTrans.bestTransition.targetID;

                // check if there are allready two of this // TODO if only one is needed skip second
                var tmpWanted = transitionsForObject[targetTrans.wantedObjId];
                if(tmpWanted != null && targetTrans.wantedObjId != objToCraftId && tmpWanted.closestObjectDistance > -1 && tmpWanted.closestObjectDistance > -1) continue;

                // check if there are allready two of this // TODO if only one is needed skip second
                //var tmpNewTarget = transitionsForObject[targetTrans.bestTransition.newTargetID];
                //if(tmpNewTarget != null && targetTrans.bestTransition.newTargetID != objToCraftId && tmpNewTarget.closestObjectDistance > -1 && tmpNewTarget.closestObjectDistance > -1) continue;


                if(targetID == 123 ) trace('TEST1 ');
                if(actorID != 0 && actorID != trans.closestObject.parentId) continue;
                if(targetID == 123 ) trace('TEST2 ');

                var tmpObject = transitionsForObject[targetTrans.bestTransition.targetID];

                if(tmpObject == null) continue;

                if(tmpObject.closestObject == null) continue;

                var tmpDistance = tmpObject.closestObjectDistance;
                var tmpTargetObject = tmpObject.closestObject;

                if(targetID == 123 ) trace('TEST3 ');

                if(targetTrans.bestTransition.actorID == targetTrans.bestTransition.targetID) // like using two milkweed
                {
                    //trace('AI: using two ' + targetTrans.bestTransition.actorID);
                    if(tmpObject.secondObject == null) continue;

                    //trace('AI: using two222');

                    tmpDistance = tmpObject.secondObjectDistance;
                    tmpTargetObject = tmpObject.secondObject;
                }

                if(targetID == 123 ) trace('TEST4 ');

                var steps = targetTrans.steps;

                if(bestTargetTrans == null || bestTargetSteps > steps || (bestTargetSteps == steps && tmpDistance < bestTargetDistance))
                {
                    bestTargetTrans = targetTrans;
                    bestTargetDistance = tmpDistance;
                    bestTargetSteps = steps;
                    bestTargetObject = tmpTargetObject;

                    if(targetID == 123 ) trace('TEST5 ${tmpTargetObject.description}');
                }
            }

            if(bestTargetObject == null) continue;

            //var targetObject = bestTargetObject.closestObject;

            if(bestTargetObject == null) continue;

            //var steps = trans.steps;
            var obj = trans.closestObject;
            var distance = trans.closestObjectDistance + bestTargetDistance; // actor plus target distance

            if(itemToCraft.transActor == null || bestTargetSteps < bestSteps || (bestTargetSteps == bestSteps && distance < bestDistance))
            {
                itemToCraft.transActor = obj;
                itemToCraft.transTarget = bestTargetObject;                    
                bestSteps = bestTargetSteps;
                bestDistance = distance;
                bestTrans = bestTargetTrans;

                if(bestTargetObject.parentId == 50) trace('TEST6 actor: ${obj.description} target: ${bestTargetObject.description} ');
            }
        }

        if(itemToCraft.transActor != null) trace('ai: craft: ' + bestTrans.getDesciption());
        if(itemToCraft.transActor != null) trace('ai: craft: steps: $bestSteps Distance: $bestDistance bestActor: ${itemToCraft.transActor.description} / target: ${itemToCraft.transTarget.id} ${itemToCraft.transTarget.description} ' + bestTrans.bestTransition.getDesciption(true));
        if(itemToCraft.transActor != null) playerInterface.say('Goto ' + itemToCraft.transActor.description );

        return itemToCraft;
    }

    private function isMovingToPlayer() : Bool
    {
        if(playerToFollow == null)
        {
            playerToFollow = playerInterface.getWorld().getClosestPlayer(20, true);

            if(playerToFollow == null) return false;
            
            trace('AAI: follow player ${playerToFollow.p_id}');
        }

        if(playerInterface.CalculateDistanceToPlayer(playerToFollow) > 25)
        {
            trace('AAI: goto player');

            var myPlayer = playerInterface.getPlayerInstance();
            playerInterface.Goto(playerToFollow.tx() + 1 - myPlayer.gx, playerToFollow.ty() - myPlayer.gy);
            return true;
        }

        return false;
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

    

    // returns true if in process of dropping item
    private function isDropingItem() : Bool
    {
        if(dropTarget == null) return false; 
        if(playerInterface.isMoving()) return true;

        var distance = playerInterface.CalculateDistanceToObject(dropTarget);
        var myPlayer = playerInterface.getPlayerInstance();

        if(distance > 1)
        {
            trace('AAI: goto drop');
            playerInterface.Goto(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);
        }
        else
        {
            trace('AAI: drop');

            playerInterface.drop(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);
            
            dropTarget = null;
        }
        
        return true;
    }

    private function isEating() : Bool
    {
        if(foodTarget == null) return false;
        if(playerInterface.isMoving()) return true;

        // TODO check if food target is still valid

        var myPlayer = playerInterface.getPlayerInstance();

        var distance = playerInterface.CalculateDistanceToObject(foodTarget);

        if(distance > 1)
        {
            trace('AAI: goto food');
            playerInterface.Goto(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
            return true;
        }

        //trace('${foodTarget.tx} - ${myPlayer.tx()}, ${foodTarget.ty} - ${myPlayer.ty()}');

        if(myPlayer.heldObject.id == 0)
        {
            trace('AAI: pickup food from floor');

            // x,y is relativ to birth position, since this is the center of the universe for a player
            var done = playerInterface.use(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy); 

            return true;
        }

        var heldObjectIsEatable = myPlayer.heldObject.objectData.foodValue > 0;
        
        if(heldObjectIsEatable == false)
        {
            trace('AAI: drop held object to eat');

            dropHeldObject();

            return true;
        }

        var oldNumberOfUses = foodTarget.numberOfUses;

        playerInterface.self(); // eat

        trace('AAI: Eat: held: ${ myPlayer.heldObject.description} food: ${foodTarget.description} foodTarget.numberOfUses ${foodTarget.numberOfUses} == oldNumberOfUses $oldNumberOfUses || emptyFood: ${myPlayer.food_store_max - myPlayer.food_store} < 3)');

        /*if(foodTarget.numberOfUses == oldNumberOfUses || myPlayer.food_store_max - myPlayer.food_store < 4)
        {
            trace('AI: Eat: set foodTarget to null');
            foodTarget = null;
        }*/

        foodTarget = null;
        return true;
        
        /*
        var oldNumberOfUses = foodTarget.numberOfUses;

        //heldObjectIsEatable = myPlayer.heldObject.objectData.foodValue > 0;
        if(heldObjectIsEatable) playerInterface.self(); // eat

        trace('AI: Eat: held: ${ myPlayer.heldObject.description} food: ${foodTarget.description} foodTarget.numberOfUses ${foodTarget.numberOfUses} == oldNumberOfUses $oldNumberOfUses || emptyFood: ${myPlayer.food_store_max - myPlayer.food_store} < 3)');

        if(foodTarget.numberOfUses == oldNumberOfUses || myPlayer.food_store_max - myPlayer.food_store < 6)
        {
            trace('AI: Eat: set foodTarget to null');
            foodTarget = null;
        }

        dropHeldObject();

        time = 0.3;
                
        return true;*/
    }

    private function checkIsHungry() : Bool
    {
        var myPlayer = playerInterface.getPlayerInstance();

        myPlayer.food_store = 12; // TODO change
        isHungry = myPlayer.food_store < 10;

        if(isHungry && foodTarget == null) searchFoodAndEat();

        //playerInterface.say('F ${Math.round(playerInterface.getPlayerInstance().food_store)}');

        //trace('AAI: F ${Math.round(playerInterface.getPlayerInstance().food_store)} P:  ${myPlayer.x},${myPlayer.y} G: ${myPlayer.tx()},${myPlayer.ty()}');
        
        return isHungry;
    }

    private function isUsingItem() : Bool
    {
        if(useTarget == null) return false; 
        if(playerInterface.isMoving()) return true;

        var distance = playerInterface.CalculateDistanceToObject(useTarget);
        var myPlayer = playerInterface.getPlayerInstance();

        trace('AAI: Use:  distance: $distance ${useTarget.description} ${useTarget.tx} ${useTarget.ty}');
    
        if(distance > 1)
        {
            trace('AAI: goto useItem');
            playerInterface.Goto(useTarget.tx - myPlayer.gx, useTarget.ty - myPlayer.gy);
            return true;
        }

        // x,y is relativ to birth position, since this is the center of the universe for a player
        var done = playerInterface.use(useTarget.tx - myPlayer.gx, useTarget.ty - myPlayer.gy);

        if(done && itemToCraft != null)
        {
            itemToCraft.countTransitionsDone += 1; 
            var taregtObjectId = playerInterface.getWorld().getObjectId(useTarget.tx, useTarget.ty)[0];
            // if object to create is held by player or is on ground, then cound as done
            if(myPlayer.heldObject.parentId == itemToCraft.itemToCraft.parentId || taregtObjectId == itemToCraft.itemToCraft.parentId) itemToCraft.countDone += 1;

            trace('AAI: ItemToCraft: ${itemToCraft.itemToCraft.description} transtions done: ${itemToCraft.countTransitionsDone} done: ${itemToCraft.countDone}');
        }

        trace('AAI: ${useTarget.description} done: $done');

        useTarget = null;
        
        return true;
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
    public var itemToCraft:ObjectData;
    public var count:Int = 1; // how many items to craft
    public var countDone:Int = 0; // allready crafted
    public var countTransitionsDone:Int = 0; // transitions done while crafting


    public var transActor:ObjectHelper = null;
    public var transTarget:ObjectHelper = null;

    public var transitionsByObjectId:Map<Int, TransitionForObject>; 

    public function new() {}
}