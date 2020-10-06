package openlife.auto.actions;

import openlife.auto.Action;


class Travel extends openlife.auto.Action{
    public function new() {
        this.name = 'Travel';
    }

    override public function step(bot:BotType){
        //check isValidAction and other validators if present
        if(this.isValidAction()){
            this.work(bot);
        }
        //then call this.work();
    }

    override public function work(bot:BotType){
        //move towards target
        bot.moveTo(bot.target.x, bot.target.y);
    }

}