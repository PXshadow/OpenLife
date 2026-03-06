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

		// Combine context and message for the AI
		var fullPrompt = ownContext
			+ "\n"
			+ otherContext
			+ "\nThe other player says to you respond in your role considering your status / prestige and the other players status / prestige: "
			+ message;

		trace(fullPrompt);

		var response = ChatResponse(fullPrompt);

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
