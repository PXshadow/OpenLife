import openfl.display.FPS;
import data.MapData.ArrayData;
import settings.Bind;
import client.Router;
import sys.thread.Thread;
import sys.net.Socket;
import client.Client;
import console.Console;
import states.game.Game;
import haxe.io.Path;
//visual client
#if openfl
import openfl.geom.Matrix;
import openfl.display.Shape;
import openfl.Assets;
import openfl.display.DisplayObjectContainer;
import openfl.net.SharedObject;
import openfl.display.Sprite;
import openfl.events.KeyboardEvent;
import openfl.events.MouseEvent;
import openfl.events.Event;
import settings.Bind;

class Main #if openfl extends Sprite #end
{
    //window
    public static inline var setWidth:Int = 1600;
    public static inline var setHeight:Int = 900;
    public static var scale:Float = 0;
    //client
    public static var client:Client;
    //over top console
    public static var console:Console;

    public static var state:states.State;
    public static var screen:DisplayObjectContainer;
    //so
    public static var so:SharedObject;
    public function new()
    {
        super();
        var j = 10/0;
        trace("j " + j);
        //setup discord
        var discord = new client.Discord();
        //stored appdata
        so = SharedObject.getLocal("client",null,true);
        //events
        addEventListener(Event.ENTER_FRAME,update);
        stage.addEventListener(Event.RESIZE,_resize);
        stage.addEventListener(KeyboardEvent.KEY_DOWN,keyDown);
        stage.addEventListener(KeyboardEvent.KEY_UP,keyUp);
        addEventListener(MouseEvent.MOUSE_WHEEL,mouseWheel);
        addEventListener(MouseEvent.MOUSE_DOWN,mouseDown);
        addEventListener(MouseEvent.MOUSE_UP,mouseUp);  
        addEventListener(MouseEvent.RIGHT_MOUSE_DOWN,mouseRightDown);
        addEventListener(MouseEvent.RIGHT_MOUSE_UP,mouseRightUp);
        //client
        client = new client.Client();
        //set state
        screen = new DisplayObjectContainer();
        //screen.mouseEnabled = false;
        //screen.mouseChildren = false;
        addChild(screen);
        state = new states.launcher.Launcher();
        //state = new states.game.Game();
        console = new console.Console();
        addChild(console);
        //debug
        #if debug
        var fps = new FPS();
        addChild(fps);
        #end
    }
    private function update(_)
    {
        if (client != null) client.update();
        if (state != null) state.update();
    }
    private function keyDown(e:KeyboardEvent)
    {
        if (console.keyDown(e.keyCode)) return;
        Bind.keys(e,true);
        if (state != null) state.keyDown();
    }
    private function keyUp(e:KeyboardEvent)
    {
        Bind.keys(e,false);
    }
    private function mouseDown(_)
    {
        if (state != null) state.mouseDown();
    }
    private function mouseUp(_)
    {
        if (state != null) state.mouseUp();
    }
    private function mouseRightDown(_)
    {
        if (state != null) state.mouseRightDown();
    }
    private function mouseRightUp(_)
    {
        if (state != null) state.mouseRightUp();
    }
    private function mouseWheel(e:MouseEvent)
    {
        if (state != null) state.mouseScroll(e);
    }
    private function _resize(_)
    {
        var tempX:Float = stage.stageWidth/setWidth;
		var tempY:Float = stage.stageHeight/setHeight;
		scale = Math.min(tempX, tempY);
		//set resize
		screen.x = Std.int((screen.stage.stageWidth - setWidth * scale) / 2); 
		screen.y = Std.int((screen.stage.stageHeight - setHeight * scale) / 2); 
	    screen.scaleX = scale; 
		screen.scaleY = scale;
        resize();
    }
    private function resize()
    {
        if (console != null)  console.resize(stage.stageWidth);
        if (state != null) state.resize();
    }
}
#else
//terminal application
class Main {
    public static var client:Client;
    public static var console:Console;
	static function main() 
	{
        var array = new ArrayData();
        array.set(-10,10,-2);
        trace("value " + array.get(-10,10));
        //create lists
        /*Static.dir = "OneLifeData7/";
        var food:String = "[\n";
        for (i in 0...Static.number())
        {
            var data = new data.ObjectData(i);
            trace("food value " + data.id + " " + data.foodValue);
            if (data.foodValue > 0)
            {
                food += "  " + data.id + ",//" + data.description + "\n";
            }
        }
        trace(food + "]");*/
        //input into output terminal
        var output = new Router(2000);
        output.bind();
        //trace
        haxe.Log.trace = function(v:Dynamic,?inf:haxe.PosInfos)
        {
            if(output.input != null) 
            {
                output.input.output.writeString(Std.string(v) + "\n");
                output.input.output.flush();
            }
        }
        //block untill accept
        output.input = output.socket.accept();
        output.socket.setBlocking(false);
        trace("output connected");
        client = new Client();
        //terminal application
        trace("start client terminal");
        console = new Console();
        var game = new Game();
        //async for input
        sys.thread.Thread.create(() -> {
            var input = Sys.stdin().readLine();
            console.run(input);
        });
        //game
        while(true)
        {
            client.update();
            Sys.sleep(1/30);
        }
	}
}
#end