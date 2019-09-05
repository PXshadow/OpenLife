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
    var objects:Array<DrawObject> = [];
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
        for (object in objects)
        {
            //bottom right corner
            object.x = Main.objects.group.x + ((data.playerMap.get(object.id).x + object.dx) * Main.objects.scale);
            object.y = Main.objects.group.y + ((data.playerMap.get(object.id).y + object.dy) * Main.objects.scale) - object.height;  
            if (object.alive == 0)
            {
                removeChild(object);
                objects.remove(object);
                object = null;
            }else{
                object.alive--;
            }
        }
    }
    public function say(id:Int,data:String)
    {
        //parse and draw above player
        var object = new Message(id,data);
        addChild(object);
        objects.push(object);
    }
    public function username(id:Int,data:String) 
    {
        var object = new Nametag(id,data);
        addChild(object);
        for (object in objects)
        {
            if (object.id == id) return;
        }
        objects.push(object); 
    }
    private function path()
    {
        
    }
}
class DrawObject extends Sprite
{
    public var alive:Int = 60 * 7;
    public var id:Int = 0;
    var text:Text;
    public var dx:Float = 0;
    public var dy:Float = 0;
    public var type:Int = 0;
    public function new(id:Int=0)
    {
        super();
        this.id = id;
        mouseEnabled = false;
        tabEnabled = false;
        mouseChildren = false;
        cacheAsBitmap = true;
    }
}
class Nametag extends DrawObject
{
    public function new(id:Int,string:String)
    {
        super(id);
        alive = -1;
        text = new Text(string,CENTER,18);
        text.cacheAsBitmap = false;
        text.width = text.textWidth + 4;
        text.height = 22;
        addChild(text);
    }
}
class Message extends DrawObject
{
    public function new(id:Int,string:String)
    {
        super(id);
        type = 1;
        text = new Text(string,CENTER,18);
        text.width = text.textWidth + 4;
        text.height = 22;
        text.cacheAsBitmap = false;
        addChild(text);
        text.x = -text.width/2;
        graphics.beginFill(0xFFFFFF);
        graphics.drawRoundRect(text.x - 10,text.y,text.width + 10 * 2,22 + 4,25,25);
        dy = -Static.GRID * 1.3;
    }
}