package openlife.auto;

class Action{
    public var name:String = "";
    public var maxPerTarget:Int = 99;
    public var maxPerAction:Int = 99;
    public var targetRange:Int = 1;
    public var reachedRange:Int = 1;

    //used to check whether the action can still be performed or not
    public function isValidAction():Bool{
        return true;
    }

    //used for travelling and error checking
    public function isValidTarget():Bool{
        return true;
    }
    //used for travelling and error checking
    public function isAddableTarget():Bool{
        return true;
    }

    //used to find a new target in case of failure
    public function newTarget():Bool{
        return true;
    }

    public function step(bot:Bot){
        //this is where the action gets run
    }

    public function work(bot:Bot){
        //gets called from step once we are at target
    }

    public function assign(role:Role){
        //Here we set the bot's action in memory. Bot.hx will have an action field.
    }

}