package openlife.auto;

import openlife.data.object.player.PlayerInstance;
using StringTools;

class Ai {
    public var player:PlayerInstance;
    var handler:MessageHandler;
    public function new(player:PlayerInstance,handler:MessageHandler) 
    {
        this.player = player;
        this.handler = handler;
    }

    public function update() 
    {

    }

    public function say(player:PlayerInstance,curse:Bool,text:String)
    {
        if (this.player.p_id == player.p_id) return;
        if (text.indexOf("HELLO") != -1) 
        {
            trace("-----------------------------------------");
            //HELLO WORLD
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