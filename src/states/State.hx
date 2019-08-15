package states;

import openfl.events.MouseEvent;
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
        resize();
    }
    public function update()
    {
        
    }
    public function resize()
    {
        
    }
    public function mouseRightDown()
    {

    }
    public function mouseRightUp()
    {
        
    }
    public function mouseDown()
    {

    }
    public function mouseUp()
    {
        
    }
    public function mouseScroll(e:MouseEvent)
    {

    }
    public function keyDown()
    {
        
    }
    public function remove()
    {
        if (!sub) Main.client.message = null;
        Main.screen.removeChild(Main.state);
        Main.state = null;
    }
}