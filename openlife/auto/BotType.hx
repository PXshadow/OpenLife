package openlife.auto;

import openlife.engine.Engine;

class BotType extends #if app Bot #else Engine #end
{
	public var currentAction:Action;
	public var lastAction:Action;
	public var role:Role;
	public var currentTarget:String;
	public var lastTarget:String;
}
