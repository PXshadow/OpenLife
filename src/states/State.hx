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
        Main.screen.addChild(this);
        addEventListener(Event.ENTER_FRAME,init);
    }
    public function init(_)
    {
        removeEventListener(Event.ENTER_FRAME,init);
    }
    public function update()
    {
        
    }
    public function keyDown(code:Int)
    {

    }
    public function keyUp(code:Int)
    {

    }
    public function resize()
    {
        
    }
    public function remove()
    {
        if (!sub) Main.client.message = null;
        Main.screen.removeChild(Main.state);
        Main.state = null;
    }
}