import PlayerData.PlayerType;
import openfl.display.FPS;
import openfl.geom.Point;
import Display.Tile;
import openfl.events.MouseEvent;
import haxe.io.Path;
import sys.FileSystem;
import openfl.display.Bitmap;
import openfl.ui.Keyboard;
import openfl.events.KeyboardEvent;
import openfl.events.Event;
import openfl.Lib;
import openfl.display.Sprite;

class Main extends Sprite
{
    public static inline var setWidth:Int = 1280;
    public static inline var setHeight:Int = 720;
    var scale:Float = 0;
    var menu:Bool = true;
    var client:Client;
    //local
    public var settings:Settings;
    //game
    public var display:Display;
    public var ui:Ui;
    //debug
    var objectList:Array<String> = [];
    var objectIndex:Int = 0;
    var focus:Tile = null;
    var focusOffset:Point;

    public function new()
    {
        super();
        //Lib.application.window.x = 1280 + 500;

        client = new Client();
        client.map.update = updateMap;
        client.player.update = updatePlayer;

        settings = Settings.getLocal();

        //debug
        objectList = FileSystem.readDirectory(Settings.assetPath + "objects");
        for(i in 0...objectList.length) objectList[i] = Path.withoutExtension(objectList[i]);
        objectList.sort(function(a:String,b:String)
        {
            if(Std.parseInt(a) > Std.parseInt(b))
            {
                return 1;
            }else{
                return -1;
            }
        });

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
        client.connect();
        renderGame();
    }
    private function renderMenu()
    {
        menu = true;
        removeChildren();
        var connect = new Button();
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
		addChild(connect);
    }
    private function renderGame()
    {
        menu = false;
        removeChildren();
        display = new Display();
        addChild(display);
        ui = new Ui();
        addChild(ui);
        var fps = new FPS(10,10,0xFFFFFF);
        addChild(fps);
        //var bitmap = new Bitmap(display.tileset.bitmapData);
        //addChild(bitmap);
    }
    public function updatePlayer()
    {
        var iterator = client.player.key.iterator();
        var player:PlayerType;
        while(iterator.hasNext())
        {
            player = iterator.next();
            display.addPlayer(player);
        }
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
        var cWidth:Int = 32;
        var cHeight:Int = 30;
        trace("map chunk add pos(" + cX + "," + cY +") size(" + cWidth + "," + cHeight + ")");
        var string = "0.0";
        for(y in cY...cY + cHeight)
        {
            for(x in cX...cX + cWidth)
            {
                string = x + "." + y;
                //trace("chunk");
                display.addChunk(client.map.biome.get(string),x,y);
                //trace("object");
                display.addObject(client.map.object.get(string),x,y);
                //trace("floor");
                display.addFloor(client.map.floor.get(string),x,y);
            }
        }
        //set pos of inital ground
        if(display.inital)
        {
            //display.x = (Main.setWidth - cWidth * Static.GRID)/2;
            //display.y = (Main.setHeight - cHeight * Static.GRID)/2;
            //trace("display x " + display.x + " y " + display.y);
            display.inital = false;
        }
        //set sizes and pos
        if (display.setX > cX) display.setX = cX;
        if (display.setY > cY) display.setY = cY;
        if (display.sizeX < cX + cWidth) display.sizeX = cX + cWidth;
        if (display.sizeY < cY + cHeight) display.sizeY = cY + cHeight;
    }
    private function update(_)
    {
        client.update(); 
        move();
        /*text.text = "";
        for(i in 0...ground.numTiles)
        {
            var tile = ground.getTileAt(i);
            text.appendText(i + " x: " + Std.string(tile.x - Static.GRID * 3) + " y: " + Std.string(tile.y - Static.GRID * 3) + "\n");
        }
        if(focus != null)
        {
            focus.x = mouseX - ground.x + focusOffset.x;
            focus.y = mouseY - ground.y + focusOffset.y;
        }*/
    }
    private function keyDown(e:KeyboardEvent)
    {
        if (menu) return;
        keys(e.keyCode,true);
        if(e.keyCode == Keyboard.Z)
        {
            if (objectIndex > 0) objectIndex --;
            objectViewer();
        }
        if(e.keyCode == Keyboard.X)
        {
            if (objectIndex < objectList.length) objectIndex ++;
            objectViewer();
        }
        if(e.keyCode == Keyboard.BACKSPACE)
        {
            client.close();
        }
    }
    private function objectViewer()
    {
        display.removeTiles();
        display.addObject(objectList[objectIndex],3,3);

    }
    private function mouseDown(_)
    {
        return;
        for(i in 0...display.numTiles)
        {
            focus = null;
            var tile = display.getTileAt(i);
            if(pointRect(mouseX,mouseY,new openfl.geom.Rectangle(display.x + tile.x,display.y + tile.y,tile.width,tile.height)))
            {
                focus = cast(tile,Tile);
                focusOffset = new Point(display.x + tile.x - mouseX,display.y + tile.y - mouseY);
            }
        }
    }
    private function mouseUp(_)
    {
        focus = null;
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
    var up:Bool = false;
    var down:Bool = false;
    var left:Bool = false;
    var right:Bool = false;
    private function keys(code:Int,bool:Bool)
    {
        switch(code)
        {
            case Keyboard.UP | Keyboard.W:
            up = bool;
            case Keyboard.DOWN | Keyboard.S:
            down = bool;
            case Keyboard.LEFT | Keyboard.D:
            left = bool;
            case Keyboard.RIGHT | Keyboard.A:
            right = bool;
        }
    }
    private function move()
    {
        var speed:Int = 20;
        //ground
        if(up) display.y += -speed;
        if (down) display.y += speed;
        if (left) display.x += -speed;
        if (right) display.x += speed;
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

    }

}