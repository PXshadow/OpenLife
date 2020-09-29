package openlife.auto.actions;

import openlife.auto.Action;


class Eat extends openlife.auto.Action{
    this.name = 'Use';

    public function isValidAction(){
        //check stuff like distance or if path can be calculated.
    }

    public function step(){
        //check isValidAction and other validators if present
        //then call this.work();
    }

    public function work(){
        //use object in hand
    }

}