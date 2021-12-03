package openlife.auto;

import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;
import openlife.data.Pos;
import openlife.auto.Pathfinder.Coordinate;
import openlife.data.map.MapData;
import openlife.data.object.player.PlayerInstance;
import openlife.data.transition.TransitionData;
import haxe.ds.Vector;

class AiHelper
{
    static final RAD:Int = MapData.RAD; // search radius

    public static function CalculateDistanceToPlayer(playerInterface:PlayerInterface, playerTo:PlayerInstance) : Float
    {
        var player = playerInterface.getPlayerInstance();
        return CalculateDistance(player.tx(), player.ty(), playerTo.tx(), playerTo.ty());
    }

    public static function CalculateDistanceToObject(playerInterface:PlayerInterface, obj:ObjectHelper) : Float
    {
        var player = playerInterface.getPlayerInstance();
        return CalculateDistance(player.tx(), player.ty(), obj.tx, obj.ty);
    }

    public static function CalculateDistance(baseX:Int, baseY:Int, toX:Int, toY:Int) : Float
    {
        return (toX - baseX) * (toX - baseX) + (toY - baseY) * (toY - baseY);
    }

    public static function GetClosestObjectById(playerInterface:PlayerInterface, objId:Int, ignoreObj:ObjectHelper = null) : ObjectHelper
    {
        var objData = ObjectData.getObjectData(objId);
        return GetClosestObject(playerInterface, objData, ignoreObj);
    }

    public static function GetClosestObject(playerInterface:PlayerInterface, objDataToSearch:ObjectData, ignoreObj:ObjectHelper = null) : ObjectHelper
    {
        var world = playerInterface.getWorld();
        var player = playerInterface.getPlayerInstance();
        var baseX = player.tx();
        var baseY = player.ty();
        var closestObject = null;
        var bestDistance = 0.0;

        for(ty in baseY - RAD...baseY + RAD)
        {
            for(tx in baseX - RAD...baseX + RAD)
            {
                if(ignoreObj != null && ignoreObj.tx == tx && ignoreObj.ty == ty) continue;
                var objData = world.getObjectDataAtPosition(tx, ty);
                
                if(objData.parentId == objDataToSearch.parentId)  // compare parent, because of dummy objects for obj with numberOfuses > 1 may have different IDs                    
                {
                    var obj = world.getObjectHelper(tx, ty);
                    var distance = AiHelper.CalculateDistance(baseX, baseY, obj.tx, obj.ty);

                    if(closestObject == null || distance < bestDistance)
                    {
                        closestObject = obj;
                        bestDistance = distance;
                    }
                }
            }
        }

        //if(closestObject !=null) trace('AI: bestdistance: $bestDistance ${closestObject.description}');

        return closestObject;
    }

    // TODO goto uses global coordinates
    public static function Goto(playerInterface:PlayerInterface, x:Int,y:Int):Bool
    {
        var player = playerInterface.getPlayerInstance();

        

        //var goal:Pos;
        //var dest:Pos;
        //var init:Pos;

        //if (player.x == x && player.y == y || moving) return false;
        //set pos
        var px = x - player.x;
        var py = y - player.y;

        //trace('AAI: GOTO: From: ${player.x},${player.y} To: $x $y / FROM ${player.tx()},${player.ty()} To: ${x + player.gx},${y + player.gy}');

        if(px == 0 && py == 0) return false; // no need to move

        if (px > RAD - 1) px = RAD - 1;
        if (py > RAD - 1) py = RAD - 1;
        if (px < -RAD) px = -RAD;
        if (py < -RAD) py = -RAD;
        //cords
        var start = new Coordinate(RAD,RAD);

        //trace('Goto: $px $py');

        var map = new MapCollision(AiHelper.CreateCollisionChunk(playerInterface));
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

        /*
        goal = new Pos(x,y);

        if (px == goal.x - player.x && py == goal.y - player.y)
        {
            trace("shift goal!");
            //shift goal as well
            goal.x += tweakX;
            goal.y += tweakY;
        }*/

        //dest = new Pos(px + player.x, py + player.y);
        //init = new Pos(player.x,player.y);
        //movePlayer(data);
        playerInterface.move(player.x, player.y, player.done_moving_seqNum++, data);

        //isMoving = true;

        return true;
    }

    private static function CreateCollisionChunk(playerInterface:PlayerInterface):Vector<Bool>
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

    public static function SearchTransitions(playerInterface:PlayerInterface, objectIdToSearch:Int) : Map<Int, TransitionForObject>
    {    
        var world = playerInterface.getWorld();
        var transitionsByObject = new Map<Int, TransitionData>();
        var transitionsForObject = new Map<Int, TransitionForObject>();
        
        var transitionsToProcess = new Array<Array<TransitionData>>();
        var steps = new Array<Int>();
        var wantedObjIds = new Array<Int>();
        var stepsCount = 1;

        transitionsToProcess.push(world.getTransitionByNewTarget(objectIdToSearch)); 
        transitionsToProcess.push(world.getTransitionByNewActor(objectIdToSearch)); 

        //var tmpTrans = world.getTransitionByNewTarget(objectIdToSearch);
        //trace('TEST' + tmpTrans);

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

            //if(transitionsByObject.exists(wantedObjId)) continue;

            for(trans in transitions)
            {
                // TODO ???
                //if(transitionsByObject.exists(trans.actorID) || transitionsByObject.exists(trans.targetID)) continue;

                //if(trans.newActorID == 34 ) trace('TEST1 WANTED: $wantedObjId T: ' + trans.getDesciption(true));
                 trace('TEST1 WANTED: $wantedObjId T: ' + trans.getDesciption(true));
                //  Not helpful to create a basket 292 + 211 = 336 + 411 / Basket + Fertile Soil Deposit  -->  Basket of Soil + Fertile Soil Pit#partial
                // TODO allow if new actor or target is closer to wanted object
                // like fill a basket of soil to create a basket
                if(transitionsByObject.exists(trans.actorID) && trans.newActorID != trans.actorID && trans.newActorID != objectIdToSearch)
                {
                    var tmpActor = transitionsForObject[trans.actorID];
                    var actorSteps = tmpActor != null ? tmpActor.steps : 10000;
                    var tmpNewActor = transitionsForObject[trans.newActorID];
                    var newActorSteps = tmpNewActor != null ? tmpNewActor.steps : 10000;

                    trace('TEST1 actorSteps: $actorSteps newActorSteps: $newActorSteps');
                    if(actorSteps < newActorSteps) continue;
                }

                // TODO above for target

                 trace('TEST2');
                if(trans.actorID == wantedObjId || trans.actorID == objectIdToSearch) continue;  
                 trace('TEST3');
                if(trans.targetID == wantedObjId || trans.targetID == objectIdToSearch) continue; 
                 trace('TEST4');

                //if(count < 10000) trans.traceTransition('AI stepsCount: $stepsCount count: $count:', true);

                //if(trans.actorID > 0 && trans.actorID != trans.newActorID && transitionsByObject.exists(trans.actorID) == false)
                if(trans.actorID > 0 && transitionsByObject.exists(trans.actorID) == false)
                {
                    trace('TEST5');
                    transitionsToProcess.push(world.getTransitionByNewTarget(trans.actorID)); 
                    transitionsToProcess.push(world.getTransitionByNewActor(trans.actorID)); 

                    steps.push(stepsCount + 1);
                    steps.push(stepsCount + 1);

                    wantedObjIds.push(trans.actorID);
                    wantedObjIds.push(trans.actorID);
                }

                //if(trans.targetID > 0 && trans.targetID != trans.newTargetID && transitionsByObject.exists(trans.targetID) == false)
                if(trans.targetID > 0 && transitionsByObject.exists(trans.targetID) == false)
                {
                    trace('TEST6');
                    transitionsToProcess.push(world.getTransitionByNewTarget(trans.targetID)); 
                    transitionsToProcess.push(world.getTransitionByNewActor(trans.targetID)); 

                    steps.push(stepsCount + 1);
                    steps.push(stepsCount + 1);

                    wantedObjIds.push(trans.targetID);
                    wantedObjIds.push(trans.targetID);
                }

                if(trans.actorID > 0) transitionsByObject[trans.actorID] = trans;
                if(trans.targetID > 0) transitionsByObject[trans.targetID] = trans;

                if(trans.actorID > 0) AddTransition(transitionsForObject, trans, trans.actorID, wantedObjId, stepsCount);
                if(trans.targetID > 0) AddTransition(transitionsForObject, trans, trans.targetID, wantedObjId, stepsCount);

                count++;
            }

            if(count > 400) break; // TODO remove
        }

        trace('AI trans search: $count transtions found! ${Sys.time() - startTime}');

        
        for(key in transitionsForObject.keys())            
        {
            var trans = transitionsForObject[key].getDesciption();

            trace('AI Search: ${trans}');
        }
        

        return transitionsForObject;

        //var transitionsByOjectKeys = [for(key in transitionsByObject.keys()) key];
    }
    
    private static function AddTransition(transitionsForObject:Map<Int, TransitionForObject>, transition:TransitionData, objId:Int, wantedObjId:Int, steps:Int)
    {
        var transitionForObject = transitionsForObject[objId];

        if(transitionForObject == null)
        {
                transitionForObject = new TransitionForObject(objId, steps, wantedObjId, transition);
                transitionForObject.steps = steps;
                transitionForObject.bestTransition = transition;
                transitionForObject.transitions.push(new TransitionForObject(objId, steps, wantedObjId, transition));
                //transitionForObject.transitions.push(transition);

                transitionsForObject[objId] = transitionForObject;

                return;
        }

        if(transitionForObject.steps > steps)
        {
            transitionForObject.steps = steps;
            transitionForObject.bestTransition = transition;
        } 

        transitionForObject.transitions.push(new TransitionForObject(objId, steps, wantedObjId, transition));
        //transitionForObject.transitions.push(transition);
    }
   
    //time routine
    //update loop
    //map
}

class TransitionForObject
{
    public var objId:Int;
    public var wantedObjId:Int;
    public var steps:Int;

    public var bestTransition:TransitionData;

    //public var transitions:Array<TransitionData> = [];

    public var transitions:Array<TransitionForObject> = [];

    public var closestObject:ObjectHelper;
    public var closestObjectDistance:Float;

    public var secondObject:ObjectHelper; // in case you need two object like using two milkeed
    public var secondObjectDistance:Float;

    public function new(objId:Int, steps:Int, wantedObjId:Int, transition:TransitionData) 
    {
        this.objId = objId;
        this.wantedObjId = wantedObjId;
        this.steps = steps;
        this.bestTransition = transition;
    }

    public function getDesciption() : String
    {
        var description = 'objId: $objId wantedObjId: $wantedObjId steps: $steps trans: ' + bestTransition.getDesciption(true);
        return description;
    }
}