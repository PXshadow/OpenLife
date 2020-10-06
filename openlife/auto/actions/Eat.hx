package openlife.auto.actions;

import openlife.auto.Action;


class Eat extends openlife.auto.Action{
    var x:Int = 0;
    var y:Int = 0;
    var set:Bool = true;
    public function new() {
        this.name = 'Eat';
    }
    
    override public function step(bot:BotType){
        //call this.work();
        if(this.isValidTarget()){
            this.work(bot);
        }
    }

    override public function work(bot:BotType){
            bot.program.self();
            return;
    }

}