package openlife.server;

import haxe.Http;
import haxe.Exception;
import haxe.Json;
import openlife.settings.ServerSettings;

/**
 * AIProvider handles communication with the MiniMax LLM API.
 * Makes HTTP requests to the MiniMax chat completion endpoint.
 */
class AIProvider {
	public static function IsLLMActivated() {
		return (ServerSettings.AiApiKey != "Not Set");
	}

	/**
	 * Call the AI API with a prompt and get the response text.
	 * @param prompt The user prompt/message
	 * @param model Optional model override (defaults to AiDefaultModel)
	 * @return The AI response text, or throws exception on failure
	 */
	public static function callAi(prompt:String, ?model:String):String {
		var useModel = (model != null) ? model : ServerSettings.AiDefaultModel;

		// Check API key
		if (ServerSettings.AiApiKey == "Not Set") {
			throw new Exception("AI API key not configured. Set ServerSettings.AiApiKey");
		}

		// Build request body
		var requestBody:Dynamic = {
			model: useModel,
			max_tokens: ServerSettings.AiMaxTokensForChat,
			messages: [
				{
					role: "system",
					content: "This is a interactiv dialog! No thinking needed! Respond fast with one or two sentences and stay in your role!"
				},
				{role: "user", content: prompt}
			]
		};

		// content: "Respond super fast as possible, directly with the final answer only. Never show thinking, reasoning, or internal monologue. Just the answer."

		var jsonBody:String = Json.stringify(requestBody);

		// Make HTTP request
		var http = new Http('${ServerSettings.AiApiUrl}/v1/messages');
		http.setHeader("Content-Type", "application/json");
		http.setHeader("Authorization", "Bearer " + ServerSettings.AiApiKey);
		http.setHeader("x-api-key", ServerSettings.AiApiKey);
		http.setHeader("anthropic-version", "2023-06-01");

		// trace(ServerSettings.AiApiKey);

		var response:String = null;
		var error:Exception = null;

		http.onData = function(data:String) {
			response = data;
		};

		http.onError = function(msg:String) {
			error = new Exception("AI HTTP Error: " + msg);
		};

		// Set request body for POST
		http.setPostData(jsonBody);

		// Synchronous request (blocking)
		hl.Gc.blocking(true);
		try {
			http.request(true);
		} catch (e:Dynamic) {
			hl.Gc.blocking(false);
			throw e;
		}
		hl.Gc.blocking(false);
		// http.request(true);
		// Sys.sleep(120);

		// Check for HTTP error
		if (error != null) {
			throw error;
		}

		// Parse response
		if (response == null || response == "") {
			throw new Exception("AI empty response");
		}

		trace(response);

		return parseResponse(response);
	}

	/**
	 * Parse the JSON response and extract the message content.
	 * @param responseJson The raw JSON response string
	 * @return The content string from the response
	 */
	private static function parseResponse(responseJson:String):String {
		var response:Dynamic;
		try {
			response = Json.parse(responseJson);
		} catch (e:Exception) {
			throw new Exception("Failed to parse AI response: " + e.message);
		}

		/**
			{usage : {input_tokens : 56, output_tokens : 49}, stop_reason : end_turn, id : 05f8ffd7915b9d744e5232be56bfb968, role : assistant, model : MiniMax-M2.5-highspeed, type : message, content : [{thinking : The user is asking me to respond with a specific phrase "AI is working!" to confirm that I received their message. This is a simple test to verify I'm functioning properly. I should respond as requested., type : thinking, signature : c958ea53c885331e7dafc7731993546ec8d81ec94a4e9c1b43eb7fa306254060},{text : AI is working!, type : text}], base_resp : {status_code : 0, status_msg : }}
		 */

		// Check for error in response
		if (Reflect.hasField(response, "type") && response.type == "error") {
			var errorMsg = "AI API error";
			if (Reflect.hasField(response, "message")) {
				errorMsg = response.message;
			}
			throw new Exception(errorMsg);
		}

		// Extract content from response
		// MiniMax response format: { content: [{ type: "text", text: "..." }] }

		// Inside parseResponse, after checking it's not an error:

		if (Reflect.hasField(response, "content")) {
			var content:Array<Dynamic> = response.content;
			if (content != null && content.length > 0) {
				var collectedText = new StringBuf();
				var collectedThinking = new StringBuf();

				for (block in content) {
					if (block == null) continue;

					// Handle standard text blocks
					if (Reflect.hasField(block, "type") && block.type == "text") {
						if (Reflect.hasField(block, "text") && Std.isOfType(block.text, String)) {
							collectedText.add(block.text);
						}
					}
					// Optional: also extract from other known text-like fields
					else if (Reflect.hasField(block, "text") && Std.isOfType(block.text, String)) {
						collectedText.add(block.text);
					}
					// You could also log or ignore "thinking" blocks
					else if (block.type == "thinking" && Std.isOfType(block.thinking, String)) {
						collectedThinking.add(block.thinking);
					}
				}

				var finalText = collectedText.toString();
				if (finalText != "") {
					return StringTools.trim(finalText);
				}
			}
		}

		// Alternative format: { choices: [{ message: { content: "..." } }] }
		if (Reflect.hasField(response, "choices")) {
			var choices:Array<Dynamic> = response.choices;
			if (choices != null && choices.length > 0) {
				var firstChoice = choices[0];
				if (Reflect.hasField(firstChoice, "message")) {
					var message = firstChoice.message;
					if (Reflect.hasField(message, "content")) {
						return message.content;
					}
				}
			}
		}

		throw new Exception("AI response format not recognized");
	}
}
