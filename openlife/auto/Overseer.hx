package openlife.auto;

class Overseer{
    public function new(){

    }
    public function run(bots:Array<Bot>){
        //Do all the calculations for resources and items and etc and then check which bots are available for which roles
        //Assign roles

        //OR we can have a foreach here and do some calculation/assignments in the same loop where we call bot.update 
        //To have a case by case kinda basis.
        //This would reduce processing at the cost of precision.


        //Run the bots
        //for (ai in ais) bot.role.run();
        for (ai in ais) bot.update();
    }
}