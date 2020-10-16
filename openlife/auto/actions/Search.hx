package openlife.auto.actions;

import openlife.data.Target;
import openlife.auto.Action;


class Search extends openlife.auto.Action{

    public function new() {
        this.name = 'Search';
    }

    //We are going to use newTarget() to check inside work() if
    //we need to keep going or not.
    //once a target has been discovered and added to the bot
    //we can return true from work and continue to the next action

    //target id should be type IDs, not unique IDs
    //check discovered map for target id
    //if target id not present continue spiral search
    override public function newTarget(bot:BotType):Bool{
        return false;
    }
    
    override public function step(bot:BotType){
        //call this.work();
    }

    //spiral search needs to use Bot.moveTo() or Bot.goTo() (whatever we name it)
    override public function work(bot:BotType){
        //Do nothing
    }

}