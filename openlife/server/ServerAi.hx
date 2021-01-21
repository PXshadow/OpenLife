package openlife.server;

import openlife.auto.Ai;

class ServerAi extends Ai
{
    public var player:GlobalPlayerInstance;

    public function new(player:GlobalPlayerInstance)
    {
        super(player);

        this.player = player;
    }  
    
    public static function CreateNew() : ServerAi 
    {
        var player = GlobalPlayerInstance.CreateNew();       
        var ai = new ServerAi(player);

        Connection.addAi(ai);

        return ai;
    }
}