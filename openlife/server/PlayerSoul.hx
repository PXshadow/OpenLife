package openlife.server;

import sys.thread.Mutex;
import openlife.auto.PlayerInterface;
import openlife.server.Lineage.PrestigeClass;
import openlife.settings.ServerSettings;
import openlife.macros.Macro;

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

		// Prestige class with exact value
		var prestige = player.prestige;
		var prestigeClass = player.lineage.prestigeClass;
		var prestigeClassName = getPrestigeClassName(prestigeClass);
		text += "You are a " + prestigeClassName + " with prestige " + Math.round(prestige) + ". ";

		// Family
		text += getFamilyText();

		// Status
		text += getStatusText();

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
		// This would need to come from AI's assigned profession
		// For now, return empty - will be enhanced later
		return "";
	}
}
