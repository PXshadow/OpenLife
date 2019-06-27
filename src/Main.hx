import openfl.display.DisplayObjectContainer;
import lime.media.AudioBuffer;
import openfl.media.Sound;
import motion.Actuate;
import PlayerData.PlayerInstance;
import openfl.display.Shape;
import PlayerData.PlayerType;
import openfl.display.FPS;
import openfl.geom.Point;
import Display.Tile;
import openfl.events.MouseEvent;
import haxe.io.Path;
#if sys
import sys.io.File;
import sys.FileSystem;
#end
import openfl.display.Bitmap;
import openfl.ui.Keyboard;
import openfl.events.KeyboardEvent;
import openfl.events.Event;
import openfl.Lib;
import openfl.display.Sprite;
import openfl.display.BitmapData;

class Main extends Sprite
{
    public static inline var setWidth:Int = 1280;
    public static inline var setHeight:Int = 720;
    var scale:Float = 0;
    var menu:Bool = !true;
    public static var client:Client;
    //local
    var settings:Settings;
    //launcher
    var launcher:Launcher;
    //game
    public static var dialog:Dialog;
    public static var display:Display;
    var grid:Shape;
    var chat:Text;
    public static var console:Console;
    //debug
    var debugText:Text;
    var tileX:Int = 0;
    var tileY:Int = 0;

    public function new()
    {
        super();
        //sounds do not
        //var sound = Sound.fromAudioBuffer(AudioBuffer.fromBytes(File.getBytes("assets/hunger.aiff")));
        //music works
        //var sound = Sound.fromAudioBuffer(AudioBuffer.fromBytes(File.getBytes("assets/music_01.ogg")));
        //sound.play();
        //Lib.application.window.x = 1280 + 500;
        //Lib.application.window.borderless = true;

        client = new Client();
        client.map.update = updateMap;
        client.player.update = updatePlayer;

        settings = Settings.getLocal();

        if(menu) renderMenu();
        if (!menu) renderGame();

        //events
        addEventListener(Event.ENTER_FRAME,update);
        stage.addEventListener(Event.RESIZE,_resize);
        stage.addEventListener(KeyboardEvent.KEY_DOWN,keyDown);
        stage.addEventListener(KeyboardEvent.KEY_UP,keyUp);
        addEventListener(MouseEvent.MOUSE_WHEEL,mouseWheel);
        addEventListener(MouseEvent.MOUSE_DOWN,mouseDown);
        addEventListener(MouseEvent.MOUSE_UP,mouseUp);  

        //debug
        if (!menu) client.connect();
        //renderGame();
        /*client.map.setX = -18;
        client.map.setY = -13;
        client.map.setRect(client.map.setX,client.map.setY,32,30,File.read("assets/map.txt").readAll().toString());*/

        /*var p = new PlayerType();
        //fill test player
        p.p_id = 15;
        p.po_id = 19;
        p.age = 15;
        p.age_r = 60;
        p.move_speed = 30;
        display.addPlayer(p);*/
    }
    private function renderMenu()
    {
        menu = true;
        removeChildren();

        launcher = new Launcher();
        addChild(launcher);

        /*var connect = new Button();
		//var serverList = new ServerList();
		//addChild(serverList);
		//connect
		connect.addChild(new Text("Connect",CENTER,12,0,80));
		connect.graphics.beginFill(0xFFFFFF);
		connect.graphics.drawRoundRect(0,0,80,20,12,12);
		connect.y = 300 + 20;
		connect.Click = function(_)
		{
			//client.connect(serverList.ip,serverList.port);
            trace("connect");
            client.connect();
            renderGame();
		}
		addChild(connect);*/
    }
    private function renderGame()
    {
        menu = false;
        removeChildren();
        display = new Display();
        addChild(display);
        dialog = new Dialog();
        addChild(dialog);
        grid = new Shape();
        createGrid();
        grid.cacheAsBitmap = true;
        addChild(grid);
        var fps = new FPS(10,10,0xFFFFFF);
        addChild(fps);

        debugText = new Text("Debug",LEFT,12,0xFFFFFF,200);
        debugText.y = 100;
        addChild(debugText);

        chat = new Text("",LEFT,16,0,200);
        chat.text = "hello";
        chat.cacheAsBitmap = false;
        chat.border = true;
        chat.borderColor = 0;
        chat.selectable = true;
        chat.mouseEnabled = true;
        chat.tabEnabled = false;
        chat.type = INPUT;
        chat.height = 20;
        chat.restrict = "a-z A-Z , . ' - ? !  ";
        chat.tabEnabled = false;
        addChild(chat);

        console = new Console();
        addChild(console);
    }
    public function updatePlayer()
    {
        //return;
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
    }
    public function updateMap()
    {
        //return;
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
        }
    }
    private function createGrid()
    {
        return;
        grid.graphics.lineStyle(2,0xFFFFFF,0.5);
        for(j in 0...Std.int(setHeight/Static.GRID) + 2)
        {
            for(i in 0...Std.int(setWidth/Static.GRID) + 2)
            {
                grid.graphics.drawRect(i * Static.GRID,j * Static.GRID,Static.GRID,Static.GRID);
            }
        }
    }
    private function update(_)
    {
        if (console != null) console.update();
        if(!menu)
        {
            debugText.text = Std.string(Math.ceil(display.mouseX/Static.GRID)) + " " +
            Std.string(Math.ceil(display.mouseY/Static.GRID) + display.setY);
            client.update(); 
            var i = Player.active.iterator();
            while(i.hasNext())
            {
                i.next().update();
            }
            move();
        }
    }
    private function keyDown(e:KeyboardEvent)
    {
        if (menu) return;
        keys(e.keyCode,true);
        /*if(e.keyCode == Keyboard.BACKSPACE && e.shiftKey)
        {
            client.close();
        }*/
        if (e.keyCode == Keyboard.ESCAPE)
        {
            Sys.exit(0);
        }
        if (e.keyCode == Keyboard.T && stage.focus != chat)
        {
            trace("chat select");
            chat.setSelection(chat.length,chat.length);
        }
        @:privateAccess if (e.keyCode == Keyboard.UP && console.stage.focus == console)
        {
            console.previous();
        }
        if(e.keyCode == Keyboard.TAB)
        {
            console.visible = !console.visible;
            @:privateAccess console.input.selectable = false;
            if(console.visible)
            {
                @:privateAccess console.input.selectable = true;
                @:privateAccess console.input.setSelection(console.input.length,console.input.length);
            }
        }
        if(e.keyCode == Keyboard.ENTER)
        {
            //console
            @:privateAccess if(console.input.selectable)
            {
                console.enter();
            }
            //chat
            if(chat.selectable && chat.text.length > 0)
            {
                client.send("SAY 0 0 " + chat.text.toUpperCase());
                chat.selectable = false;
                chat.text = "";
            }
        }
    }
    private function mouseDown(_)
    {

    }
    private function mouseUp(_)
    {

    }
    private function mouseWheel(e:MouseEvent)
    {
        display.scaleX += e.delta * 0.1;
        display.scaleY += e.delta * 0.1;
    }
    public static function pointRect(pX:Float, pY:Float, rect:openfl.geom.Rectangle):Bool
	{
		//y
		if (pY < rect.y) return false;
		if (pY > rect.y + rect.height) return false;
		//x
		if (pX < rect.x) return false;
		if (pX > rect.x + rect.width) return false;
		return true;
	}
    private function keyUp(e:KeyboardEvent)
    {
        if (menu) return;
        keys(e.keyCode,false);
    }
    var cameraUp:Bool = false;
    var cameraDown:Bool = false;
    var cameraLeft:Bool = false;
    var cameraRight:Bool = false;
    var playerUp:Bool = false;
    var playerDown:Bool = false;
    var playerLeft:Bool = false;
    var playerRight:Bool = false;
    private function keys(code:Int,bool:Bool)
    {
        switch(code)
        {
            //player
            case Keyboard.W:
            playerUp = bool;
            case Keyboard.S:
            playerDown = bool;
            case Keyboard.A:
            playerLeft = bool;
            case Keyboard.D:
            playerRight = bool;
            //camera
            case Keyboard.UP:
            cameraUp = bool;
            case Keyboard.DOWN:
            cameraDown = bool;
            case Keyboard.LEFT:
            cameraLeft = bool;
            case Keyboard.RIGHT:
            cameraRight = bool;
        }
    }
    private function move()
    {
        if (stage.focus == chat || console.visible) return;

        var moveArray = [display,dialog];
        var speed:Int = 30;
        //camera movement
        if (cameraUp) for(obj in moveArray) obj.y += speed;
        if (cameraDown) for (obj in moveArray) obj.y += -speed;
        if (cameraLeft) for (obj in moveArray) obj.x += speed;
        if (cameraRight) for (obj in moveArray) obj.x += -speed;

        //player movement
        if (Player.main == null) return;
        var mX:Int = 0;
        var mY:Int = 0;
        if (playerUp) mY += 1;
        if (playerDown) mY += -1;
        if (playerLeft) mX += -1;
        if (playerRight) mX += 1;
        if (mX != 0 || mY != 0) Player.main.move(mX,mY);
    }
    private function _resize(_)
    {
        trace("width " + stage.stageWidth + " height " + stage.stageHeight);
        var tempX:Float = stage.stageWidth/setWidth;
		var tempY:Float = stage.stageHeight/setHeight;
		scale = Math.min(tempX, tempY);
		//set resize
		x = Std.int((stage.stageWidth - setWidth * scale) / 2); 
		y = Std.int((stage.stageHeight - setHeight * scale) / 2); 
		scaleX = scale; 
		scaleY = scale;
        resize();
    }
    private function resize()
    {
        if (chat != null) chat.y = setHeight - chat.height;
        if (console != null)
        {
            trace("console resize");
            console.x = -x * 1/scale;
            console.resize(stage.stageWidth/scale);
        }
    }

}