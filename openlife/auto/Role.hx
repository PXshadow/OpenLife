package openlife.auto;

import openlife.auto.actions.*;
import openlife.engine.Engine;

class Role{
    public var name:String;
    public var actions:Array<Action>;
    public var inflowActions:Array<Action>;
    public var outflowActions:Array<Action>;

    public function assignAction():Bool{
        //check isValidAction
        //check isValidTarget
        //set Bot's lastaction and currentaction
        return true;
    }

    //####MAGIC WITH THE ACTION ARRAYS
    public function selectInflowAction(){

    }
    public function selectOutflowAction(){

    }
    //Had some issues with this
    //selectAction should return an Action and so should nextAction but I couldn't get the individual actions to be reckognized by vcode, even with the import.
    public function selectAction():Action{
        return null;
    }
    public function nextAction():Action{
        return this.selectAction();
    }
    //###END MAGIC

    public function run(bot:BotType){
        //Check current action isValidAction
        //if action is valid then run action
        //otherwise next action
    }

    public function assign(bot:BotType){
        bot.role = this;
    }

}