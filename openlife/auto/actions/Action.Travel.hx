package openlife.auto.actions;

import openlife.auto.Action;


class Travel extends openlife.auto.Action{
    this.name = 'Travel';

    public function isValidAction(){
        //check stuff like target validity, target distance or if path can be calculated.
    }

    public function step(){
        //check isValidAction and other validators if present
        //then call this.work();
    }

    public function work(){
        //move towards target
    }

}