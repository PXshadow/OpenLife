package ui;
import openfl.display.Sprite;
import openfl.events.Event;
import openfl.events.MouseEvent;
class Button extends Sprite
{
    //functions 
	public var Down:Dynamic->Void;
	public var Up:Dynamic->Void;
	public var Over:Dynamic->Void;
	public var Out:Dynamic->Void;
	public var outUp:Bool = true;
	public var Click:Dynamic->Void;
	public var textfield:Text;
	public var text(get, set):String;
	function get_text():String 
	{
		return textfield.text;
	}
	function set_text(text:String):String 
	{
		if (textfield == null)
		{
			textfield = new Text("",LEFT,24,0xFFFFFF,200);
			textfield.cacheAsBitmap = false;
			textfield.bold = true;
			textfield.invalidate();
			addChild(textfield);
		}
		textfield.text = text;
		return text;
	}
    public function new()
    {
        super();
        buttonMode = true;
		//cacheAsBitmap = true;
        addEventListener(Event.REMOVED_FROM_STAGE, removeFromStage);
	    addEventListener(Event.ADDED_TO_STAGE, addToStage);
    }
    public function addToStage(_)
	{
		if(Down != null)addEventListener(MouseEvent.MOUSE_DOWN,mouseDown);
		if(Up != null)addEventListener(MouseEvent.MOUSE_UP, mouseUp);
		if(Click != null)addEventListener(MouseEvent.CLICK, mouseClick);
		if(Over != null)addEventListener(MouseEvent.MOUSE_OVER, mouseOver);
		if (Out != null)
		{
			addEventListener(MouseEvent.MOUSE_OUT, mouseOut);
			if(outUp)addEventListener(MouseEvent.MOUSE_UP, mouseOut);
		}
		removeEventListener(Event.ADDED, addToStage);
	}
    public function removeFromStage(_)
    {
        if(Down != null)removeEventListener(MouseEvent.MOUSE_DOWN,mouseDown);
        if(Up != null)removeEventListener(MouseEvent.MOUSE_UP, mouseUp);
		if(Over != null)removeEventListener(MouseEvent.MOUSE_OVER, mouseOver);
		if (Out != null)
		{
			removeEventListener(MouseEvent.MOUSE_OUT, mouseOut);
			if(outUp)removeEventListener(MouseEvent.MOUSE_UP, mouseUp);
		}
		if(Click != null)removeEventListener(MouseEvent.CLICK, mouseClick);
		removeEventListener(Event.REMOVED_FROM_STAGE, removeFromStage);
    }
    private function mouseDown(e:MouseEvent)
	{
		if (Down != null) Down(e);
	}
	private function mouseUp(e:MouseEvent)
	{
		if (Up != null) Up(e);
	}
	private function mouseClick(e:MouseEvent)
	{
		if (Click != null) Click(e);
	}
	private function mouseOver(e:MouseEvent)
	{
		if (Over != null) Over(e);
	}
	private function mouseOut(e:MouseEvent)
	{
	   if (Out != null) Out(e);
	}
}