package openlife.auto;

import openlife.data.object.ObjectHelper;
import openlife.data.object.ObjectData;
import openlife.data.transition.TransitionData;
import openlife.data.object.player.PlayerInstance;
import openlife.data.map.MapData;
import openlife.data.Pos;
using StringTools;
import openlife.auto.Pathfinder.Coordinate;
import haxe.ds.Vector;

class Ai
{
    var playerInterface:PlayerInterface;

    var goal:Pos;
    var dest:Pos;
    var init:Pos;

    var done = false;

    var time:Float = 5;

    //var berryHunter:Bool = false;
    var isHungry = false;
    var foodTarget:ObjectHelper = null; 
    var dropTarget:ObjectHelper = null;
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

            searchTransitions(objectIdToSearch);
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
            goto(player.tx() + 1 - myPlayer.gx, player.ty() - myPlayer.gy);
            playerInterface.say("YES CAPTAIN");
        }
        if (text.contains("FOLLOW ME"))
        {
            playerToFollow = player;
            goto(player.tx() + 1 - myPlayer.gx, player.ty() - myPlayer.gy);
            playerInterface.say("SURE CAPTAIN");
        }
        if (text.contains("STOP"))
        {
            playerToFollow = null;
            playerInterface.say("YES CAPTAIN");
        }
        if (text.contains("EAT!"))
        {
            searchFoodAndEat();
            playerInterface.say("YES CAPTAIN");
        }
    }

    public function searchFoodAndEat()
    {
        var myPlayer = playerInterface.getPlayerInstance();
        foodTarget = searchBestFood();
        if(foodTarget != null) goto(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
    }

    public function doTimeStuff(timePassedInSeconds:Float) 
    {
        // @PX do time stuff here is called from TimeHelper

        var myPlayer = playerInterface.getPlayerInstance();

        if(dropTarget != null && playerInterface.isMoving() == false)
        {
            var distance = calculateDistanceToObject(myPlayer, dropTarget);

            if(distance > 1)
            {
                goto(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);
            }
            else
            {
                playerInterface.drop(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);
                dropTarget = null;
            }
            
            return;
        }

        if(foodTarget == null && playerInterface.isMoving() == false)
        {
            if(playerToFollow != null && calculateDistanceToPlayer(myPlayer, playerToFollow) > 2)
            {
                goto(playerToFollow.tx() + 1 - myPlayer.gx, playerToFollow.ty() - myPlayer.gy);
            }
        } 

        if(foodTarget != null && playerInterface.isMoving() == false)
        {
            var distance = calculateDistanceToObject(myPlayer, foodTarget);

            if(distance > 1)
            {
                goto(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
            }
            else
            {
                //trace('${foodTarget.tx} - ${myPlayer.tx()}, ${foodTarget.ty} - ${myPlayer.ty()}');

                // TODO eat food if held food is eatable
                
                
                var oldNumberOfUses = foodTarget.numberOfUses;

                // x,y is relativ to birth position, since this is the center of the universe for a player
                var done = playerInterface.use(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
                
                playerInterface.self();

                trace('Eat: ${foodTarget.description} foodTarget.numberOfUses ${foodTarget.numberOfUses} == oldNumberOfUses $oldNumberOfUses || emptyFood: ${myPlayer.food_store_max - myPlayer.food_store} < 3)');

                if(foodTarget.numberOfUses == oldNumberOfUses || myPlayer.food_store_max - myPlayer.food_store < 3)
                {
                    foodTarget = null;
                }

                // drop held object
                if(myPlayer.heldObject.id > 0)
                {
                    var emptyObject = ObjectData.getObjectData(0);
                    var emptyTileObj = getClosestObject(myPlayer, emptyObject);
                    dropTarget = emptyTileObj;

                    trace('Eat: Drop ${emptyTileObj.tx} ${emptyTileObj.ty} $emptyTileObj');
                    // x,y is relativ to birth position, since this is the center of the universe for a player
                    //if(emptyTileObj != null) playerInterface.drop(emptyTileObj.tx - myPlayer.gx, emptyTileObj.ty - myPlayer.gy);
                }
            }
        } 

        time -= timePassedInSeconds;

        if(time < 0)
        {
            time = 3;

            isHungry = myPlayer.food_store < 10;

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

    // is called once a movement is finished (client side it must be called manually after a PlayerUpdate)
    public function finishedMovement()
    {
        /*if(playerToFollow != null && foodTarget == null)
        {
            var myPlayer = playerInterface.getPlayerInstance();

            var distance = calculateDistanceToPlayer(myPlayer, playerToFollow);
            if(distance > 2) goto(playerToFollow.tx() + 1 - myPlayer.gx, playerToFollow.ty() - myPlayer.gy);
            //playerInterface.say("I FOLLOW");
        }*/
    }

    

    private function searchBestFood() : ObjectHelper
    {
        var player = playerInterface.getPlayerInstance();
        var baseX = player.tx();
        var baseY = player.ty();
        var world = playerInterface.getWorld();
        var bestFood = null;
        var bestDistance = 0.0;
        

        // TODO consider current food vlaue cravings

        for(ty in baseY - RAD...baseY + RAD)
        {
            for(tx in baseX - RAD...baseX + RAD)
            {
                var obj = world.getObjectHelper(tx, ty, true);
                if(obj == null) continue;

                var objData = obj.objectData;
                if(objData.dummyParent !=null) objData = objData.dummyParent; // use parent objectdata

                if(objData.foodValue > 0 || objData.foodFromTarget != null)                    
                {
                    var distance = calculateDistance(baseX, baseY, obj.tx, obj.ty);
                    //trace('search food: best $bestDistance dist $distance ${obj.description}');

                    if(bestFood == null || distance < bestDistance)
                    {
                        bestFood = obj;
                        bestDistance = distance;
                    }
                }
            }
        }

        trace('bestfood: $bestDistance ${bestFood.description}');

        return bestFood;
    }

    public function getClosestObject(player:PlayerInstance, objData:ObjectData) : ObjectHelper
    {
        var baseX = player.tx();
        var baseY = player.ty();
        var world = playerInterface.getWorld();
        var closestObject = null;
        var bestDistance = 0.0;

        for(ty in baseY - RAD...baseY + RAD)
        {
            for(tx in baseX - RAD...baseX + RAD)
            {
                var obj = world.getObjectHelper(tx, ty, true);
                if(obj == null) continue;

                if(obj.parentId == objData.parentId)                    
                {
                    var distance = calculateDistance(baseX, baseY, obj.tx, obj.ty);

                    if(closestObject == null || distance < bestDistance)
                    {
                        closestObject = obj;
                        bestDistance = distance;
                    }
                }
            }
        }

        trace('bestdistance: $bestDistance ${closestObject.description}');

        return closestObject;
    }

    private function calculateDistanceToPlayer(player:PlayerInstance, playerTo:PlayerInstance) : Float
    {
        return calculateDistance(player.tx(), player.ty(), playerTo.tx(), playerTo.ty());
    }

    private function calculateDistanceToObject(player:PlayerInstance, obj:ObjectHelper) : Float
    {
        return calculateDistance(player.tx(), player.ty(), obj.tx, obj.ty);
    }

    private function calculateDistance(baseX:Int, baseY:Int, toX:Int, toY:Int) : Float
    {
        return (toX - baseX) * (toX - baseX) + (toY - baseY) * (toY - baseY);
    }

    final RAD:Int = MapData.RAD;

    // TODO goto uses global coordinates
    public function goto(x:Int,y:Int):Bool
    {
        var player = playerInterface.getPlayerInstance();
        //if (player.x == x && player.y == y || moving) return false;
        //set pos
        var px = x - player.x;
        var py = y - player.y;

        if(px == 0 && py == 0) return false; // no need to move

        if (px > RAD - 1) px = RAD - 1;
        if (py > RAD - 1) py = RAD - 1;
        if (px < -RAD) px = -RAD;
        if (py < -RAD) py = -RAD;
        //cords
        var start = new Coordinate(RAD,RAD);

        //trace('Goto: $px $py');

        var map = new MapCollision(createCollisionChunk());
        //pathing
        var path = new Pathfinder(cast map);
        var paths:Array<Coordinate> = null;
        //move the end cords
        var tweakX:Int = 0;
        var tweakY:Int = 0;

        for (i in 0...3)
        {
            switch(i)
            {
                case 1:
                tweakX = x - player.x < 0 ? 1 : -1;
                case 2:
                tweakX = 0;
                tweakY = y - player.y < 0 ? 1 : -1;
            }

            var end = new Coordinate(px + RAD + tweakX, py + RAD + tweakY);

            trace('goto: end $end');

            paths = path.createPath(start,end,MANHATTAN,true);
            if (paths != null) break;
        }

        if (paths == null) 
        {
            //if (onError != null) onError("can not generate path");
            trace("CAN NOT GENERATE PATH");
            return false;
        }

        /*for(path in paths)
        {
            trace(path);
        }*/

        var data:Array<Pos> = [];
        paths.shift();
        //var mx:Array<Int> = [];
        //var my:Array<Int> = [];
        var tx:Int = start.x;
        var ty:Int = start.y;

        for (path in paths)
        {
            data.push(new Pos(path.x - tx,path.y - ty));
        }

        goal = new Pos(x,y);

        if (px == goal.x - player.x && py == goal.y - player.y)
        {
            trace("shift goal!");
            //shift goal as well
            goal.x += tweakX;
            goal.y += tweakY;
        }

        dest = new Pos(px + player.x,py + player.y);
        init = new Pos(player.x,player.y);
        //movePlayer(data);
        playerInterface.move(player.x,player.y,player.done_moving_seqNum++,data);

        //isMoving = true;

        return true;
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

    private function searchTransitions(objectIdToSearch:Int) : Map<Int, TransitionForObject>
    {    
        // TODO might be good to also allow multiple transitions for one object
        var world = this.playerInterface.getWorld();
        var transitionsByObject = new Map<Int, TransitionData>();
        var transitionsForObject = new Map<Int, TransitionForObject>();
        
        var transitionsToProcess = new Array<Array<TransitionData>>();
        var steps = new Array<Int>();
        var wantedObjIds = new Array<Int>();
        var stepsCount = 1;

        transitionsToProcess.push(world.getTransitionByNewTarget(objectIdToSearch)); 
        transitionsToProcess.push(world.getTransitionByNewActor(objectIdToSearch)); 

        steps.push(stepsCount);
        steps.push(stepsCount);

        wantedObjIds.push(objectIdToSearch);
        wantedObjIds.push(objectIdToSearch);

        var count = 1;  
        
        var startTime = Sys.time();

        while (transitionsToProcess.length > 0)
        {
            var transitions = transitionsToProcess.shift();
            stepsCount = steps.shift();
            var wantedObjId = wantedObjIds.shift();

            for(trans in transitions)
            {
                if(transitionsByObject.exists(trans.actorID) || transitionsByObject.exists(trans.targetID)) continue;

                //if(count < 10000) trans.traceTransition('AI stepsCount: $stepsCount count: $count:', true);

                if(trans.actorID > 0 && trans.actorID != trans.newActorID && transitionsByObject.exists(trans.actorID) == false)
                {
                    transitionsToProcess.push(world.getTransitionByNewTarget(trans.actorID)); 
                    transitionsToProcess.push(world.getTransitionByNewActor(trans.actorID)); 

                    steps.push(stepsCount + 1);
                    steps.push(stepsCount + 1);

                    wantedObjIds.push(trans.actorID);
                    wantedObjIds.push(trans.actorID);
                }

                if(trans.targetID > 0 && trans.targetID != trans.newTargetID && transitionsByObject.exists(trans.targetID) == false)
                {
                    transitionsToProcess.push(world.getTransitionByNewTarget(trans.targetID)); 
                    transitionsToProcess.push(world.getTransitionByNewActor(trans.targetID)); 

                    steps.push(stepsCount + 1);
                    steps.push(stepsCount + 1);

                    wantedObjIds.push(trans.targetID);
                    wantedObjIds.push(trans.targetID);
                }

                if(trans.actorID > 0) transitionsByObject[trans.actorID] = trans;
                if(trans.targetID > 0) transitionsByObject[trans.targetID] = trans;

                if(trans.actorID > 0) addTransition(transitionsForObject, trans, trans.actorID, wantedObjId, stepsCount);
                if(trans.targetID > 0) addTransition(transitionsForObject, trans, trans.targetID, wantedObjId, stepsCount);

                count++;
            }
        }

        trace('search: $count transtions found! ${Sys.time() - startTime}');

        /*
        for(key in transitionsForObject.keys())            
        {
            var trans = transitionsForObject[key].getDesciption();

            trace('Search: ${trans}');
        }
        */

        return transitionsForObject;

        //var transitionsByOjectKeys = [for(key in transitionsByObject.keys()) key];
    }
    
    private function addTransition(transitionsForObject:Map<Int, TransitionForObject>, transition:TransitionData, objId:Int, wantedObjId:Int, steps:Int)
    {
        var transitionForObject = transitionsForObject[objId];

        if(transitionForObject == null)
        {
             transitionForObject = new TransitionForObject(objId, steps, wantedObjId, transition);
             transitionForObject.steps = steps;
             transitionForObject.bestTransition = transition;
             transitionForObject.transitions.push(new TransitionForObject(objId, steps, wantedObjId, transition));

             transitionsForObject[objId] = transitionForObject;
        }

        if(transitionForObject.steps > steps)
        {
            transitionForObject.steps = steps;
            transitionForObject.bestTransition = transition;
        } 

        transitionForObject.transitions.push(new TransitionForObject(objId, steps, wantedObjId, transition));
    }

    public function createCollisionChunk():Vector<Bool>
    {
        var player:PlayerInstance = playerInterface.getPlayerInstance();
        var world = playerInterface.getWorld(); 
        var RAD = MapData.RAD;
        var vector = new Vector<Bool>((RAD * 2) * (RAD * 2));
        var int:Int = -1;

        for (y in player.ty() - RAD...player.ty() + RAD)
        {
            for (x in player.tx() - RAD...player.tx() + RAD)
            {
                int++;

                var obj = world.getObjectHelper(x,y);
                vector[int] = obj.blocksWalking() || world.isBiomeBlocking(x,y); 

                //if(obj.blocksWalking()) trace('${player.tx()} ${player.ty()} $x $y ${obj.description}');  
            }
        }

        //trace(vector);

        return vector;       
    }
}
//time routine
//update loop
//map

class TransitionForObject
{
    public var objId:Int;
    public var wantedObjId:Int;
    public var steps:Int;

    public var bestTransition:TransitionData;

    public var transitions:Array<TransitionForObject> = [];

    public function new(objId:Int, steps:Int, wantedObjId:Int, transition:TransitionData) 
    {
        this.objId = objId;
        this.wantedObjId = wantedObjId;
        this.steps = steps;
        this.bestTransition = transition;
    }

    public function getDesciption() : String
    {
        var description = 'objId: $objId wantedObjId: $wantedObjId steps: $steps trans: ' + bestTransition.getDesciption();
        return description;
    }
}