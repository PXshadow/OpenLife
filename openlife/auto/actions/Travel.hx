package openlife.auto.actions;

import openlife.auto.Action;


class Travel extends openlife.auto.Action{
    public function new() {
        this.name = 'Travel';
    }

    override public function isValidAction():Bool{
        //check stuff like target validity, target distance or if path can be calculated.
        return false;
    }

    override public function step(bot:BotType){
        //check isValidAction and other validators if present
        //then call this.work();
    }

    override public function work(bot:BotType){
        //move towards target
    }

}