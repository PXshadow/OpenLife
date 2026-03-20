package openlife.server;

import haxe.Exception;
import sys.thread.Mutex;
import sys.thread.Thread;
import sys.io.File;
import sys.io.FileOutput;
import openlife.settings.ServerSettings;
import openlife.auto.AiBase;

/**
 * AiHandler handles AI chat responses with rate limiting and retry logic.
 * Provides the main entry point for getting AI responses.
 */
class AiHandler {
	// Rate limiting: track timestamps of all calls in the last hour
	private static var callTimestamps:Array<Float> = [];
	private static var mutex:Mutex = new Mutex();

	// File logging for AI conversations
	private static var logMutex:Mutex = new Mutex();
	private static var logFileBaseName:String = "log/ai_conversation_log";

	/**
	 * Set the log file base name for AI conversation logging.
	 * The date will be appended to create daily log files.
	 * @param baseName The base file name (without date and extension)
	 */
	public static function setLogFilePath(baseName:String):Void {
		logMutex.acquire();
		logFileBaseName = baseName;
		logMutex.release();
	}

	/**
	 * Get today's date formatted as YYYY-MM-DD.
	 * @return The formatted date string
	 */
	private static function getDateString():String {
		var now = Date.now();
		var year = now.getFullYear();
		var month = now.getMonth() + 1;
		var day = now.getDate();
		return StringTools.lpad(Std.string(year), "0", 4) + "-" + StringTools.lpad(Std.string(month), "0", 2) + "-" + StringTools.lpad(Std.string(day), "0", 2);
	}

	/**
	 * Get the current log file path with date appended.
	 * @return The full log file path for today
	 */
	private static function getCurrentLogFilePath():String {
		return logFileBaseName + "_" + getDateString() + ".txt";
	}

	/**
	 * Thread-safe method to log AI conversation to a file.
	 * Logs the timestamp, fullPrompt, and response.
	 * @param fullPrompt The prompt sent to the AI
	 * @param response The response from the AI (can be null if failed)
	 */
	private static function logToFile(fullPrompt:String, response:String):Void {
		logMutex.acquire();
		try {
			// Ensure log directory exists
			var logDir = "log";
			if (!sys.FileSystem.exists(logDir)) {
				sys.FileSystem.createDirectory(logDir);
			}

			var timestamp = Date.now().toString();
			var logFilePath = getCurrentLogFilePath();
			var logEntry = "========================================\n";
			logEntry += "Timestamp: " + timestamp + "\n";
			logEntry += "Log File: " + logFilePath + "\n\n";
			logEntry += "--- FULL PROMPT ---\n" + fullPrompt + "\n\n";
			logEntry += "--- RESPONSE ---\n" + (response != null ? response : "(null - failed or rate limited)") + "\n";
			logEntry += "========================================\n\n";

			File.append(logFilePath).writeString(logEntry);
		} catch (e:Dynamic) {
			trace('Failed to write to log file: $e');
		}
		logMutex.release();
	}

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

		// Start timing
		var startTime = Sys.time();

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

		// Calculate elapsed time in seconds
		var elapsedSeconds = Sys.time() - startTime;

		// Log timing info
		if (response != null) {
			trace('AI call succeeded in ${Math.round(elapsedSeconds)} seconds - attempts: ${attempts}');
			trace(response);
		}
		else {
			trace('AI call failed after ${Math.round(elapsedSeconds)} seconds - attempts: ${attempts}');
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
		var fullPrompt = buildPrompt(fromPlayer, toPlayer, message);
		trace(fullPrompt);
		var response = ChatResponse(fullPrompt);
		// trace(response);
		return response;
	}

	/**
	 * Build the prompt string for AI response from player context.
	 * Extracts soul text, relationship info, memory, and combines with the message.
	 * @param fromPlayer The AI player who will respond
	 * @param toPlayer The human player who sent the message
	 * @param message The message from the human player
	 * @return The full prompt string to send to the AI
	 */
	private static function buildPrompt(fromPlayer:GlobalPlayerInstance, toPlayer:GlobalPlayerInstance, message:String):String {
		// Get context about the AI (fromPlayer)
		var ownContext = fromPlayer.playerSoul.getSoulText();

		// Get context about the human player (toPlayer)
		var otherContext = toPlayer.playerSoul.getExternalIntro();

		// Get relationship info between the two players
		var relationshipContext = getRelationshipInfo(fromPlayer, toPlayer);
		var doCommandText = checkIfShouldDoCommand(fromPlayer, toPlayer);

		// Get memory context (interactions with other players)
		var memoryContext = fromPlayer.playerSoul.getMemoryText();

		// Get chat memory context
		var chatMemoryContext = fromPlayer.playerSoul.getChatMemoryText();

		// Combine context and message for the AI
		var prompt = ownContext + "\n" + otherContext + "\n" + relationshipContext + "\n" + doCommandText;

		// Add memory context if available
		if (memoryContext.length > 0) {
			prompt += "\n" + memoryContext;
		}

		// Add chat memory context if available
		if (chatMemoryContext.length > 0) {
			prompt += "\n" + chatMemoryContext;
		}

		prompt += "\n" + getCommandContext();

		prompt += "\nThe other player says to you respond in your role considering your status / prestige and the other players status / prestige: " + message;

		return prompt;
	}

	public static function checkIfShouldDoCommand(fromPlayer:GlobalPlayerInstance, toPlayer:GlobalPlayerInstance) {
		if (fromPlayer.isFollowerFrom(toPlayer)) return "You are a follower of this player! Therefore if asked you should do commands!";
		// TODO check if exiled
		if (fromPlayer.isCloseRelative(toPlayer)) return "You are a close relative of this player so if asked you should help him / her!";

		// myPlayer.say('I AM NOT YOUR FOLLOWER!');
		// myPlayer.doEmote(Emote.angry);
		return "You are not a follower of this player, so if asked you can reject commands of this player!";
	}

	private static function getCommandContext():String {
		return "Always respond with valid JSON including a fitting emote and any actions you want to perform:
{ \"text\": \"your response\", \"emote\": \"emoteName\" }
Add only the actions you want to perform:
- \"text\": your roleplay response
- \"emote\": one ONLY: happy, angry, love, sad, joy, blush, devious, shock, terrified, homesick, mad, oreally, ill, hmph, snowSplat
- \"followPlayer\": true - to stay close / walk with the player
- \"drop\": true - to drop held item
- \"makeItem\": \"item name or id\" - to craft an item (e.g. \"knife\" or 71)

Examples:
{ \"text\": \"I will follow you!\", \"emote\": \"happy\", \"followPlayer\": true }
{ \"text\": \"Making a knife.\", \"emote\": \"joy\", \"makeItem\": \"knife\" }
{ \"text\": \"Hello!\", \"emote\": \"joy\" }";
	}

	private static function getEmoteId(emoteName:String):Int {
		switch (emoteName.toLowerCase()) {
			case "happy":
				return 0;
			case "mad":
				return 1;
			case "angry":
				return 2;
			case "sad":
				return 3;
			case "devious":
				return 4;
			case "joy":
				return 5;
			case "blush":
				return 6;
			case "snowSplat":
				return 8;
			case "oreally":
				return 14;
			case "ill":
				return 10;
			case "hmph":
				return 12;
			case "shock":
				return 15;
			case "love":
				return 13;
			case "terrified":
				return 27;
			case "homesick":
				return 28;
			default:
				return -1;
		}
	}

	public static function parseAiResponse(response:String, aiPlayer:GlobalPlayerInstance):String {
		var text = response;
		try {
			var json = haxe.Json.parse(response);
			if (Reflect.hasField(json, "text")) {
				text = json.text;
			}

			// Handle emote at top level
			if (Reflect.hasField(json, "emote")) {
				var emoteName = json.emote;
				var emoteId = getEmoteId(emoteName);
				if (emoteId >= 0) {
					aiPlayer.doEmote(emoteId, 300);
				}
			}

			// Check for action commands at top level
			if (aiPlayer.connection != null && aiPlayer.connection.serverAi != null) {
				var ai = aiPlayer.connection.serverAi.ai;

				// followPlayer action
				if (Reflect.hasField(json, "followPlayer") && json.followPlayer == true) {
					ai.startFollowingPlayer(aiPlayer);
				}

				// drop action
				if (Reflect.hasField(json, "drop") && json.drop == true) {
					ai.doDropCommand();
				}

				// makeItem action
				if (Reflect.hasField(json, "makeItem") && json.makeItem != null) {
					ai.doMakeCraftCommand(Std.string(json.makeItem), true);
				}
			}
		} catch (e:Dynamic) {
			// Not valid JSON, use original response
		}
		return text;
	}

	/**
	 * Async version of respondToPlayer that calls the LLM in a separate thread.
	 * Executes the onSuccess callback with the AI response when complete.
	 * @param fromPlayer The AI player who will respond
	 * @param toPlayer The human player who sent the message
	 * @param message The message from the human player
	 * @param onSuccess Callback function executed on success with the AI response string
	 */
	public static function respondToPlayerAsync(fromPlayer:GlobalPlayerInstance, toPlayer:GlobalPlayerInstance, message:String, onSuccess:String->Void):Void {
		// Build the prompt in the main thread (player context must be accessed there)
		var fullPrompt = buildPrompt(fromPlayer, toPlayer, message);

		// Store references for use in thread
		var fromSoul = fromPlayer.playerSoul;

		// Spawn a new thread to call the LLM without blocking the main thread
		Thread.create(function() {
			// Call ChatResponse directly with the pre-built prompt
			var response = ChatResponse(fullPrompt);
			response = response.split("\n").join(" ");

			// Log the conversation to file (thread-safe)
			logToFile(fullPrompt, response);

			// Add to chat memory only if call succeeded
			if (response != null) {
				fromSoul.addChatEntry(toPlayer, message, response);
				// TODO toSoul.addChatEntry(toPlayer, message, response);
			}

			// Parse response to extract text and execute commands BEFORE sending
			var parsedText = parseAiResponse(response, fromPlayer);
			// Execute the callback with the response (replace newlines with spaces)
			sendResponseInChunks(fromPlayer, parsedText, onSuccess);
		});
	}

	private static function sendResponseInChunks(fromPlayer:GlobalPlayerInstance, response:String, onSuccess:String->Void):Void {
		if (response == null) {
			onSuccess(response);
			return;
		}

		var aiBase = fromPlayer.connection.serverAi.ai;
		var separators = [".", "!", "?", "*"];
		var maxLen = ServerSettings.MaxAIResponseperSay;

		// Check if splitting is needed
		var needsSplit = response.length > maxLen && containsAnySeparator(response, separators);

		if (!needsSplit) {
			var waitTime = (response.length / 100) * ServerSettings.AIWaitTimePer100Chars;
			if (aiBase != null) aiBase.setWaitingTimeMin(waitTime);
			onSuccess(response);
			return;
		}

		// Split and send chunks
		var chunks = splitResponse(response, maxLen, separators, 10);

		for (i in 0...chunks.length) {
			var chunk = chunks[i];
			var waitTime = (chunk.length / 100) * ServerSettings.AIWaitTimePer100Chars;

			logToFile('Chunk: ${i + 1}: ', chunk);

			if (aiBase != null) aiBase.setWaitingTimeMin(waitTime);
			onSuccess(chunk);
			if (i < chunks.length - 1) Sys.sleep(waitTime);
		}
	}

	private static function containsAnySeparator(text:String, separators:Array<String>):Bool {
		for (sep in separators) {
			if (text.indexOf(sep) >= 0) return true;
		}
		return false;
	}

	private static function splitResponse(text:String, maxLen:Int, separators:Array<String>, maxSplits:Int):Array<String> {
		var chunks = new Array<String>();
		var remaining = text;

		while (remaining.length > 0 && chunks.length < maxSplits) {
			if (remaining.length <= maxLen) {
				chunks.push(remaining);
				break;
			}

			// Find the best split point
			var splitIndex = -1;
			var bestSeparator = "";

			// First, try to find a separator before or at maxLen
			for (sep in separators) {
				var idx = remaining.lastIndexOf(sep, maxLen);
				if (idx > splitIndex) {
					splitIndex = idx;
					bestSeparator = sep;
				}
			}

			// If no separator found before maxLen, find first separator after maxLen
			if (splitIndex == -1) {
				var earliestAfter = remaining.length;
				for (sep in separators) {
					var idx = remaining.indexOf(sep, maxLen);
					if (idx >= 0 && idx < earliestAfter) {
						earliestAfter = idx;
						bestSeparator = sep;
					}
				}
				splitIndex = earliestAfter;
				if (splitIndex == remaining.length) splitIndex = -1;
			}

			if (splitIndex > 0) {
				chunks.push(remaining.substring(0, splitIndex + 1));
				remaining = remaining.substring(splitIndex + 1);
			}
			else {
				// No separator found at all, take maxLen characters
				chunks.push(remaining.substring(0, maxLen));
				remaining = remaining.substring(maxLen);
			}
		}

		return chunks;
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
