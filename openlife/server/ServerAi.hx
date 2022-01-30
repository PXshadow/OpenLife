package openlife.server;

import openlife.settings.ServerSettings;
import openlife.auto.Ai;

class ServerAi extends Ai
{
    public static var AiIdIndex = 1;
    public var player:GlobalPlayerInstance;
    public var connection:Connection;
    public var timeToRebirth:Float = 0;

    public function new(newPlayer:GlobalPlayerInstance)
    {
        super(newPlayer);
        this.player = newPlayer;
        this.connection = player.connection;
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

    public function doRebirth(timePassedInSeconds:Float)
    {
        if(timeToRebirth == 0) timeToRebirth = (ServerSettings.TimeToAiRebirth / 2) + WorldMap.calculateRandomFloat() * ServerSettings.TimeToAiRebirth;
        timeToRebirth -= timePassedInSeconds;

        if(timeToRebirth > 0) return;

        //trace('doRebirth: ');    

        this.player = GlobalPlayerInstance.CreateNewAiPlayer(connection);
        this.myPlayer = player; // TODO same player for AI
        this.newBorn();
    }
}