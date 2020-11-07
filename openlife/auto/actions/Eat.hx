package openlife.auto.actions;

import openlife.auto.Action;


class Eat extends openlife.auto.Action{
    public function new() {
        this.name = 'Eat';
    }

    ///As long as this function returns true we continue the action
    //When this function returns false we call nextAction from the role
    override public function isValidAction():Bool{
        return true;
    }
    
    override public function step(bot:BotType){
        //call this.work();
        if(this.isValidTarget()){ //this isValidTarget() check is subject to change
            this.work(bot);
        }
    }

    override public function work(bot:BotType){
            bot.program.self();
            return;
    }

}