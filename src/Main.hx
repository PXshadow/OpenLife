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
    var menu:Bool = false;
    public static var client:Client;
    //local
    public var settings:Settings;
    //launcher
    public var launcher:Launcher;
    //game
    public var display:Display;
    public var grid:Shape;
    //debug
    var objectList:Array<String> = [];
    var objectIndex:Int = 0;
    var focus:Tile = null;
    var focusOffset:Point;
    //movement
    var moveX:Int = -1;
    var moveY:Int = -1;
    var moveActive:Bool = false;

    public function new()
    {
        super();
        //Lib.application.window.x = 1280 + 500;

        client = new Client();
        client.map.update = updateMap;
        client.player.update = updatePlayer;

        settings = Settings.getLocal();

        //debug
        /*objectList = FileSystem.readDirectory(Settings.assetPath + "objects");
        for(i in 0...objectList.length) objectList[i] = Path.withoutExtension(objectList[i]);
        objectList.sort(function(a:String,b:String)
        {
            if(Std.parseInt(a) > Std.parseInt(b))
            {
                return 1;
            }else{
                return -1;
            }
        });*/



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
        //client.connect();
        renderGame();
        client.map.setX = -18;
        client.map.setY = -13;
        client.map.setRect(client.map.setX,client.map.setY,32,30,File.read("assets/map.txt").readAll().toString());
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
        grid = new Shape();
        createGrid();
        grid.cacheAsBitmap = true;
        addChild(grid);
        var fps = new FPS(10,10,0xFFFFFF);
        //addChild(fps);
        //var bitmap = new Bitmap(display.tileset.bitmapData);
        //addChild(bitmap);
        //test tree
        /*var bitmap = new Bitmap(BitmapData.fromFile(Settings.assetPath + "kapuk_tree_work_in_progress.png"));
        bitmap.x = Static.GRID * 1;
        bitmap.y = Static.GRID * 1;
        addChild(bitmap);*/
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
                //trace("chunk");
                display.addChunk(client.map.biome.get(string),x,y);
                //trace("object");
                display.addObject(client.map.object.get(string),x,y);
                //trace("floor");
                display.addFloor(client.map.floor.get(string),x,y);
            }
        }
        if(display.inital)
        {
            display.x = display.setX * Static.GRID + setWidth/2;
            display.y = display.setY * Static.GRID + setHeight/2;
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
        client.update(); 
        var i = Player.active.iterator();
        while(i.hasNext())
        {
            i.next().update();
        }
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
        var pX:Int = Std.int(display.mouseX/Static.GRID) + display.setX;
        var pY:Int = Std.int(display.mouseY/Static.GRID) + display.setY;
        if(Player.main != null) Player.main.move(pX,pY);
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
        var moveArray = [display];
        //ground
        if(up) for(obj in moveArray) obj.y += speed;
        if (down) for(obj in moveArray) obj.y += -speed;
        if (left) for(obj in moveArray) obj.x += speed;
        if (right) for(obj in moveArray) obj.x += -speed;
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