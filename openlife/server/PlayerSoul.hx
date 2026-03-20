package openlife.server;

import openlife.auto.AiHelper;
import sys.thread.Mutex;
import openlife.auto.PlayerInterface;
import openlife.server.Lineage.PrestigeClass;
import openlife.settings.ServerSettings;
import openlife.macros.Macro;
import openlife.server.WorldMap;

/**
 * Types of interactions that can be tracked in player memory
 */
enum InteractionType {
	AttackDamage;
	ServedFood;
	ProvidedCloths;
	GivenCoins;
	ProvidedHealing; // Prepared for future use
	Trade; // Prepared for future use
}

/**
 * Data class to store interaction values with a specific player
 */
class InteractionData {
	public var playerId:Int;
	public var playerName:String;
	public var playerFamilyName:String;
	public var attackDamage:Float = 0;
	public var servedFood:Float = 0;
	public var providedCloths:Float = 0;
	public var givenCoins:Float = 0;
	public var providedHealing:Float = 0; // Prepared for future use
	public var tradeValue:Float = 0; // Prepared for future use

	public function new(playerId:Int, playerName:String, playerFamilyName:String) {
		this.playerId = playerId;
		this.playerName = playerName;
		this.playerFamilyName = playerFamilyName;
	}
}

/**
 * Data class to store chat history entries
 */
class ChatEntry {
	public var playerId:Int;
	public var playerName:String;
	public var playerFamilyName:String;
	public var message:String;
	public var reply:String;

	public function new(playerId:Int, playerName:String, playerFamilyName:String, message:String, reply:String) {
		this.playerId = playerId;
		this.playerName = playerName;
		this.playerFamilyName = playerFamilyName;
		this.message = message;
		this.reply = reply;
	}

	public function toString():String {
		return 'Name $playerName $playerFamilyName: $message Your reply: $reply';
	}
}

/**
 * PlayerSoul provides context about a player for AI interactions.
 * Used to generate roleplay text for LLM-based AI responses.
 */
class PlayerSoul {
	public var player:GlobalPlayerInstance;

	// Thread-safe memory for player interactions
	private var memory:Map<Int, InteractionData> = new Map<Int, InteractionData>();
	private var memoryOrder:Array<Int> = []; // Track order of entries for FIFO removal
	private var memoryMutex:Mutex = new Mutex();

	// Thread-safe chat memory
	private var chatMemory:Array<ChatEntry> = [];
	private var chatMemoryMutex:Mutex = new Mutex();

	public function new(player:GlobalPlayerInstance) {
		this.player = player;
	}

	/**
	 * Add or update an interaction with another player.
	 * Thread-safe method for async access.
	 */
	public function addInteraction(otherPlayerId:Int, otherPlayerName:String, otherPlayerFamilyName:String, type:InteractionType, value:Float):Void {
		memoryMutex.acquire();
		Macro.exception(addInteractionHelper(otherPlayerId, otherPlayerName, otherPlayerFamilyName, type, value));
		memoryMutex.release();
	}

	private function addInteractionHelper(otherPlayerId:Int, otherPlayerName:String, otherPlayerFamilyName:String, type:InteractionType, value:Float):Void {
		var interaction = memory.get(otherPlayerId);
		if (interaction == null) {
			// Create new entry
			interaction = new InteractionData(otherPlayerId, otherPlayerName, otherPlayerFamilyName);
			memory.set(otherPlayerId, interaction);
			memoryOrder.push(otherPlayerId);

			// Enforce memory limit (remove oldest if needed)
			if (memoryOrder.length > ServerSettings.AiMemoryMaxEntries) {
				var oldestId = memoryOrder.shift();
				memory.remove(oldestId);
			}
		}

		// Update the appropriate field based on type
		switch (type) {
			case InteractionType.AttackDamage:
				interaction.attackDamage += value;
			case InteractionType.ServedFood:
				interaction.servedFood += value;
			case InteractionType.ProvidedCloths:
				interaction.providedCloths += value;
			case InteractionType.GivenCoins:
				interaction.givenCoins += value;
			case InteractionType.ProvidedHealing:
				interaction.providedHealing += value;
			case InteractionType.Trade:
				interaction.tradeValue += value;
		}
	}

	/**
	 * Add a chat entry to the chat memory.
	 * Thread-safe method for async access.
	 * Only call this after successful LLM response.
	 */
	public function addChatEntry(fromPlayer:GlobalPlayerInstance, message:String, reply:String):Void {
		chatMemoryMutex.acquire();
		Macro.exception(addChatEntryHelper(fromPlayer, message, reply));
		chatMemoryMutex.release();
	}

	private function addChatEntryHelper(fromPlayer:GlobalPlayerInstance, message:String, reply:String):Void {
		chatMemory.push(new ChatEntry(fromPlayer.id, fromPlayer.name, fromPlayer.familyName, message, reply));

		// Enforce chat memory limit (remove oldest if needed)
		while (chatMemory.length > ServerSettings.AiChatMemoryMaxEntries) {
			chatMemory.shift();
		}
	}

	/**
	 * Get the memory text for AI context.
	 * Returns formatted string of all interactions.
	 */
	public function getMemoryText():String {
		memoryMutex.acquire();
		var result = "";
		Macro.exception(result = getMemoryTextHelper());
		memoryMutex.release();
		return result;
	}

	private function getMemoryTextHelper():String {
		if (memoryOrder.length == 0) return "";

		var result = "Recent interactions with other players: ";
		for (playerId in memoryOrder) {
			var interaction = memory.get(playerId);
			if (interaction == null) continue;

			var playerName = interaction.playerName + " " + interaction.playerFamilyName;
			var parts:Array<String> = [];
			if (interaction.attackDamage > 0) {
				parts.push('$playerName, AttackDamage += ${interaction.attackDamage}');
			}
			if (interaction.servedFood > 0) {
				parts.push('$playerName, ServedFood += ${interaction.servedFood}');
			}
			if (interaction.providedCloths > 0) {
				parts.push('$playerName, ProvidedCloths += ${interaction.providedCloths}');
			}
			if (interaction.givenCoins > 0) {
				parts.push('$playerName, GivenCoins += ${interaction.givenCoins}');
			}
			if (interaction.providedHealing > 0) {
				parts.push('$playerName, ProvidedHealing += ${interaction.providedHealing}');
			}
			if (interaction.tradeValue > 0) {
				parts.push('$playerName, Trade += ${interaction.tradeValue}');
			}

			if (parts.length > 0) {
				result += parts.join("; ") + " --- ";
			}
		}
		return result;
	}

	/**
	 * Get the chat memory text for AI context.
	 * Returns formatted string of recent chat history.
	 */
	public function getChatMemoryText():String {
		chatMemoryMutex.acquire();
		var result = "";
		Macro.exception(result = getChatMemoryTextHelper());
		chatMemoryMutex.release();
		return result;
	}

	// TODO only from player talking to
	private function getChatMemoryTextHelper():String {
		if (chatMemory.length == 0) return "";

		var result = "Recent chat history: ";
		for (entry in chatMemory) {
			result += entry.toString() + " --- ";
		}
		return result;
	}

	/**
	 * Returns text describing this player for their own context.
	 * Used when the AI needs to know who it is.
	 * Includes exact prestige value for comparison.
	 */
	public function getSoulText():String {
		var text = "You are " + player.name + " " + player.familyName + ", a ";
		text += player.isFemale() ? "female" : "male";
		text += " aged " + Math.floor(player.trueAge) + " years. ";
		if (player.trueAge < 6) text += "You are very young! Speak according to your age! ";
		// Current season
		text += 'It is currently ${TimeHelper.SeasonText}. ';

		// Prestige class with exact value
		var prestige = player.prestige;
		var prestigeClass = player.lineage.prestigeClass;
		var prestigeClassName = getPrestigeClassName(prestigeClass);
		text += "You are a " + prestigeClassName + " with prestige " + Math.round(prestige) + ". ";

		// Family
		text += getFamilyText();

		// Status
		text += getStatusText();

		// Temperature context
		text += getTemperatureContextText();

		// Home context
		text += getHomeContextText();

		// Profession (if any)
		var profession = getProfessionText();
		if (profession.length > 0) {
			text += "Your profession: " + profession + ". ";
		}

		// Held object
		if (player.heldObject != null) {
			text += "You are holding " + player.heldObject.objectData.name + ". ";
		}

		// Emotional state
		if (player.isAngryOrTerrified()) {
			text += "You are angry or terrified. ";
		}

		// Weapon
		if (player.isHoldingWeapon()) {
			text += "You are holding a weapon. ";
		}

		return text;
	}

	/**
	 * Returns text describing this player from an outsider's perspective.
	 * Used when another player interacts with this player.
	 */
	public function getExternalIntro():String {
		var text = "You are communicating with " + player.name + " " + player.familyName + ", a ";
		text += player.isFemale() ? "female" : "male";
		text += " aged " + Math.floor(player.trueAge) + " years. ";

		// Prestige class with exact value
		var prestige = player.prestige;
		var prestigeClass = player.lineage.prestigeClass;
		var prestigeClassName = getPrestigeClassName(prestigeClass);
		text += "They are a " + prestigeClassName + " with prestige " + Math.round(prestige) + ". ";

		// Family from outsider view
		text += getExternalFamilyText();

		// Status visible to others
		text += getExternalStatusText();

		// Profession (if any)
		var profession = getProfessionText();
		if (profession.length > 0) {
			if (player.isFemale()) text += "Her profession is " + profession + ". ";
			else text += "His profession is " + profession + ". ";
		}

		// Held object (visible)
		if (player.heldObject != null) {
			text += "They are holding " + player.heldObject.objectData.name + ". ";
		}

		// Emotional state (visible)
		if (player.isAngryOrTerrified()) {
			text += "They look angry or terrified. ";
		}

		// Weapon (visible)
		if (player.isHoldingWeapon()) {
			text += "They are holding a weapon. ";
		}

		return text;
	}

	private function getPrestigeClassName(prestigeClass:PrestigeClass):String {
		return switch (prestigeClass) {
			case PrestigeClass.NotSet: "commoner";
			case PrestigeClass.Serf: "serf";
			case PrestigeClass.Commoner: "commoner";
			case PrestigeClass.Noble: "noble";
			case PrestigeClass.King: "king";
			case PrestigeClass.Emperor: "emperor";
			default: "commoner";
		}
	}

	/**
	 * Get a descriptive temperature label based on heat value.
	 * 7 temperature levels: freezing, cold, cool, mild, warm, hot, sweltering
	 * @param heat The heat value (0.0 to 1.0)
	 * @return A descriptive temperature label
	 */
	public static function getTemperatureLabel(heat:Float):String {
		if (heat < 0.1) return "freezing";
		if (heat < 0.25) return "cold";
		if (heat < 0.4) return "cool";
		if (heat < 0.6) return "mild";
		if (heat < 0.75) return "warm";
		if (heat < 0.9) return "hot";
		return "sweltering";
	}

	/**
	 * Get temperature context text for AI.
	 * Includes current temperature with descriptive label.
	 * @return Text describing current temperature conditions
	 */
	private function getTemperatureContextText():String {
		var heat = player.heat;
		var tempLabel = getTemperatureLabel(heat);
		var text = "The temperature is " + tempLabel + ". ";

		// Add tile temperature at player position
		var tileTemperature = WorldMap.world.getTileTemperature(player.tx, player.ty);
		var tileTempLabel = getTemperatureLabel(tileTemperature);
		text += "The surrounding temperature is " + tileTempLabel + ". ";

		return text;
	}

	/**
	 * Get home context text describing distance and direction to player's home.
	 * Uses 1 tile = 1 mile conversion.
	 * @return Text describing distance and direction to home
	 */
	private function getHomeContextText():String {
		// Check if player has a home set
		if (player.home == null || (player.home.tx == 0 && player.home.ty == 0)) {
			return "No home. ";
		}

		// Calculate quad distance to home
		var quadDist = AiHelper.CalculateQuadDistanceToObject(player, player.home);
		var distance = Math.sqrt(quadDist);
		var miles = Math.round(distance);

		// Calculate direction
		var dx = player.home.tx - player.tx;
		var dy = player.home.ty - player.ty;

		// Determine direction with intermediate directions
		var direction = "";
		var useIntermediate = Math.abs(dx) < 2 * Math.abs(dy) || Math.abs(dy) < 2 * Math.abs(dx); // Only use intermediate if both are significant

		if (dx > 0 && dy > 0) {
			// East, South
			direction = useIntermediate ? "south east" : (Math.abs(dx) >= Math.abs(dy) ? "east" : "south");
		}
		else if (dx < 0 && dy > 0) {
			// West, South
			direction = useIntermediate ? "south west" : (Math.abs(dx) >= Math.abs(dy) ? "west" : "south");
		}
		else if (dx > 0 && dy < 0) {
			// East, North
			direction = useIntermediate ? "north east" : (Math.abs(dx) >= Math.abs(dy) ? "east" : "north");
		}
		else if (dx < 0 && dy < 0) {
			// West, North
			direction = useIntermediate ? "north west" : (Math.abs(dx) >= Math.abs(dy) ? "west" : "north");
		}
		else if (dx > 0) {
			direction = "east";
		}
		else if (dx < 0) {
			direction = "west";
		}
		else if (dy > 0) {
			direction = "south";
		}
		else if (dy < 0) {
			direction = "north";
		}

		if (miles < 20) {
			return "You are at your home. ";
		}

		var milesText = miles == 1 ? "mile" : "miles";
		if (direction.length > 0) {
			return "Your home is " + miles + " " + milesText + " to the " + direction + ". ";
		}
		else {
			return "Your home is " + miles + " " + milesText + " away. ";
		}
	}

	private function getFamilyText():String {
		var text = "";

		// Father
		if (player.father != null) {
			var father = player.father;
			text += "Your father is " + father.name + " " + father.familyName + ". ";
		}

		// Mother
		if (player.mother != null) {
			var mother = player.mother;
			text += "Your mother is " + mother.name + " " + mother.familyName + ". ";
		}

		// Children - need to find them through lineage
		// This would require searching, so for now we'll skip detailed children info

		return text;
	}

	private function getExternalFamilyText():String {
		var text = "";

		if (player.father != null) {
			var father = player.father;
			text += "Their father is " + father.name + " " + father.familyName + ". ";
		}

		// Mother
		if (player.mother != null) {
			var mother = player.mother;
			text += "Their mother is " + mother.name + " " + mother.familyName + ". ";
		}

		return text;
	}

	private function getStatusText():String {
		var text = "";

		// Starving status
		var foodPercent = (player.food_store / player.food_store_max) * 100;
		if (foodPercent < 20) {
			text += "You are starving! Food level: " + Math.floor(foodPercent) + "%. ";
		}
		else if (foodPercent < 50) {
			text += "You are hungry. Food level: " + Math.floor(foodPercent) + "%. ";
		}

		// Health
		if (player.isWounded()) {
			text += "You are wounded. ";
		}

		// Temperature
		if (player.isSuperHot()) {
			text += "You are very hot. ";
		}
		if (player.isSuperCold()) {
			text += "You are very cold. ";
		}

		return text;
	}

	private function getExternalStatusText():String {
		var text = "";

		// Starving status (visible)
		var foodPercent = (player.food_store / player.food_store_max) * 100;
		if (foodPercent < 20) {
			text += "They look starving! ";
		}
		else if (foodPercent < 50) {
			text += "They look hungry. ";
		}

		// Health (visible wounds)
		if (player.isWounded()) {
			text += "They are wounded. ";
		}

		// Temperature (visible)
		if (player.isSuperHot()) {
			text += "They look very hot. ";
		}
		if (player.isSuperCold()) {
			text += "They look very cold. ";
		}

		return text;
	}

	private function getProfessionText():String {
		// Use assignedProfession if available, otherwise fall back to lastProfession
		var text = player.assignedProfession;
		if (text == null || text == player.lastProfession) {
			text = player.lastProfession;
		}
		else {
			text += ' doing ' + player.lastProfession;
		}
		if (text == null) text = 'NONE';
		return text;
	}
}
