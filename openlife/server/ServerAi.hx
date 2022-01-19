package openlife.server;

import openlife.auto.Ai;

class ServerAi extends Ai
{
    public static var AiIdIndex = 1;
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
        var email = 'AI${AiIdIndex++}';
        var newConnection = new Connection(null, Server.server);
        newConnection.playerAccount = PlayerAccount.GetOrCreatePlayerAccount(email, email);
        newConnection.playerAccount.isAi = true;
        var newPlayer = GlobalPlayerInstance.CreateNewAiPlayer(newConnection);

        return new ServerAi(newPlayer);
    }
}