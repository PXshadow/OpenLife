package data.transition;

import sys.io.File;
import sys.FileSystem;
import data.transition.TransitionData;

class Transition
{
    var transitions:Array<TransitionData> = [];
    public function new()
    {
        for (name in FileSystem.readDirectory(Static.dir + "transitions"))
        {
            transitions.push(new TransitionData(name,File.getContent(Static.dir + "transitions/" + name)));
        }
    }
}