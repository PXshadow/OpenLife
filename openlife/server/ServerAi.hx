package openlife.server;

import openlife.auto.Ai;

class ServerAi extends Ai
{
    public var player:GlobalPlayerInstance;

    public function new()
    {
        player = new GlobalPlayerInstance();
        super(player);
        player.serverAi = this;
        Connection.addAi(player.serverAi);
    }
}