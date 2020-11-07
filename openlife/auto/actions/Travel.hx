package openlife.auto.actions;

import openlife.auto.Action;


class Travel extends openlife.auto.Action{
    public function new() {
        this.name = 'Travel';
    }

    //As long as this function returns true we continue the action
    //When this function returns false we call nextAction from the role
    override public function isValidAction():Bool{
        return true;
    }

    override public function step(bot:BotType){
        //check isValidAction and other validators if present
        if(this.isValidAction()){ //this isValidAction() check is subject to change
            this.work(bot);
        }
        //then call this.work();
    }

    override public function work(bot:BotType){
        //move towards target
        bot.moveTo(bot.target.x, bot.target.y);
    }

}