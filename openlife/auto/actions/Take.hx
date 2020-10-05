package openlife.auto.actions;

import openlife.auto.Action;


class Take extends openlife.auto.Action{

    public function new() {
        this.name = 'Take';
    }

    override public function isValidAction():Bool{
        //check stuff like target validity, distance or if path can be calculated.
        return false;
    }

    override public function step(bot:BotType){
        //check isValidAction and other validators if present
        //then call this.work();
    }

    override public function work(bot:BotType){
        //Take target object
    }

}