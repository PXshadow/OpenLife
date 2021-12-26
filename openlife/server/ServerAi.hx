package openlife.server;

import openlife.auto.Ai;

class ServerAi extends Ai
{
    public var player:GlobalPlayerInstance;

    public function new(newPlayer:GlobalPlayerInstance)
    {
        super(newPlayer);
        this.player = newPlayer;
        player.connection.serverAi = this;

        Connection.addAi(this);
    }

    public static function createNewServerAiWithNewPlayer() : ServerAi
    {
        var newConnection = new Connection(null, Server.server);
        var newPlayer = GlobalPlayerInstance.CreateNewAiPlayer(newConnection);

        return new ServerAi(newPlayer);
    }
}