package openlife.auto.actions;

import openlife.auto.Action;

class Idle extends openlife.auto.Action {
	public function new() {
		this.name = 'Idle';
	}

	override public function step(bot:BotType) {
		// call this.work();
	}

	override public function work(bot:BotType) {
		// Do nothing
	}
}
