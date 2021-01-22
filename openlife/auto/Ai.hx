package openlife.auto;

import openlife.data.transition.TransitionData;
import openlife.data.object.player.PlayerInstance;
using StringTools;

class Ai
{
    var playerInterface:PlayerInterface;

    var done = false;

    public function new(player:PlayerInterface) 
    {
        this.playerInterface = player;
    }

    //public var player(get, null):Int; 

    public function doTimeStuff(timePassedInSeconds:Float) 
    {
        // @PX do time stuff here is called from TimeHelper

        if(done) return;

        done = true;

        searchTransitions(273);
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

        if (text.indexOf("HELLO") != -1) 
        {
            //HELLO WORLD

            //trace('im a nice bot!');

            playerInterface.say("HELLO WORLD");
        }
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

    private function searchTransitions(objectIdToSearch:Int)
    {    
        // TODO might be good to also allow multiple transitions for one object
        var world = this.playerInterface.getWorld();
        var transitionsByObject = new Map<Int, TransitionData>();
        var transitionsToProcess = new Array<Array<TransitionData>>();

        transitionsToProcess.push(world.getTransitionByNewTarget(objectIdToSearch)); 
        transitionsToProcess.push(world.getTransitionByNewActor(objectIdToSearch)); 

        var count = 1;

        while (transitionsToProcess.length > 0)
        {
            var transitions = transitionsToProcess.pop();

            for(trans in transitions)
            {
                if(transitionsByObject.exists(trans.actorID) && transitionsByObject.exists(trans.targetID)) continue;

                if(count < 100) trans.traceTransition('AI $count:', true);

                if(trans.actorID > 0 && trans.actorID != trans.newActorID && transitionsByObject.exists(trans.actorID) == false)
                {
                    transitionsToProcess.push(world.getTransitionByNewTarget(trans.actorID)); 
                    transitionsToProcess.push(world.getTransitionByNewActor(trans.actorID)); 
                }

                if(trans.targetID > 0 && trans.targetID != trans.newTargetID && transitionsByObject.exists(trans.targetID) == false)
                {
                    transitionsToProcess.push(world.getTransitionByNewTarget(trans.targetID)); 
                    transitionsToProcess.push(world.getTransitionByNewActor(trans.targetID)); 
                }

                transitionsByObject[trans.actorID] = trans;
                transitionsByObject[trans.targetID] = trans;

                count++;
            }
        }

        trace('search: $count transtions found!');
        //ar transitionsByOjectKeys = [for(key in transitionsByObject.keys()) key];

        for(key in transitionsByObject.keys())            
        {
            var trans = transitionsByObject[key].getDesciption();

            trace('Search: object: $key trans: ${trans}');
        }

        /*

        for(trans in transitions)
        {
            trans.traceTransition("AI:", true);

            transitionsByObject[trans.actorID] = trans;
            transitionsByObject[trans.targetID] = trans;

            if(trans.actorID > 0 transitionsByObject) // ignore time - 1 = transitions 
            {
                var actorTransitions = world.getTransitionByNewTarget(trans.actorID);

                for(actorTrans in actorTransitions)
                {
                    actorTrans.traceTransition("AI Actor:", true);                
                }
            }

            if(trans.targetID > 0) // ignore time - 1 = player transitions and other "strange" transitions 
            {
                var targetTransitions = world.getTransitionByNewTarget(trans.targetID);

                for(targetTrans in targetTransitions)
                {
                    targetTrans.traceTransition("AI Target:", true);
                }
            }
        }

        */
    }
}
//time routine
//update loop
//map