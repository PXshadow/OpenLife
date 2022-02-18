package openlife.server;

import openlife.settings.ServerSettings;
import openlife.auto.Ai;

class ServerAi extends Ai
{
    public static var AiIdIndex = 1;

    public var player:GlobalPlayerInstance;
    public var number:Int;
    public var connection:Connection;
    public var timeToRebirth:Float = 0;


    public function new(newPlayer:GlobalPlayerInstance)
    {
        super(newPlayer);

        this.number = AiIdIndex++;
        this.player = newPlayer;
        this.connection = player.connection;
        player.connection.serverAi = this;

        Connection.addAi(this);
    }

    public static function createNewServerAiWithNewPlayer() : ServerAi
    {
        var email = 'AI${AiIdIndex}';
        var newConnection = new Connection(null, Server.server);
        newConnection.playerAccount = PlayerAccount.GetOrCreatePlayerAccount(email, email);
        newConnection.playerAccount.isAi = true;
        var newPlayer = GlobalPlayerInstance.CreateNewAiPlayer(newConnection);

        var ai = new ServerAi(newPlayer);
        
        trace('new ai: ${ai.number} ${newConnection.playerAccount.email}');  
        
        ai.newBorn();
        return ai;
    }

    public function doRebirth(timePassedInSeconds:Float)
    {
        // TODO limit / increase Ais if serversettings change, or connected players change 
        //if(this.number > ServerSettings.NumberOfAis)
        if(this.player.account.isAi == false) // it was a replacement for a player 
        {
            trace('remove ai: ${this.number}');   
            Connection.removeAi(this);
            return;
        }

        if(timeToRebirth == 0) timeToRebirth = (ServerSettings.TimeToAiRebirth / 2) + WorldMap.calculateRandomFloat() * ServerSettings.TimeToAiRebirth;
        timeToRebirth -= timePassedInSeconds;

        if(timeToRebirth > 0) return;
        timeToRebirth = 0;
        //trace('doRebirth: ');    
        
        this.player = GlobalPlayerInstance.CreateNewAiPlayer(connection);
        this.myPlayer = player; // TODO same player for AI
        this.newBorn();
    }
}