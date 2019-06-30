package states;

import openfl.events.Event;
import openfl.ui.Keyboard;
import openfl.display.DisplayObjectContainer;

class State extends DisplayObjectContainer
{
    var sub:Bool = false;
    public function new(sub:Bool=false)
    {
        this.sub = sub;
        super();
        if (!sub) addEventListener(Event.ENTER_FRAME,init);
    }
    private function init(_)
    {
        removeEventListener(Event.ENTER_FRAME,init);
        @:privateAccess Main.resize();
    }
    public function update()
    {
        
    }
    public function remove()
    {
        if (!sub) Main.client.message = null;
    }
}