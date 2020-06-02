package data.transition;

import game.Game;
import sys.io.File;
import sys.FileSystem;
import data.transition.TransitionData;

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
        for (name in FileSystem.readDirectory(Game.dir + "transitions"))
        {
            transitions.push(new TransitionData(name,File.getContent(Game.dir + "transitions/" + name)));
        }
    }
    public function make(id:Int):Array<Int>
    {
        //TODO: 
        return [0];
    }
}