package states.game;
import haxe.Timer;
import openfl.display.Sprite;
import openfl.display.DisplayObjectContainer;
import ui.Text;
import openfl.display.Shape;

class Draw extends Sprite
{
    var game:Game;
    var px:Float = 0;
    var py:Float = 0;
    var messages:Array<Message> = [];
    public function new(game:Game)
    {
        this.game = game;
        super();
    }
    public function update()
    {
        for (message in messages)
        {
            message.x = game.objects.group.x + game.data.playerMap.get(message.id).x;
            message.y = game.objects.group.y + game.data.playerMap.get(message.id).y;  
        }
        /*graphics.clear();
        if (game.program.setup) path();
        }*/
    }
    public function say(data:String)
    {
        //parse and draw above player
        var message = new Message(0,"Hello");
        messages.push(message);
    }
    private function path()
    {
        
    }
}
class Message extends DisplayObjectContainer
{
    public var id:Int = 0;
    var text:Text;
    var shape:Shape;
    public var alive:Int = 60 * 4;
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
}