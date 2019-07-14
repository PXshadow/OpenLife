import settings.Bind;
import client.Router;
import sys.thread.Thread;
import sys.net.Socket;
import client.Client;
import console.Console;
import states.game.Game;
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

class Main #if openfl extends Sprite #end
{
    //window
    public static inline var setWidth:Int = 1280;
    public static inline var setHeight:Int = 720;
    private static var scale:Float = 0;
    //client
    public static var client:Client;
    //over top console
    public static var console:Console;

    public static var state:states.State;
    public static var screen:DisplayObjectContainer;
    //so
    public static var so:SharedObject;
    //cursor
    private var cursor:Shape;
    public function new()
    {
        super();
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
        //client
        client = new client.Client();
        //set state
        screen = new DisplayObjectContainer();
        screen.mouseChildren = false;
        screen.mouseEnabled = false;
        addChild(screen);
        state = new states.launcher.Launcher();
        //state = new states.game.Game();
        console = new console.Console();
        addChild(console);

        cursor = new Shape();
        cursor.cacheAsBitmap = true;
        var mat = new Matrix();
        mat.createGradientBox(16,16);
        cursor.graphics.beginGradientFill(openfl.display.GradientType.RADIAL,[0xFFFFFF,0xFFFFFF],[1,0],[0,255],mat);
        cursor.graphics.drawCircle(8,8,8);
        addChild(cursor);
    }
    private function update(_)
    {
        //cursor
        if (cursor != null)
        {
            cursor.x = mouseX - 8;
            cursor.y = mouseY - 8;
        }
        if (client != null) client.update();
        if (state != null) state.update();
    }
    private function keyDown(e:KeyboardEvent)
    {
        if (console.keyDown(e.keyCode)) return;
        Bind.keys(e,true);
    }
    private function keyUp(e:KeyboardEvent)
    {
        Bind.keys(e,false);
    }
    private function mouseDown(_)
    {

    }
    private function mouseUp(_)
    {

    }
    private function mouseWheel(e:MouseEvent)
    {

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