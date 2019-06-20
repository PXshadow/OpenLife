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
        var text = new Text(string,LEFT,18,0,100);
        //text.x = player.x;
        //text.y = player.y;
        addChild(text);
    }
}