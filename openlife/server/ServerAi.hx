package openlife.server;

import openlife.auto.Ai;
import openlife.auto.AiBase;
import openlife.auto.AiPx;
import openlife.auto.AiPx;
import openlife.settings.ServerSettings;

class ServerAi{
	public static var AiIdIndex = 1;

	public var player:GlobalPlayerInstance;
	public var ai:AiBase;
	public var number:Int;
	public var connection:Connection;
	public var timeToRebirth:Float = 0;

	public function new(newPlayer:GlobalPlayerInstance) {
		
		ai = ServerSettings.NumberOfAiPx >= AiIdIndex ? new AiPx(newPlayer) : new Ai(newPlayer) ;
		this.number = AiIdIndex++;
		this.player = newPlayer;
		this.connection = player.connection;
		player.connection.serverAi = this;

		Connection.addAi(this);
	}

	public static function createNewServerAiWithNewPlayer():ServerAi {
		var email = 'AI${AiIdIndex}';
		var newConnection = new Connection(null, Server.server);
		newConnection.playerAccount = PlayerAccount.GetOrCreatePlayerAccount(email, email);
		newConnection.playerAccount.isAi = true;
		var newPlayer = GlobalPlayerInstance.CreateNewAiPlayer(newConnection);

		var serverAi = new ServerAi(newPlayer);

		trace('new ai: ${serverAi.number} ${newConnection.playerAccount.email}');

		serverAi.ai.newBorn();
		return serverAi;
	}

	public function doRebirth(timePassedInSeconds:Float) {
		// TODO limit / increase Ais if serversettings change, or connected players change
		if (this.player.account.isAi == false) // it was a replacement for a player
		{
			trace('remove ai because it was to replace human: ${this.number}');
			Connection.removeAi(this);
			return;
		}
		if(this.number > ServerSettings.NumberOfAis)
		{
			trace('remove ai because to many ai: ${this.number}');
			Connection.removeAi(this);
			return;
		}
		
		if (timeToRebirth == 0){
			var agefactor = Math.max(1, player.age - 60);
			var waitingTime = agefactor * ServerSettings.TimeToAiRebirthPerYear;
			timeToRebirth = 2 * waitingTime * WorldMap.calculateRandomFloat();
		}

		timeToRebirth -= timePassedInSeconds;

		if (timeToRebirth > 0) return;
		timeToRebirth = 0;
		// trace('doRebirth: ');

		this.player = GlobalPlayerInstance.CreateNewAiPlayer(connection);
		this.ai.myPlayer = player; // TODO same player for AI
		this.ai.newBorn();
	}

	public function doTimeStuff(timePassedInSeconds:Float){
		ai.doTimeStuff(timePassedInSeconds);
	}
}
