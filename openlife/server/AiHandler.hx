package openlife.server;

import haxe.Exception;
import sys.thread.Mutex;
import openlife.settings.ServerSettings;

/**
 * AiHandler handles AI chat responses with rate limiting and retry logic.
 * Provides the main entry point for getting AI responses.
 */
class AiHandler {
	// Rate limiting: track timestamps of all calls in the last hour
	private static var callTimestamps:Array<Float> = [];
	private static var mutex:Mutex = new Mutex();

	/**
	 * Get a chat response from the AI.
	 * Handles rate limiting and retry on failure.
	 * @param inputText The user input text
	 * @return The AI response text, or null if rate limited or all retries fail
	 */
	public static function ChatResponse(inputText:String):String {
		// Check rate limit first
		if (!checkRateLimit()) {
			trace("AI rate limit exceeded");
			return null;
		}

		// Record this call
		recordCall();

		// Try to get response with one retry
		var response:String = null;
		var attempts = 0;
		var maxAttempts = 2; // Initial + 1 retry

		while (attempts < maxAttempts) {
			attempts++;
			try {
				response = AIProvider.callAi(inputText);
				break; // Success, exit loop
			} catch (e:Exception) {
				trace('AI call attempt $attempts failed: ${e.message}');

				// Only retry on network-related errors, not on API errors
				var isNetworkError = isNetworkError(e.message);
				if (!isNetworkError || attempts >= maxAttempts) {
					// Don't retry on API errors (invalid key, bad request, etc.)
					// or if we've exhausted retries
					break;
				}

				// Brief delay before retry (could add sleep here if needed)
				// trace("Retrying AI call...");
			}
		}

		return response;
	}

	/**
	 * Check if we're within the rate limit.
	 * @return true if we can make a call, false if rate limit exceeded
	 */
	private static function checkRateLimit():Bool {
		mutex.acquire();
		var result = innerCheckRateLimit();
		mutex.release();
		return result;
	}

	private static function innerCheckRateLimit():Bool {
		cleanOldTimestamps();
		var currentCount = callTimestamps.length;
		var limit = ServerSettings.AiCallsPerHour;
		return currentCount < limit;
	}

	/**
	 * Record a new call timestamp for rate limiting.
	 */
	private static function recordCall():Void {
		mutex.acquire();
		callTimestamps.push(Sys.time());
		mutex.release();
	}

	/**
	 * Remove timestamps older than 1 hour from the tracking array.
	 */
	private static function cleanOldTimestamps():Void {
		var oneHourAgo = Sys.time() - 3600; // 3600 seconds = 1 hour

		// Keep only recent timestamps
		callTimestamps = callTimestamps.filter(function(timestamp:Float):Bool {
			return timestamp > oneHourAgo;
		});
	}

	/**
	 * Determine if an error message indicates a network error
	 * that would warrant a retry vs an API error that won't succeed on retry.
	 */
	private static function isNetworkError(errorMsg:String):Bool {
		var lowerMsg = errorMsg.toLowerCase();

		// Network-related errors that might succeed on retry
		var networkPatterns = [
			"connection",
			"timeout",
			"network",
			"socket",
			"dns",
			"refused",
			"reset",
			"unreachable",
			"http error"
		];

		for (pattern in networkPatterns) {
			if (lowerMsg.indexOf(pattern) != -1) {
				return true;
			}
		}

		// API errors that won't succeed on retry
		var apiErrorPatterns = [
			"api key",
			"authentication",
			"unauthorized",
			"forbidden",
			"bad request",
			"invalid",
			"rate limit",
			"quota",
			"payment"
		];

		for (pattern in apiErrorPatterns) {
			if (lowerMsg.indexOf(pattern) != -1) {
				return false;
			}
		}

		// Default to true (retry) for unknown errors, as network issues are more common
		return true;
	}

	/**
	 * Get current rate limit status (for debugging/monitoring).
	 * @return Current number of calls in the last hour
	 */
	public static function getCurrentCallCount():Int {
		mutex.acquire();
		cleanOldTimestamps();
		var count = callTimestamps.length;
		mutex.release();
		return count;
	}

	/**
	 * Get the rate limit (max calls per hour).
	 */
	public static function getRateLimit():Int {
		return ServerSettings.AiCallsPerHour;
	}

	/**
	 * Generate relationship info text between two players.
	 * Checks alliance status, exile status, and leadership relationships.
	 * @param fromPlayer The AI player who will respond (self)
	 * @param toPlayer The human player who sent the message (other)
	 * @return Text describing the relationship between the two players
	 */
	public static function getRelationshipInfo(fromPlayer:GlobalPlayerInstance, toPlayer:GlobalPlayerInstance):String {
		var text = "";

		// Check if players are allied (same top leader)
		if (fromPlayer.isAlly(toPlayer)) {
			text += "You are allied with this player (same tribe/family). ";
		}

		// Check if friendly (ally + no recent attacks)
		if (fromPlayer.isFriendly(toPlayer)) {
			text += "You are on friendly terms with this player. ";
		}

		// Check if toPlayer is exiled by fromPlayer (AI exiled the human)
		if (toPlayer.isExiledBy(fromPlayer)) {
			text += "You have exiled this player! They are not welcome in your tribe. ";
		}

		// Check if fromPlayer (AI) is exiled by toPlayer (human exiled AI)
		if (fromPlayer.isExiledBy(toPlayer)) {
			text += "This player has exiled you! You are not welcome in their tribe. ";
		}

		// Check if toPlayer is exiled by any leader from fromPlayer's perspective
		if (toPlayer.isExiledByAnyLeaderFrom(fromPlayer)) {
			text += "This player has been exiled by a leader in your tribe. ";
		}

		// Check if toPlayer is a leader (has followers)
		var toPlayerTopLeader = toPlayer.getTopLeader();
		if (toPlayer.followPlayer != null || toPlayerTopLeader != toPlayer) {
			// toPlayer follows someone, so they are not the top leader
			// Check if toPlayer has followers
			var hasFollowers = false;
			for (p in GlobalPlayerInstance.AllPlayers) {
				if (p.followPlayer == toPlayer && p != toPlayer) {
					hasFollowers = true;
					break;
				}
			}
			if (hasFollowers) {
				text += "This player is a leader with followers in their tribe. ";
			}
		}

		// Check if fromPlayer (AI) is the leader of toPlayer (human follows AI)
		if (toPlayer.followPlayer == fromPlayer) {
			text += "This player follows you as their leader! ";
		}

		// Check if fromPlayer (AI) is the top leader of toPlayer
		if (toPlayerTopLeader == fromPlayer) {
			text += "You are the top leader of this player's tribe! ";
		}

		// Check if toPlayer is the leader of fromPlayer (AI follows human)
		var fromPlayerTopLeader = fromPlayer.getTopLeader();
		if (fromPlayer.followPlayer == toPlayer) {
			text += "You follow this player as your leader! ";
		}

		// Check if toPlayer is the top leader of fromPlayer
		if (fromPlayerTopLeader == toPlayer) {
			text += "This player is the top leader of your tribe! ";
		}

		return text;
	}

	/**
	 * Generate AI response to a message from another player.
	 * Builds context from both players and sends to AI for roleplay response.
	 * @param fromPlayer The AI player who will respond
	 * @param toPlayer The human player who sent the message
	 * @param message The message from the human player
	 * @return The AI's response text, or null if rate limited
	 */
	public static function respondToPlayer(fromPlayer:GlobalPlayerInstance, toPlayer:GlobalPlayerInstance, message:String):String {
		// Get context about the AI (fromPlayer)
		var ownContext = fromPlayer.playerSoul.getSoulText();

		// Get context about the human player (toPlayer)
		var otherContext = toPlayer.playerSoul.getExternalIntro();

		// Get relationship info between the two players
		var relationshipContext = getRelationshipInfo(fromPlayer, toPlayer);

		// Combine context and message for the AI
		var fullPrompt = ownContext
			+ "\n"
			+ otherContext
			+ "\n"
			+ relationshipContext
			+ "\nThe other player says to you respond in your role considering your status / prestige and the other players status / prestige: "
			+ message;

		trace(fullPrompt);

		var response = ChatResponse(fullPrompt);

		trace(response);

		return response;
	}

	/**
	 * Test function to verify AI connection is working.
	 * Makes a test call with a simple message and traces the response.
	 */
	public static function Test():Void {
		trace("=== AI Handler Test Start ===");

		var testMessage = "Hello, please respond with 'AI is working!' if you receive this.";
		trace('Sending test message: $testMessage');

		var response = ChatResponse(testMessage);

		if (response != null) {
			trace('AI Test Response: $response');
			trace("=== AI Handler Test SUCCESS ===");
		}
		else {
			trace("AI Test Response: null (failed or rate limited)");
			trace("=== AI Handler Test FAILED ===");
		}

		// Print rate limit status
		trace('Rate limit status: ${getCurrentCallCount()} / ${getRateLimit()} calls used');
	}
}
