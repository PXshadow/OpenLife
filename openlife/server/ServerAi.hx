package openlife.server;

import openlife.auto.Ai;

class ServerAi extends Ai
{
    public var player:GlobalPlayerInstance;

    public function new()
    {
        player = GlobalPlayerInstance.CreateNewAiPlayer(this); 
        super(player);
        //player.serverAi = this;
        player.connection = new Connection(null, Server.server); 
        player.connection.serverAi = this;

        Connection.addAi(player.serverAi);
    }
}