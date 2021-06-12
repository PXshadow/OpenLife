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
    }

    public function searchFoodAndEat()
    {
        var myPlayer = playerInterface.getPlayerInstance();
        foodTarget = searchBestFood();
        if(foodTarget != null) playerInterface.Goto(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
    }

    public function doTimeStuff(timePassedInSeconds:Float) 
    {
        // @PX do time stuff here is called from TimeHelper

        var myPlayer = playerInterface.getPlayerInstance();

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
            }
            
            return;
        }

        if(foodTarget == null && playerInterface.isMoving() == false)
        {
            if(playerToFollow != null && playerInterface.CalculateDistanceToPlayer(playerToFollow) > 2)
            {
                playerInterface.Goto(playerToFollow.tx() + 1 - myPlayer.gx, playerToFollow.ty() - myPlayer.gy);
            }
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
                    var emptyTileObj = AiHelper.GetClosestObject(playerInterface, emptyObject);
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

    // is called once a movement is finished (client side it must be called manually after a PlayerUpdate)
    public function finishedMovement()
    {

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
                // TODO directly get object data for speed without creating helperobject
                if(world.getObjectId(tx, ty)[0] == 0) continue; // to speed up dont create object helper for empty objects

                var obj = world.getObjectHelper(tx, ty);
                var objData = obj.objectData;

                if(objData.dummyParent !=null) objData = objData.dummyParent; // use parent objectdata

                //var distance = calculateDistance(baseX, baseY, obj.tx, obj.ty);
                //trace('search food $tx, $ty: foodvalue: ${objData.foodValue} bestdistance: $bestDistance distance: $distance ${obj.description}');

                //var tmp = ObjectData.getObjectData(31);
                //trace('berry food: ${tmp.foodValue}');

                if(objData.foodValue > 0 || objData.foodFromTarget != null)                    
                {
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

