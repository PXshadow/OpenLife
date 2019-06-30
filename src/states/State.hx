package states;

import openfl.events.Event;
import openfl.ui.Keyboard;
import openfl.display.DisplayObjectContainer;

class State extends DisplayObjectContainer
{
    public function new()
    {
        super();
        addEventListener(Event.ENTER_FRAME,init);
    }
    private function init(_)
    {
        removeEventListener(Event.ENTER_FRAME,init);
        @:privateAccess Main.resize();
    }
    public function update()
    {
        
    }
    public function message(data:String,tag:client.MessageTag)
    {
        
    }
    public function remove()
    {

    }
}