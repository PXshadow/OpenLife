package openlife.data.transition;

import openlife.engine.Engine;
import openlife.data.transition.TransitionData;

class Transition
{
    /**
     * List of all of the transitions
     */
    var transitions:Array<TransitionData> = [];
    /**
     * Set all of the transitions
     */
    public function new()
    {
        /*for (name in FileSystem.readDirectory(Engine.dir + "transitions"))
        {
            transitions.push(new TransitionData(name,File.getContent(Engine.dir + "transitions/" + name)));
        }*/
    }
    public function make(id:Int):Array<Int>
    {
        //TODO: 
        return [0];
    }
}