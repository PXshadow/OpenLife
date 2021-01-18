package openlife.auto;

import openlife.data.object.player.PlayerInstance;
using StringTools;

class Ai {
    public var myPlayer:PlayerInstance;
    var handler:MessageHandler;

    public function new(player:PlayerInstance,handler:MessageHandler) 
    {
        this.myPlayer = player;
        this.handler = handler;
    }

    public function update() 
    {

    }

    public function say(player:PlayerInstance,curse:Bool,text:String)
    {
        trace('im a super evil bot!');

        trace('ai3: ${myPlayer.p_id} player: ${player.p_id}');

        if (this.myPlayer.p_id == player.p_id) return;

        trace('im a evil bot!');

        if (text.indexOf("HELLO") != -1) 
        {
            //HELLO WORLD

            trace('im a nice bot!');

            handler.say("HELLO WORLD");
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