package openlife.auto;

import openlife.auto.actions.*;
import openlife.engine.Engine;

class Role{
    public var resourceNeeded:Int;
    public var name:String;
    public var actions:Array<Action>;
    public var inflowActions:Array<Action>;
    public var outflowActions:Array<Action>;
    public var actionsChecked:Array<String>;

    //need to figure out how to assign actions to bot
    public function assignAction():Bool{
        //check isValidAction
        //check isValidTarget
        //set Bot's lastaction and currentaction
        return true;
    }

    //Think the action flow is mostly implemented
    //####MAGIC WITH THE ACTION ARRAYS
    public function selectInflowAction():Action{
        for(i in inflowActions){
            if(i.isValidAction() && !actionsChecked.contains(i.name)){
                actionsChecked.push(i.name);
                return i;
            }
        }
        //default return to get rid of compile errors
        return null;
    }
    public function selectOutflowAction():Action{
        for(i in outflowActions){
            if(i.isValidAction() && !actionsChecked.contains(i.name)){
                actionsChecked.push(i.name);
                return i;
            }
        }
        return null;
    }
    public function selectAction():Action{
        actionsChecked = new Array<String>();
        if(resourceNeeded==null && this.inflowActions.length>0){
            return this.selectInflowAction();
        } else if(this.outflowActions.length>0){
            return this.selectOutflowAction();
        } else {
            //default cycle through actions array
            for(i in actions){
                if(i.isValidAction() && !actionsChecked.contains(i.name)){
                    actionsChecked.push(i.name);
                    return i;
                }
            }
        }
        return null;
    }
    public function nextAction():Action{
        return this.selectAction();
    }
    //###END MAGIC

    public function run(bot:BotType){
        //Use select action
        //Check current action isValidAction
        //if action is valid then run action
        //otherwise next action
        bot.currentAction.step();
    }

    public function assign(bot:BotType){
        bot.role = this;
    }

}