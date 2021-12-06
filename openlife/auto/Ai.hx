package openlife.auto;

import openlife.server.WorldMap;
import openlife.server.GlobalPlayerInstance;
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

    var itemToCraftId = -1;
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
            var id = GlobalPlayerInstance.findObjectByCommand(text);

            if(id > 0)
            {
                itemToCraftId = id;
                itemToCraft.countDone = 0;
                itemToCraft.countTransitionsDone = 0;
                var obj = ObjectData.getObjectData(id);
                playerInterface.say("Making: " + obj.description);
            }
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

        //if(itemToCraft.transTarget.parentId == myPlayer.heldObject.parentId)

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
        
        if(playerInterface.isMoving()) return;

        checkIsHungry();
     
        if(isDropingItem()) return;
        if(isEating()) return;
        if(isUsingItem()) return;
        if(isMovingToPlayer()) return;

        //if(playerToFollow == null) return; // Do stuff only if close to player TODO remove if testing AI without player
        if(itemToCraftId == itemToCraft.itemToCraft.parentId && itemToCraft.countDone >= itemToCraft.count) return;
        if(itemToCraftId <= 0) itemToCraftId = itemToCraft.itemToCraft.parentId;

        //trace('AI: itemToCraftId: $itemToCraftId ${itemToCraft.itemToCraft.parentId}' );

        if(itemToCraftId > 0)
        {
            craftItem(itemToCraftId);
        }
        else
        {
            //craftItem(58); // Thread
            //craftItem(74, 1, true); //Fire Bow Drill
            craftItem(78, 1, true); // Smoldering Tinder 
            //craftItem(808); // wild onion
            //craftItem(292, 1, true); // 292 basket
            //craftItem(224); // Harvested Wheat
            //craftItem(124); // Reed Bundle
            //craftItem(225); //Wheat Sheaf
        }
        //craftItem(34,1); // 34 sharpstone
        //craftItem(224); // Harvested Wheat
        //craftItem(58); // Thread
    }

    // TODO consider held object / backpack / contained objects
    // TODO consider if object is reachable
    // TODO store transitions for crafting to have faster lookup
    // TODO consider too look for a natural spawned object with the fewest steps on the list
    private function craftItem(objId:Int, count:Int = 1, ignoreHighTech:Bool = false) : Bool
    {                
        var player = playerInterface.getPlayerInstance();

        if(itemToCraft.transActor != null && player.heldObject.parentId == itemToCraft.transActor.parentId)
        {
            itemToCraft.transActor = null; // actor is allready in the hand
            var target = AiHelper.GetClosestObject(playerInterface, itemToCraft.transTarget.objectData);
            useTarget = target != null ? target : itemToCraft.transTarget; // since other search radius might be bigger

            return true;
        }    

        if(itemToCraft.itemToCraft.parentId != objId)
        {
            itemToCraft.itemToCraft = ObjectData.getObjectData(objId);
            itemToCraft.countDone = 0;
            itemToCraft.countTransitionsDone = 0;
            itemToCraft.transitionsByObjectId = playerInterface.SearchTransitions(objId, ignoreHighTech); 
            itemToCraft.notReachableObjects = new Map<Int,Int>();
        }
        
        searchBestObjectForCrafting(itemToCraft);

        if(itemToCraft.transActor == null)
        {
            trace('AI: craft: ${itemToCraft.itemToCraft.description} did not find any item in search radius for crafting!');
            return false;
        }

        if(player.heldObject.parentId == itemToCraft.transActor.parentId)
        {
            trace('AI: craft Actor is held already' + itemToCraft.transActor.description);
            playerInterface.say('Goto actor ' + itemToCraft.transTarget.description );

            useTarget = itemToCraft.transTarget; 
            itemToCraft.transActor = null; // actor is allready in the hand

        }
        else
        {
            trace('AI: craft goto actor: ' + itemToCraft.transActor.description);
            playerInterface.say('Goto target ' + itemToCraft.transActor.description );
            
            useTarget = itemToCraft.transActor;

            if(player.heldObject.id != 0)
            {
                    dropHeldObject();
                    return true;
            }
        }
        
        
        return true;
    }

    private function searchBestObjectForCrafting(itemToCraft:IntemToCraft) : IntemToCraft
    {
        //var itemToCraft = new IntemToCraft();
        itemToCraft.transActor = null;
        itemToCraft.transTarget = null;

        var objToCraftId = itemToCraft.itemToCraft.parentId;
        var transitionsByObjectId = itemToCraft.transitionsByObjectId;

        var world = playerInterface.getWorld();
        var player = playerInterface.getPlayerInstance();
        var baseX = player.tx();
        var baseY = player.ty();
        var bestDistance = 0.0;
        var bestSteps = 0;
        var bestTrans = null; 
        var radius = 0;


        // TODO dont cut down tables <3371> if not needed  

        while (radius < ServerSettings.AiMaxSearchRadius)
        {
            radius += ServerSettings.AiMaxSearchIncrement;

            trace('AI search radius: $radius');

            // reset objects so that it can be filled again
            itemToCraft.clearTransitionsByObjectId();   

            // check if held object can be used to craft item
            var trans = transitionsByObjectId[player.heldObject.parentId];

            if(trans != null)
            {
                trans.closestObject = player.heldObject;
                trans.closestObjectDistance = 0;
                trans.closestObjectPlayerIndex = 0; // held in hand
            }

            // go through all close by objects and map them to the best transition
            for(ty in baseY - radius...baseY + radius)
            {
                for(tx in baseX - radius...baseX + radius)
                {
                    if(itemToCraft.isObjectNotReachable(tx,ty)) continue;

                    var objData = world.getObjectDataAtPosition(tx, ty);

                    if(objData.id == 0) continue;            
                    if(objData.dummyParent != null) objData = objData.dummyParent; // use parent objectdata

                    // Ignore container with stuff inside 
                    // TODO consider contained objects
                    if(objData.numSlots > 0)
                    {
                        var container = world.getObjectHelper(tx,ty);
                        if(container.containedObjects.length > 0)
                        {
                            //trace('AI: search IGNORE container: ${objData.description}');
                            continue;
                        }
                    }

                    // check if object can be used to craft item
                    var trans = transitionsByObjectId[objData.id];
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
            for(trans in transitionsByObjectId)
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
                    var isUsingTwo = targetTrans.bestTransition.actorID == targetTrans.bestTransition.targetID;
                    var traceTrans = targetTrans.bestTransition.newActorID == 57;

                    if(traceTrans) trace('Target1: ' + targetTrans.bestTransition.getDesciption(true));

                    // check if there are allready two of this // TODO if only one is needed skip second
                    var tmpWanted = transitionsByObjectId[targetTrans.wantedObjId];
                    if(tmpWanted != null && targetTrans.wantedObjId != objToCraftId && tmpWanted.closestObjectDistance > -1 && (isUsingTwo == false || tmpWanted.secondObjectDistance > -1)) continue;

                    if(traceTrans) trace('Target2: ' + targetTrans.bestTransition.getDesciption(true));

                    if(actorID != 0 && actorID != trans.closestObject.parentId) continue;

                    var tmpObject = transitionsByObjectId[targetTrans.bestTransition.targetID];

                    if(traceTrans) trace('Target3: ' + targetTrans.bestTransition.getDesciption(true));

                    if(tmpObject == null) continue;

                    if(tmpObject.closestObject == null) continue;

                    if(traceTrans) trace('Target4: ' + targetTrans.bestTransition.getDesciption(true));

                    var tmpDistance = tmpObject.closestObjectDistance;
                    var tmpTargetObject = tmpObject.closestObject;

                    if(isUsingTwo) // like using two milkweed
                    {
                        trace('AI: using two ' + targetTrans.bestTransition.actorID);

                        if(tmpObject.secondObject == null) continue;

                        trace('AI: using two 2');

                        tmpDistance = tmpObject.secondObjectDistance;
                        tmpTargetObject = tmpObject.secondObject;
                    }

                    if(traceTrans) trace('Target5: ' + targetTrans.bestTransition.getDesciption(true));

                    var steps = targetTrans.steps;

                    if(bestTargetTrans == null || bestTargetSteps > steps || (bestTargetSteps == steps && tmpDistance < bestTargetDistance))
                    {
                        if(traceTrans) trace('Target6: bestTarget ' + targetTrans.bestTransition.getDesciption(true));
                        bestTargetTrans = targetTrans;
                        bestTargetDistance = tmpDistance;
                        bestTargetSteps = steps;
                        bestTargetObject = tmpTargetObject;
                    }
                }

                if(bestTargetObject == null) continue;

                //var targetObject = bestTargetObject.closestObject;

                if(bestTargetObject == null) continue;

                //var steps = trans.steps;
                var obj = trans.closestObject;
                var distance = trans.closestObjectDistance + bestTargetDistance; // actor plus target distance

                var traceTrans = bestTargetTrans.bestTransition.newActorID == 57;
                if(traceTrans) trace('Target7: ' + bestTargetTrans.bestTransition.getDesciption(true));

                if(itemToCraft.transActor == null || bestSteps > bestTargetSteps  || (bestTargetSteps == bestSteps && distance < bestDistance))
                {
                    if(traceTrans) trace('Target8: ' + bestTargetTrans.bestTransition.getDesciption(true));

                    itemToCraft.transActor = obj;
                    itemToCraft.transTarget = bestTargetObject;                    
                    bestSteps = bestTargetSteps;
                    bestDistance = distance;
                    bestTrans = bestTargetTrans;

                    //if(bestTargetObject.parentId == 50) trace('TEST6 actor: ${obj.description} target: ${bestTargetObject.description} ');
                }
            }

            if(itemToCraft.transActor != null) trace('ai: craft: steps: $bestSteps Distance: $bestDistance bestActor: ${itemToCraft.transActor.description} / target: ${itemToCraft.transTarget.id} ${itemToCraft.transTarget.description} ' + bestTrans.getDesciption());
            if(itemToCraft.transActor != null) return itemToCraft;

        }

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
            var done = playerInterface.Goto(useTarget.tx - myPlayer.gx, useTarget.ty - myPlayer.gy);

            // TODO use item not reachable or bug in pathing?
            if(done == false)
            {
                trace('AI: GOTO failed! Ignore ${useTarget.tx} ${useTarget.ty} '); 
                itemToCraft.addNotReachableObject(useTarget);
                useTarget = null;
                itemToCraft.transActor = null;
                itemToCraft.transTarget = null;
            }
            
            return true;
        }

        // x,y is relativ to birth position, since this is the center of the universe for a player
        var done = playerInterface.use(useTarget.tx - myPlayer.gx, useTarget.ty - myPlayer.gy);

        if(done)
        {
            itemToCraft.countTransitionsDone += 1; 
            var taregtObjectId = playerInterface.getWorld().getObjectId(useTarget.tx, useTarget.ty)[0];
            // if object to create is held by player or is on ground, then cound as done
            if(myPlayer.heldObject.parentId == itemToCraft.itemToCraft.parentId || taregtObjectId == itemToCraft.itemToCraft.parentId) itemToCraft.countDone += 1;

            trace('AI: done: ${useTarget.description} ItemToCraft: ${itemToCraft.itemToCraft.description} transtions done: ${itemToCraft.countTransitionsDone} done: ${itemToCraft.countDone} FROM: ${itemToCraft.count}');
        }
        else
        {
            trace('AI: Use failed! Ignore ${useTarget.tx} ${useTarget.ty} '); 

            // TODO check why use is failed... for now add to ignore list
            itemToCraft.addNotReachableObject(useTarget);
            useTarget = null;
            itemToCraft.transActor = null;
            itemToCraft.transTarget = null;
        }

        

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

    public var notReachableObjects = new Map<Int, Int>();

    public function new()
    {
        itemToCraft = ObjectData.getObjectData(0);
    }

    public function clearTransitionsByObjectId()
    {
        // reset objects so that it can be filled again
        for(trans in transitionsByObjectId)
        {
            trans.closestObject = null;
            trans.closestObjectDistance = -1;
            trans.closestObjectPlayerIndex = -1;

            trans.secondObject = null;
            trans.closestObjectDistance = -1;
        }
    }

    public function addNotReachableObject(obj:ObjectHelper)
    {
        var index = WorldMap.world.index(obj.tx, obj.ty);
        notReachableObjects[index] = obj.parentId;
    }

    public function isObjectNotReachable(tx:Int, ty:Int) : Bool
    {
        var index = WorldMap.world.index(tx, ty);
        var notReachable = notReachableObjects.exists(index);

        if(notReachable) trace('isObjectNotReachable: $notReachable $tx,$ty');

        return notReachable;
    }
}