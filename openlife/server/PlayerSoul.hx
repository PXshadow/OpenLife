package openlife.server;

import openlife.auto.PlayerInterface;
import openlife.server.Lineage.PrestigeClass;

/**
 * PlayerSoul provides context about a player for AI interactions.
 * Used to generate roleplay text for LLM-based AI responses.
 */
class PlayerSoul {
	public var player:GlobalPlayerInstance;

	public function new(player:GlobalPlayerInstance) {
		this.player = player;
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
