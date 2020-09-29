package openlife.auto.actions;

import openlife.auto.Action;


class Take extends openlife.auto.Action{
    this.name = 'Take';

    public function isValidAction(){
        //check stuff like target validity, distance or if path can be calculated.
    }

    public function step(){
        //check isValidAction and other validators if present
        //then call this.work();
    }

    public function work(){
        //Take target object
    }

}