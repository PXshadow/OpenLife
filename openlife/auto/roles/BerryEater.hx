package openlife.auto.roles;

import openlife.auto.Role;
import openlife.auto.actions.*;

class BerryEater extends openlife.auto.Role{
    public function new() {
        //find closest berry with map functions or overseer functions
        this.actions = [ new Travel(), new Take()];
    }
    override public function selectAction():String {
        super.selectAction(); 
        
        for(a in actions){
            if(a.isValidAction()){
                //a.assign(bot.role);
                //once the action is assigned it should get automatically called by role.run()
                //The overseer calls role.run for each bot and assigns roles.
            }
        }
        return "";
    }
}