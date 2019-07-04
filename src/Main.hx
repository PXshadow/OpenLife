import openfl.geom.Matrix;
import openfl.display.Shape;
import openfl.Assets;
import openfl.display.DisplayObjectContainer;
import openfl.net.SharedObject;
import openfl.display.Sprite;
import openfl.events.KeyboardEvent;
import openfl.events.MouseEvent;
import openfl.events.Event;
class Main extends Sprite
{
    //window
    public static inline var setWidth:Int = 1280;
    public static inline var setHeight:Int = 720;
    private static var scale:Float = 0;
    //client
    public static var client:client.Client;
    //over top console
    public static var console:console.Console;
    //state
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
    public function updatePlayer()
    {
        /*//return;
        for(player in client.player.array)
        {
            if(!display.updatePlayer(player))
            {
                display.addPlayer(player);
            }
        }
        //add primary player
        if(Main.client.player.primary == -1) 
        {
            Main.client.player.primary = client.player.array[client.player.array.length - 1].p_id;
            Player.main = Player.active.get(Main.client.player.primary);
            //set console
            Console.interp.variables.set("player",Player.main);
            //Player.main.alpha = 0.2;
        }
        client.player.array = [];
        */
    }
    public function updateMap()
    {
        /*return;
        if(display == null)
        {
            trace("no display");
            return;
        }
        var cX:Int = client.map.setX;
        var cY:Int = client.map.setY;
        var cWidth:Int = client.map.setWidth;
        var cHeight:Int = client.map.setHeight;
        //set sizes and pos
        if (display.setX > cX) display.setX = cX;
        if (display.setY > cY) display.setY = cY;
        if (display.sizeX < cX + cWidth) display.sizeX = cX + cWidth;
        if (display.sizeY < cY + cHeight) display.sizeY = cY + cHeight;
        //add to display
        trace("map chunk add pos(" + cX + "," + cY +") size(" + cWidth + "," + cHeight + ")");
        var string = "0.0";
        for(y in cY...cY + cHeight)
        {
            for(x in cX...cX + cWidth)
            {
                string = x + "." + y;
                //trace("string " + string);
                //trace("chunk " + client.map.biome.get(string));
                display.addChunk(client.map.biome.get(string),x,y);
                //trace("object");
                display.addObject(client.map.object.get(string),x,y);
                //trace("floor");
                display.addFloor(client.map.floor.get(string),x,y);
            }
        }
        //0 chunk display.addChunk(4,0,0);
        if(display.inital)
        {
            display.x = display.setX * Static.GRID + setWidth/2;
            display.y = display.setY * Static.GRID + setHeight/2;
            dialog.x = display.x;
            dialog.y = display.y;
            display.inital = false;
        }*/
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
        /*if (console != null) console.update();
        if(!menu)
        {
            debugText.text = stage.mouseX + "\n" + stage.mouseY + "\nnum " + Main.display.numTiles;
            client.update(); 
            var i = Player.active.iterator();
            while(i.hasNext())
            {
                i.next().update();
            }
            move();
        }*/
    }
    private function keyDown(e:KeyboardEvent)
    {
        console.keyDown(e.keyCode);
        state.keyDown(e.keyCode);
        /*if (menu) return;
        if(stage.focus == console.input)
        {
            consoleKeys(e.keyCode);
        }else{
            playerKeys(e.keyCode);
            moveKeys(e.keyCode,true);
        }
        //toggle no matter what
        if(e.keyCode == Keyboard.TAB)
        {
            console.visible = !console.visible;
            if(console.visible)
            {
                console.input.type = INPUT;
                stage.focus = console.input;
            }else{
                console.input.type = DYNAMIC;
                stage.focus = null;
            }
        }*/
    }
    public function playerKeys(code:Int)
    {
        /*switch(code)
        {
            case Keyboard.SPACE:
            if (Player.main.oid == 0)
            {
                Player.main.use();
            }else{
                Player.main.drop();
            }
        }*/
    }
    private function mouseDown(_)
    {

    }
    private function mouseUp(_)
    {

    }
    private function mouseWheel(e:MouseEvent)
    {
        //e.delta * 0.1;
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