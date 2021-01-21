package openlife.auto;

import openlife.data.object.player.PlayerInstance;
using StringTools;

class Ai
{
    var playerInterface:PlayerInterface;

    public function new(player:PlayerInterface) 
    {
        this.playerInterface = player;
    }

    //public var player(get, null):Int; 

    public function doTimeStuff(timePassedInSeconds:Float) 
    {
        // @PX do time stuff here is called from TimeHelper
    }

    public function say(player:PlayerInstance,curse:Bool,text:String)
    {
        var myPlayer = playerInterface.getPlayerInstance();
        //trace('im a super evil bot!');

        //trace('ai3: ${myPlayer.p_id} player: ${player.p_id}');

        if (myPlayer.p_id == player.p_id) return;

        //trace('im a evil bot!');

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
}
//time routine
//update loop
//map