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

class Main
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
        screen.mouseEnabled = false;
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
        state.keyDown(e.keyCode);
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
    private function keyUp(e:KeyboardEvent)
    {
        state.keyUp(e.keyCode);
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
#end
//terminal application
class Main {
    public static var client:Client;
    public static var console:Console;
	static function main() 
	{
        trace("start terminal application");
	    client = new Client();
        console = new Console();
        new Game();
        #if (target.threaded)
        sys.thread.Thread.create(() -> {
        while (true) {
          trace("read " + Sys.stdin().readLine());
          Sys.sleep(0);
        }
        });
        #end
        var i:Int = 0;
        while (true)
        {
            trace("hi " + i++);
            Sys.sleep(1);
        }
	}
}
