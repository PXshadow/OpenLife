package openlife.auto.roles;

import haxe.iterators.StringIteratorUnicode;
import openlife.auto.Role;
import openlife.auto.actions.*;

class BerryEater extends openlife.auto.Role {
	public function new() {
		this.name = 'BerryEater';
		// find closest berry with map functions or overseer functions
		this.actions = [new Search(), new Travel(), new Take(), new Eat()];
	}

	override public function selectAction():Action {
		// still working on this
		// need to think about how to check the action validity and select the action
		// role needs to call action.newTarget()

		// super.selectAction();

		for (a in actions) {
			if (a.isValidAction()) {
				// a.assign(bot.role);
				// once the action is assigned it should get automatically called by role.run()
				// The overseer calls role.run for each bot and assigns roles.
			}
		}
		return null;
	}

	override public function run(bot:BotType) {
		// Check current action isValidAction
		// if action is valid then run action
		// otherwise next action

		// Kept this for possible overrides
		/*
			bot.event.says = function(id:Int,text:String,curse:Bool)
			{
				trace("text " + text);
				if (text.indexOf("BERRY") != -1)
				{
					
				}
			}
			bot.event.foodChange = function(store:Int, capacity:Int, ateId:Int, fillMax:Int, speed:Float, responsible:Int)
			{
				if (store > 4) return;
				
			}
		 */
	}
}
