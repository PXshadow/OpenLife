package openlife.auto.roles;

import openlife.auto.Role;

class BerryEater extends openlife.auto.Role{
    //find closest berry with map functions or overseer functions
    this.actions = [ Travel, Take, Eat ];
    public function selectAction(bot:Bot):String{
        for(a in actions){
            if(a.isValidAction()){
                a.assignAction(bot);
                //once the action is assigned it should get automatically called by role.run()
                //The overseer calls role.run for each bot and assigns roles.
            }
        }
        return "";
    }
}