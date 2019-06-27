import motion.Actuate;
import haxe.Timer;
import openfl.display.DisplayObjectContainer;

class Dialog extends DisplayObjectContainer
{
    public function new()
    {
        super();
    }
    public function say(data:String)
    {
        //set global
        //x = Main.display.x;
        //y = Main.display.y;

        trace("dialog " + data);
        var slash = data.indexOf("/");
        var id = Std.parseInt(data.substring(0,slash));
        var curse = data.substring(slash + 1,slash + 2);
        var string = data.substring(slash + 2,data.length);
        trace("id " + id + " curse " + curse + " text " + string);
        var player = Player.active.get(id);
        if (player == null) return;
        var text = new Text(string,CENTER,18,0xFFFFFF,Static.GRID);
        var count:Int = 30;
        var timer = new Timer(100);
        timer.run = function()
        {
            if(count-- <= 0)
            {
                Actuate.tween(text,0.5,{alpha:0}).onComplete(function(_)
                {
                    removeChild(text);
                    text = null;
                });
            }else{
                text.x = (-Main.display.setX + player.tileX - 0.5) * Static.GRID;
                text.y = (-Main.display.setX - player.tileY - 0.5) * Static.GRID;
            }
        }
        text.x = (-Main.display.setX + player.tileX - 0.5) * Static.GRID;
        text.y = (-Main.display.setX - player.tileY - 0.5) * Static.GRID;
        addChild(text);
    }
}