package openlife.macros;

// import openlife.server.WorldMap;
// import openlife.server.GlobalPlayerInstance;
import haxe.macro.Expr;

class Macro {
	public static macro function exception(expr:Expr) {
		return macro if (openlife.settings.ServerSettings.debug) {
			$expr;
		} else {
			try {
				$expr;
			} catch (e) {
				trace('WARNING: ' + e + '\n' + e.details());
			}
		}
	}
	/*
		public static macro function doException(expr:Expr, player:GlobalPlayerInstance, targetPlayer:GlobalPlayerInstance)
		{
			var done = false;
			// make sure that if both players at the same time try to interact with each other it does not end up in a dead lock 
			while(targetPlayer.mutex.tryAcquire() == false)
			{
				player.mutex.release();

				Sys.sleep(WorldMap.calculateRandomFloat() / 5);

				player.mutex.acquire();
			} 

			Macro.exception(done = expr);

			// send always PU so that player wont get stuck
			if(done == false)
			{
				player.connection.send(PLAYER_UPDATE,[player.toData()]);
				player.connection.send(FRAME);
			}

			targetPlayer.mutex.release();
	}*/
}
// Mutex example
/**if(ServerSettings.useOnePlayerMutex) AllPlayerMutex.acquire();
	else
	{
		this.mutex.acquire();

		// make sure that if both players at the same time try to interact with each other it does not end up in a dead lock 
		while(targetPlayer.mutex.tryAcquire() == false)
		{
			this.mutex.release();

			Sys.sleep(WorldMap.calculateRandomFloat() / 5);

			this.mutex.acquire();
		} 
}  **/
