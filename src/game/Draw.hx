package game;
import data.GameData;
import console.Program;
import haxe.Timer;
import openfl.display.Sprite;
import openfl.display.DisplayObjectContainer;
import ui.Text;
import openfl.display.Shape;

class Draw extends Sprite
{
    var px:Float = 0;
    var py:Float = 0;
    var messages:Array<Message> = [];
    var program:Program;
    var data:GameData;
    public function new(data:GameData,program:Program)
    {
        this.data = data;
        this.program = program;
        super();
    }
    public function update()
    {
        for (message in messages)
        {
            message.x = Main.objects.group.x + (data.playerMap.get(message.id).x * Main.objects.scale);
            message.y = Main.objects.group.y + ((data.playerMap.get(message.id).y - Static.GRID * 1.3) * Main.objects.scale) - message.height;  
            message.alive--;
            if (message.alive <= 0)
            {
                removeChild(message);
                messages.remove(message);
                message = null;
            }
        }
    }
    public function say(id:Int,data:String)
    {
        //parse and draw above player
        var message = new Message(id,data);
        addChild(message);
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
    public var alive:Int = 60 * 7;
    public function new(id:Int,string:String)
    {
        super();
        cacheAsBitmap = true;
        this.id = id; 
        text = new Text(string,CENTER,18,10);
        text.background = true;
        text.width = text.textWidth + 4;
        text.height = 22;
        text.cacheAsBitmap = false;
        addChild(text);
        text.x = -text.width/2;
    }
}