package openlife.auto;
import openlife.engine.Engine;
class Action{
    public var name:String = "";
    public var maxPerTarget:Int = 99;
    public var maxPerAction:Int = 99;
    public var targetRange:Int = 1;
    public var reachedRange:Int = 1;
    public var eventOverride = function(){};
    public var eventOverrideBool:Bool = false;

    //Might want to add a field inside action to indicate when the action is completed

    //used to check whether the action can still be performed or not
    public function isValidAction():Bool{
        return true;
    }

    //used for travelling and error checking
    public function isValidTarget():Bool{
        return true;
    }

    //TODO: target finding code should go in here so that it can be reused across actions
    //used for travelling and error checking
    public function isAddableTarget():Bool{
        return true;
    }

    //used to find a new target in case of failure
    public function newTarget(bot:BotType):Bool{
        return true;
    }

    public function step(bot:BotType){
        //this is where the action gets run
        if(eventOverrideBool == true){
            eventOverride();
            return;
        }
    }

    public function work(bot:BotType){
        //gets called from step once we are at target
    }

    public function assign(bot:BotType){
        //Here we set the bot's action in memory. Bot.hx will have an action field.
        bot.currentAction = this;
    }

}