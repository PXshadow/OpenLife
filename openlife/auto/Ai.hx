package openlife.auto;

import openlife.server.NamingHelper;
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

    public var myPlayer:PlayerInterface;

    var time:Float = 5;

    var foodTarget:ObjectHelper = null; 
    var dropTarget:ObjectHelper = null;
    var useTarget:ObjectHelper = null;

    var itemToCraftId = -1;
    var itemToCraft:IntemToCraft = new IntemToCraft();

    var isHungry = false;

    var playerToFollow:PlayerInterface;

    var children = new Array<PlayerInterface>();

    var notReachableObjects = new Map<Int, Float>();
    var objectsWithHostilePath = new Map<Int, Float>();

    public function new(player:PlayerInterface) 
    {
        this.myPlayer = player;
        //this.myPlayer = cast(playerInterface, GlobalPlayerInstance); // TODO support only client AI
    }

    public function newBorn()
    {
        foodTarget = null; 
        dropTarget = null;
        useTarget = null;

        itemToCraftId = -1;
        itemToCraft = new IntemToCraft();

        isHungry = false;

        playerToFollow = null;
        children = new Array<PlayerInterface>();
    }

    public function say(player:PlayerInterface, curse:Bool,text:String) 
    {
        //var myPlayer = myPlayer.getPlayerInstance();
        var world = myPlayer.getWorld();
        //trace('im a super evil bot!');

        //trace('ai3: ${myPlayer.p_id} player: ${player.p_id}');

        if (myPlayer.id == player.id) return;

        //trace('im a evil bot!');

        //trace('AI ${text}');

        if (text.contains("TRANS")) 
        {
            trace('AI look for transitions: ${text}');

            var objectIdToSearch = 273; // 273 = Cooked Carrot Pie // 250 = Hot Adobe Oven

            AiHelper.SearchTransitions(myPlayer, objectIdToSearch);
        }

        if (text.contains("HELLO")) 
        {
            //HELLO WORLD

            //trace('im a nice bot!');

            myPlayer.say("HELLO WORLD");
        }
        if (text.contains("JUMP")) 
        {
            myPlayer.say("JUMP");
            myPlayer.jump();
        }
        if (text.contains("MOVE"))
        {
            myPlayer.Goto(player.tx + 1 - myPlayer.gx, player.ty - myPlayer.gy);
            myPlayer.say("YES CAPTAIN");
        }
        if (text.contains("FOLLOW ME"))
        {
            playerToFollow = player;
            myPlayer.Goto(player.tx + 1 - myPlayer.gx, player.ty - myPlayer.gy);
            myPlayer.say("SURE CAPTAIN");
        }
        if (text.contains("STOP"))
        {
            playerToFollow = null;
            myPlayer.say("YES CAPTAIN");
        }
        /*if (text.contains("EAT!"))
        {
            AiHelper.SearchBestFood();
            searchFoodAndEat();
            myPlayer.say("YES CAPTAIN");
        }*/
        if (text.contains("MAKE!"))
        {
            var id = GlobalPlayerInstance.findObjectByCommand(text);

            if(id > 0)
            {
                itemToCraftId = id;
                itemToCraft.countDone = 0;
                itemToCraft.countTransitionsDone = 0;
                var obj = ObjectData.getObjectData(id);
                myPlayer.say("Making: " + obj.description);
            }
        }
    }

    public function searchFoodAndEat()
    {
        //var myPlayer = myPlayer.getPlayerInstance();
        foodTarget = AiHelper.SearchBestFood(myPlayer);
        //if(foodTarget != null) myPlayer.Goto(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
    }

    public function dropHeldObject() 
    {
        //var myPlayer = myPlayer.getPlayerInstance();
        
        if(myPlayer.heldObject.id == 0) return;

        var emptyTileObj = myPlayer.GetClosestObjectById(0); // empty
        dropTarget = emptyTileObj;

        //if(itemToCraft.transTarget.parentId == myPlayer.heldObject.parentId)

        trace('Drop ${emptyTileObj.tx} ${emptyTileObj.ty} $emptyTileObj');
        // x,y is relativ to birth position, since this is the center of the universe for a player
        //if(emptyTileObj != null) playerInterface.drop(emptyTileObj.tx - myPlayer.gx, emptyTileObj.ty - myPlayer.gy);
    }

    public function isChildAndHasMother() // must not be his original mother
    {
        var mother = myPlayer.getFollowPlayer();
        return (myPlayer.age < ServerSettings.MinAgeToEat &&  mother != null && mother.isDeleted() == false);
    }
    
    // do time stuff here is called from TimeHelper
    public function doTimeStuff(timePassedInSeconds:Float) 
    {
        var player = myPlayer.getPlayerInstance();

        time -= timePassedInSeconds;

        if(time > 0) return;
        time += ServerSettings.AiReactionTime; //0.5; // minimum AI reacting time
        
        cleanupBlockedObjects();

        if(myPlayer.getHeldByPlayer() != null)
        {
            //time += WorldMap.calculateRandomInt(); // TODO still jump and do stuff once in a while?
            return;
        } 

        var animal = AiHelper.GetCloseDeadlyAnimal(myPlayer);
        var deadlyPlayer = AiHelper.GetCloseDeadlyPlayer(myPlayer);

        if(escape(animal, deadlyPlayer)) return;
        checkIsHungryAndEat();
        if(isChildAndHasMother()){if(isMovingToPlayer()) return;}
        
        if(isDropingItem()) return;
        if(myPlayer.age < ServerSettings.MinAgeToEat && myPlayer.food_store < 2) return; // do nothing and wait for mother to feed
        if(isEating()) return;
        if(isFeedingChild()) return;
        if(isUsingItem()) return;
        if(isMovingToPlayer()) return;
        if(myPlayer.isMoving()) return;

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

    private function cleanupBlockedObjects()
    {
        for(key in notReachableObjects.keys())
        {
            var time = notReachableObjects[key];
            time -= ServerSettings.AiReactionTime;

            if(time <= 0)
            {
                notReachableObjects.remove(key);
                //trace('Unblock: remove $key t: $time');
                continue;    
            }

            //trace('Unblock: $key t: $time');

            notReachableObjects[key] = time;
        }

        for(key in objectsWithHostilePath.keys())
        {
            var time = objectsWithHostilePath[key];
            time -= ServerSettings.AiReactionTime;

            if(time <= 0)
            {
                objectsWithHostilePath.remove(key);
                //trace('Unblock: remove $key t: $time');
                continue;    
            }

            //trace('Unblock: $key t: $time');

            objectsWithHostilePath[key] = time;
        }
    }

    private function isFeedingChild()
    {
        if(myPlayer.isFertile() == false) return false; 
        if(myPlayer.food_store < 3) return false; 

        var heldPlayer = myPlayer.getHeldPlayer();

        if(heldPlayer != null)
        {
            if(heldPlayer.name == ServerSettings.StartingName && (heldPlayer.mother == myPlayer || heldPlayer.age > 1.5))
            {
                var newName = NamingHelper.GetRandomName(myPlayer.isFemale());
                trace('AAI: ${myPlayer.id} child newName: $newName');
                myPlayer.say('You are $newName');
            }

            if(heldPlayer.food_store > Math.min(5, heldPlayer.food_store_max - 1))
            {
                var done = myPlayer.dropPlayer();

                //trace('AAI: ${myPlayer.id} child drop ${heldPlayer.name} $done');

                return true;
            }
        }

        if(heldPlayer != null) return false;

        if(myPlayer.heldObject.id != 0)
        {
            dropHeldObject();
            return true;
        }

        var child = AiHelper.GetCloseHungryChild(myPlayer);
        if(child == null) return false;

        var childFollowPlayer = child.getFollowPlayer();
        if(childFollowPlayer.isFertile() == false)
        {
            playerToFollow = myPlayer;
        }

        var distance = myPlayer.CalculateDistanceToPlayer(child);
        var childX = child.tx - myPlayer.gx;
        var childY = child.ty - myPlayer.gy;

        if(distance > 1)
        {
            trace('AAI: ${myPlayer.id} goto child');
            myPlayer.Goto(childX, childY);
            return true;
        }

        myPlayer.say('Pickup ${child.name}');
        var done = myPlayer.doBaby(childX, childY, child.id);

        //trace('AAI: child ${child.name} pickup $done');

        return true;
    }

    private function escape(animal:ObjectHelper, deadlyPlayer:GlobalPlayerInstance)
    {
        if(animal == null && deadlyPlayer == null) return false;

        var player = myPlayer.getPlayerInstance();

        var distAnimal = animal == null ? 999999 : AiHelper.CalculateDistanceToObject(myPlayer, animal);
        var distPlayer = deadlyPlayer == null ? 999999 : AiHelper.CalculateDistanceToPlayer(myPlayer, deadlyPlayer);
        var escapePlayer = distAnimal > distPlayer;
        var description = escapePlayer ? deadlyPlayer.name : animal.description;
        var escapeTx = escapePlayer ? deadlyPlayer.tx : animal.tx;
        var escapeTy = escapePlayer ? deadlyPlayer.ty : animal.ty;

        myPlayer.say('Escape ${description}!');
        trace('AAI: ${myPlayer.id} escape!');
        
        var tx = escapeTx > player.tx ?  player.tx - 3 : player.tx + 3;
        var ty = escapeTy > player.ty ?  player.ty - 3 : player.ty + 3;

        myPlayer.Goto(tx - player.gx, ty - player.gy);

        if(useTarget != null || foodTarget != null)
        {
            addObjectWithHostilePath(useTarget);
            addObjectWithHostilePath(foodTarget);
            useTarget = null;
            foodTarget = null;
            itemToCraft.transActor = null;
            itemToCraft.transTarget = null;
        }

        return true;
    }

    // TODO consider held object / backpack / contained objects
    // TODO consider if object is reachable
    // TODO store transitions for crafting to have faster lookup
    // TODO consider too look for a natural spawned object with the fewest steps on the list
    private function craftItem(objId:Int, count:Int = 1, ignoreHighTech:Bool = false) : Bool
    {                
        var player = myPlayer.getPlayerInstance();

        if(itemToCraft.transActor != null && player.heldObject.parentId == itemToCraft.transActor.parentId)
        {
            itemToCraft.transActor = null; // actor is allready in the hand
            var target = AiHelper.GetClosestObject(myPlayer, itemToCraft.transTarget.objectData);
            useTarget = target != null ? target : itemToCraft.transTarget; // since other search radius might be bigger

            return true;
        }    

        if(itemToCraft.itemToCraft.parentId != objId)
        {
            itemToCraft.itemToCraft = ObjectData.getObjectData(objId);
            itemToCraft.countDone = 0;
            itemToCraft.countTransitionsDone = 0;
            itemToCraft.transitionsByObjectId = myPlayer.SearchTransitions(objId, ignoreHighTech); 
            //itemToCraft.notReachableObjects = new Map<Int,Int>();
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
            myPlayer.say('Goto actor ' + itemToCraft.transTarget.description );

            useTarget = itemToCraft.transTarget; 
            itemToCraft.transActor = null; // actor is allready in the hand

        }
        else
        {
            trace('AI: craft goto actor: ' + itemToCraft.transActor.description);
            myPlayer.say('Goto target ' + itemToCraft.transActor.description );
            
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

        var world = myPlayer.getWorld();
        var player = myPlayer.getPlayerInstance();
        var baseX = player.tx;
        var baseY = player.ty;
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
                    if(this.isObjectNotReachable(tx,ty)) continue;

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
                    var objDistance = myPlayer.CalculateDistanceToObject(obj);
                    
                    if(trans.closestObject == null || trans.closestObjectDistance > objDistance)
                    {
                        if(AiHelper.IsDangerous(myPlayer, obj)) continue;

                        trans.secondObject = trans.closestObject;
                        trans.secondObjectDistance = trans.closestObjectDistance;

                        trans.closestObject = obj;
                        trans.closestObjectDistance = objDistance;
                        
                        continue;
                    }
                    
                    if(trans.secondObject == null || trans.secondObjectDistance > objDistance)
                    {
                        if(AiHelper.IsDangerous(myPlayer, obj)) continue;
                        
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
                    var traceTrans = AiHelper.ShouldDebug(targetTrans.bestTransition);

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
                        trace('Target4: AI: using two ' + targetTrans.bestTransition.actorID);

                        if(tmpObject.secondObject == null) continue;

                        trace('Target4: AI: using two 2');

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

    private function isMovingToPlayer(maxDistance = 25) : Bool
    {
        if(playerToFollow == null)
        {
            if(isChildAndHasMother())
            {   
                playerToFollow = myPlayer.getFollowPlayer();
            }
            else
            {
                // get close human player
                playerToFollow = myPlayer.getWorld().getClosestPlayer(20, true);

                if(playerToFollow == null) return false;
            
                //trace('AAI: ${myPlayer.id} follow player ${playerToFollow.p_id}');
            }
        }

        if(myPlayer.CalculateDistanceToPlayer(playerToFollow) > maxDistance)
        {
            trace('AAI: ${myPlayer.id} goto player');

            myPlayer.Goto(playerToFollow.tx + 1 - myPlayer.gx, playerToFollow.ty - myPlayer.gy);
            myPlayer.say('${playerToFollow.name}');
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
        if(myPlayer.isMoving()) return true;

        var distance = myPlayer.CalculateDistanceToObject(dropTarget);
        //var myPlayer = myPlayer.getPlayerInstance();

        if(distance > 1)
        {
            trace('AAI: ${myPlayer.id} goto drop');
            myPlayer.Goto(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);
        }
        else
        {
            trace('AAI: ${myPlayer.id} drop');

            myPlayer.drop(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);
            
            dropTarget = null;
        }
        
        return true;
    }

    private function isEating() : Bool
    {
        if(foodTarget == null) return false;
        if(myPlayer.isMoving()) return true;


        // TODO check if food target is still valid

        // var myPlayer = myPlayer.getPlayerInstance();

        var distance = myPlayer.CalculateDistanceToObject(foodTarget);

        if(distance > 1)
        {
            trace('AAI: ${myPlayer.id} goto food');
            myPlayer.Goto(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
            return true;
        }

        var heldPlayer = myPlayer.getHeldPlayer();
        if(heldPlayer != null)
        {
            var done = myPlayer.dropPlayer();

            trace('AAI: ${myPlayer.id} child drop for eating ${heldPlayer.name} $done');

            return true;
        }

        //trace('${foodTarget.tx} - ${myPlayer.tx()}, ${foodTarget.ty} - ${myPlayer.ty()}');

        if(myPlayer.heldObject.id == 0)
        {
            trace('AAI: ${myPlayer.id} pickup food from floor');

            // x,y is relativ to birth position, since this is the center of the universe for a player
            var done = myPlayer.use(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy); 

            return true;
        }

        var heldObjectIsEatable = myPlayer.heldObject.objectData.foodValue > 0;
        
        if(heldObjectIsEatable == false)
        {
            trace('AAI: ${myPlayer.id} drop held object to eat');

            dropHeldObject();

            return true;
        }

        var oldNumberOfUses = foodTarget.numberOfUses;

        myPlayer.self(); // eat

        trace('AAI: ${myPlayer.id} Eat: held: ${ myPlayer.heldObject.description} food: ${foodTarget.description} foodTarget.numberOfUses ${foodTarget.numberOfUses} == oldNumberOfUses $oldNumberOfUses || emptyFood: ${myPlayer.food_store_max - myPlayer.food_store} < 3)');

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

    private function checkIsHungryAndEat() : Bool
    {
        var player = myPlayer.getPlayerInstance();

        if(isHungry)
        {
            isHungry = player.food_store < player.food_store_max * 0.75;
        }
        else
        {
            isHungry = player.food_store < Math.min(3, player.food_store_max * 0.5);
        }

        if(isHungry && foodTarget == null) searchFoodAndEat();

        myPlayer.say('F ${Math.round(myPlayer.getPlayerInstance().food_store)}');

        //trace('AAI: ${myPlayer.id} F ${Math.round(playerInterface.getPlayerInstance().food_store)} P:  ${myPlayer.x},${myPlayer.y} G: ${myPlayer.tx()},${myPlayer.ty()}');
        
        return isHungry;
    }

    private function isUsingItem() : Bool
    {
        if(useTarget == null) return false; 
        if(myPlayer.isMoving()) return true;

        var distance = myPlayer.CalculateDistanceToObject(useTarget);
        //var myPlayer = myPlayer.getPlayerInstance();

        trace('AAI: ${myPlayer.id} Use:  distance: $distance ${useTarget.description} ${useTarget.tx} ${useTarget.ty}');
    
        if(distance > 1)
        {
            trace('AAI: ${myPlayer.id} goto useItem');
            var done = myPlayer.Goto(useTarget.tx - myPlayer.gx, useTarget.ty - myPlayer.gy);

            // TODO use item not reachable or bug in pathing?
            if(done == false)
            {
                trace('AI: GOTO failed! Ignore ${useTarget.tx} ${useTarget.ty} '); 
                this.addNotReachableObject(useTarget);
                useTarget = null;
                itemToCraft.transActor = null;
                itemToCraft.transTarget = null;
            }
            
            return true;
        }

        var heldPlayer = myPlayer.getHeldPlayer();
        if(heldPlayer != null)
        {
            var done = myPlayer.dropPlayer();

            trace('AAI: ${myPlayer.id} child drop for using ${heldPlayer.name} $done');

            return true;
        }

        // x,y is relativ to birth position, since this is the center of the universe for a player
        var done = myPlayer.use(useTarget.tx - myPlayer.gx, useTarget.ty - myPlayer.gy);

        if(done)
        {
            itemToCraft.countTransitionsDone += 1; 
            var taregtObjectId = myPlayer.getWorld().getObjectId(useTarget.tx, useTarget.ty)[0];
            // if object to create is held by player or is on ground, then cound as done
            if(myPlayer.heldObject.parentId == itemToCraft.itemToCraft.parentId || taregtObjectId == itemToCraft.itemToCraft.parentId) itemToCraft.countDone += 1;

            trace('AI: done: ${useTarget.description} ItemToCraft: ${itemToCraft.itemToCraft.description} transtions done: ${itemToCraft.countTransitionsDone} done: ${itemToCraft.countDone} FROM: ${itemToCraft.count}');
        }
        else
        {
            trace('AI: Use failed! Ignore ${useTarget.tx} ${useTarget.ty} '); 

            // TODO check why use is failed... for now add to ignore list
            this.addNotReachableObject(useTarget);
            useTarget = null;
            itemToCraft.transActor = null;
            itemToCraft.transTarget = null;
        }

        useTarget = null;
        
        return true;
    }

    public function addObjectWithHostilePath(obj:ObjectHelper)
    {
        if(obj == null) return;
        var index = WorldMap.world.index(obj.tx, obj.ty);
        objectsWithHostilePath[index] = 30; // block for 30 sec
    }

    public function isObjectWithHostilePath(tx:Int, ty:Int) : Bool
    {
        var index = WorldMap.world.index(tx, ty);
        var notReachable = objectsWithHostilePath.exists(index);

        //if(notReachable) trace('isObjectNotReachable: $notReachable $tx,$ty');

        return notReachable;
    }

    public function addNotReachableObject(obj:ObjectHelper)
    {
        var index = WorldMap.world.index(obj.tx, obj.ty);
        notReachableObjects[index] = 120; // block for 120 sec
    }

    public function isObjectNotReachable(tx:Int, ty:Int) : Bool
    {
        var index = WorldMap.world.index(tx, ty);
        var notReachable = notReachableObjects.exists(index);

        if(notReachable) trace('isObjectNotReachable: $notReachable $tx,$ty');

        return notReachable;
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

    public function newChild(child:PlayerInterface)
    {
        this.children.push(child);    
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
}