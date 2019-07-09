/*package states.game;
import openfl.display.Shape;
import motion.Actuate;
import haxe.Timer;
import openfl.display.DisplayObjectContainer;
import ui.Text;
class Dialog extends DisplayObjectContainer
{
    var game:Game;
    public function new(game:Game)
    {
        this.game = game;
        super();
    }
    public function say(data:String)
    {
        //parse and draw above player
        addChild(new Bubble(0,"Hello"));
    }
    public function update()
    {
        if(numChildren > 0)
        {
            for(i in 0...numChildren)
            {
                cast(getChildAt(i),Bubble).update();
            }
        }
    }
}
class Bubble extends DisplayObjectContainer
{
    var id:Int = 0;
    var text:Text;
    var shape:Shape;
    public function new(id:Int,string:String)
    {
        super();
        this.id = id; 
        shape = new Shape();
        shape.graphics.beginFill(0xFFFFFF,0.5);
        shape.graphics.drawRoundRect(0,0,Static.GRID * 2,20,15,15);
        addChild(shape);
        text = new Text("",CENTER,18,10,Static.GRID * 2 - 20);
        addChild(text);
    }
    public function update()
    {
        //follow player
        if(id > 0)
        {

        }
    }
}*/