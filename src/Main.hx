import data.Pos;
import console.Console;
import data.animation.AnimationPlayer;
import game.Player;
import data.object.player.PlayerInstance;
import haxe.io.Path;
import data.object.player.PlayerMove;
#if openfl
import openfl.display.FPS;
import openfl.display.BlendMode;
import graphics.TgaData;
import openfl.display.Bitmap;
import openfl.display.BitmapData;
import openfl.display.PixelSnapping;
import game.GroundOverlay;
import game.Ui;
import openfl.events.MouseEvent;
import openfl.events.KeyboardEvent;
import openfl.ui.Keyboard;
import openfl.display.Shape;
import openfl.events.Event;
import openfl.display.Tile;
import lime.media.AudioSource;
import openfl.media.SoundChannel;
import openfl.media.Sound;
import haxe.ds.Vector;
import lime.app.Future;
import data.object.ObjectData;
import game.Game;
import game.Ground;
import game.Objects;
import data.map.MapInstance;
import ui.Text;
import ui.InputText;
import ui.Button;
import game.Weather;

class Main extends game.Game
{
    var objects:Objects;
    var grid:debug.Grid;
    var ui:Ui;
    var ground:Ground;
    var groundOverlay:GroundOverlay;
    var player:Player;
    var console:Console;
    var selectX:Int = 0;
    var selectY:Int = 0;
    var selects:Array<Tile> = [];
    var cursor:Bitmap;
    var gameBool:Bool = false;
    var weather:Weather;
    var fps:FPS;
    public function new()
    {
        //openfl.ui.Mouse.cursor = openfl.ui.MouseCursor.AUTO;
        directory();
        super();
        new resources.ObjectBake();
        cred();
        //login();
        game();
        //new editor.Inspector(objects,this);
        //new debug.ObjectSpriteViewer(1349,objects);
        connect();
        stage.addEventListener(Event.RESIZE,resize);
        stage.addEventListener(KeyboardEvent.KEY_DOWN,keyDown);
        stage.addEventListener(KeyboardEvent.KEY_UP,keyUp);
        stage.addEventListener(MouseEvent.MOUSE_DOWN,mouseDown);
        stage.addEventListener(MouseEvent.MOUSE_WHEEL,mouseWheel);
        stage.addEventListener(Event.ENTER_FRAME,update);
        stage.addEventListener(MouseEvent.MIDDLE_MOUSE_DOWN,mouseWheelDown);
        stage.addEventListener(MouseEvent.MIDDLE_MOUSE_UP,mouseWheelUp);
        stage.addEventListener(MouseEvent.MOUSE_OUT,mouseOut);
        //stage.addEventListener(MouseEvent.mouseou);
        //new data.sound.AiffData(File.getBytes(Game.dir + "sounds/1645.aiff"));
        //create console
        console = new Console();
        console.set("game",this);
        console.visible = false;
        addChild(console);
        /*grid = new debug.Grid();
        addChild(grid);*/
        resize(null);
        fps = new openfl.display.FPS(10,10,0xFFFFFF);
        addChild(fps);
    }
    var omx:Float;
    var omy:Float;
    var drag:Bool = false;
    private function mouseWheelDown(_)
    {
        omx = stage.mouseX;
        omy = stage.mouseY;
        drag = true;
    }
    private function mouseWheelUp(_)
    {
        drag = false;
    }
    private function mouseWheel(e:MouseEvent)
    {
        zoom(e.delta);
    }
    private inline function lerp(v0:Float,v1:Float,t:Float)
    {
        return v1 + t * (v1 - v0);
    }
    private function mouseDown(_)
    {
        if (cursor != null) cursor.visible = true;
        if (player != null)
        {
            /*if (player.ix == selectX && player.y == selectY)
            {
                trace("select player");
            }else{
                trace("non select player " + player.ix + " " + selectX);
            }*/
            for (obj in selects)
            {
                obj.alpha += -1;
            }
            selects = [];
            if (Math.abs(player.ix - selectX) <= 8 && Math.abs(player.iy - selectY) <= 8)
            {
                var list = Game.data.tileData.object.get(selectX,-selectY);
                var bool = false;
                if (list.length > 0)
                {
                    var mx = (mouseX - objects.group.x)/objects.scale;
                    var my = (mouseY - objects.group.y)/objects.scale;
                    for (obj in list)
                    {
                        if (obj.x - obj.originX > mx) continue;
                        if (obj.y - obj.originY > my) continue;
                        if (obj.x + obj.width/2 < mx) continue;
                        if (obj.y + obj.height/2 < my) continue;
                        bool = true;
                        break;
                    }
                }
                if (!bool)
                {
                    //non object select
                    trace("MOVE TO");
                    Game.program.move(player,selectX,selectY);
                }else{
                    selects = list;
                    for (obj in list)
                    {
                        obj.alpha += 1;
                    }
                }
            }
        }
    }
    var left:Bool = false;
    var right:Bool = false;
    var up:Bool = false;
    var down:Bool = false;
    private function keyDown(e:KeyboardEvent)
    {
        if (console != null) if (console.keyDown(e.keyCode)) return;
        switch (e.keyCode)
        {
            case Keyboard.I: zoom(1);
            case Keyboard.O: zoom(-1);
            case Keyboard.W: up = true;
            case Keyboard.S: down = true;
            case Keyboard.A: left = true;
            case Keyboard.D: right = true;
        }
    }
    private function keyUp(e:KeyboardEvent)
    {
        switch(e.keyCode)
        {
            case Keyboard.W: up = false;
            case Keyboard.S: down = false;
            case Keyboard.A: left = false;
            case Keyboard.D: right = false;
            case Keyboard.NUMBER_9:
            player.instance.age--;
            player.set(player.instance);
            case Keyboard.NUMBER_0:
            player.instance.age++;
            player.set(player.instance);
            case Keyboard.TAB:
            /*var bitmap = new Bitmap(objects.tileset.bitmapData);
            bitmap.width = stage.stageWidth;
            bitmap.height = stage.stageHeight;
            bitmap.alpha = 0.5;
            addChild(bitmap);*/
            //trace("percent " + ((objects.tileY + objects.tileHeight)/ objects.tileset.bitmapData.height));
        }
    }
    private function zoom(i:Int)
    {
        if (objects.scale > 2 && i > 0 || objects.scale < 0.3 && i < 0) return;
        var scale = i * 0.2;
        objects.scale += scale;
        objects.group.x += scale * objects.group.width;
        objects.group.y += scale * objects.group.height;
    }
    private function resize(_)
    {
        if (objects != null)
        {
            objects.width = stage.stageWidth;
            objects.height = stage.stageHeight;
            /*if (groundOverlay != null)
            {
                groundOverlay.width = objects.width;
                groundOverlay.height = objects.height;
            }*/
        }
        if (console != null) console.resize(stage.stageWidth);
    }
    private function game()
    {
        trace("create game");
        objects = new Objects();
        //weather = new Weather(objects);
        ground = new Ground();
        groundOverlay = new GroundOverlay(ground);
        addChild(ground);
        addChild(groundOverlay);
        addChild(objects);
        //weather.wind();
    }
    private function login()
    {
        var keyText = new Text("Key",LEFT,24,0xFFFFFF);
        keyText.y = 100;
        var emailText = new Text("Email",LEFT,24,0xFFFFFF);
        emailText.y = 50;
        addChild(keyText);
        addChild(emailText);
        
        var serverText = new Text("Address",LEFT,24,0xFFFFFF);
        var portText = new Text("Port",LEFT,24,0xFFFFFF);
        serverText.y = 150;
        portText.y = 200;
        addChild(serverText);
        addChild(portText);

        var keyInput = new InputText();
        keyInput.x = 100;
        keyInput.y = 100;
        addChild(keyInput);
        var emailInput = new InputText();
        emailInput.x = 100;
        emailInput.y = 50;
        addChild(emailInput);

        var serverInput = new InputText();
        serverInput.x = 100;
        serverInput.y = 150;
        addChild(serverInput);

        var portInput = new InputText();
        portInput.x = 100;
        portInput.y = 200;
        addChild(portInput);
        var join = new Button();
        join.text = "Join";
        join.y = 250;
        join.graphics.beginFill(0x808080);
        join.graphics.drawRect(0,0,60,30);
        join.Click = function(_)
        {
            if (emailInput.text.indexOf("@") == -1 || 
            emailInput.text.length < 5 || 
            keyInput.text.length < 4 || 
            serverInput.text.indexOf(".") == -1 ||
            serverInput.text.length < 4 ||
            Std.parseInt(portInput.text) == null
            ) return;
            //settings set
            settings.data.set("email",emailInput.text);
            settings.data.set("accountKey",keyInput.text);
            settings.data.set("customServerAddress",serverInput.text);
            settings.data.set("customServerPort",portInput.text);
            //client set
            client.ip = settings.data.get("customServerAddress");
            client.port = Std.parseInt(settings.data.get("customServerPort"));
            client.email = settings.data.get("email");
            client.key = settings.data.get("accountKey");
            //remove login
            removeChild(keyText);
            removeChild(emailText);
            removeChild(serverText);
            removeChild(portText);
            removeChild(keyInput);
            removeChild(emailInput);
            removeChild(serverInput);
            removeChild(portInput);
            removeChild(join);
            keyText = emailText = serverText = portText = null;
            keyInput = emailInput = serverInput = portInput = null;
            join = null;
            //start game
            game();
            connect();
        }
        addChild(join);
    }
    private function update(_) 
    {
        client.update();
        selectX = Math.floor((mouseX - ground.x + (Static.GRID/2) * objects.scale)/(Static.GRID * objects.scale));
        selectY = Math.floor((mouseY - ground.y + (Static.GRID/2) * objects.scale)/(Static.GRID * objects.scale));
        if (!gameBool) return;
        stage.window.title = fps.text;
        if (player.follow)
        {
            objects.group.x = lerp(objects.group.x,-player.x * objects.scale + objects.width/2 ,0.18);
            objects.group.y = lerp(objects.group.y,-player.y * objects.scale + objects.height/2,0.18);
        }
        //stage.window.title = 'x: $selectX y: $selectY';
        if (drag)
        {
            objects.group.x += stage.mouseX - omx;
            objects.group.y += stage.mouseY - omy;
            omx = stage.mouseX;
            omy = stage.mouseY;
            player.follow = false;
        }
        ground.x = objects.group.x;
        ground.y = objects.group.y;
        ground.scaleX = objects.group.scaleX;
        ground.scaleY = objects.group.scaleY;
        if (cursor != null)
        {
            cursor.x = ground.scaleX;
            cursor.y = ground.scaleY;
            cursor.x = mouseX;
            cursor.y = mouseY;
        }
        player.mx = 0;
        player.my = 0;
        if (up) player.my++;
        if (down) player.my--;
        if (left) player.mx--;
        if (right) player.mx++;
        player.step();
    }
    override function playerMoveStart(move:PlayerMove) 
    {
        super.playerMoveStart(move);
        var player = Game.data.playerMap.get(move.id);
        if (player == null || (player == this.player && !move.trunc)) return;
        player.move(move);
    }
    private function mouseOut(_)
    {
        //in and out cursor
        if (cursor != null) cursor.visible = !cursor.visible;
    }
    override function grave(x:Int, y:Int, id:Int) 
    {
        super.grave(x, y, id);
        trace('grave $x $y $id');
    }
    
    override function playerUpdate(instances:Array<PlayerInstance>) 
    {
        super.playerUpdate(instances);
        for (i in 0...instances.length)
        {
            objects.addPlayer(instances[i]);
            //animation.clear(objects.player.sprites());
        }
        if (player == null)
        {
            //main player
            player = objects.player;
            player.main = true;
            gameBool = true;
        }
    }
    override function mapChunk(instance:MapInstance) 
    {
        trace("map " + instance.toString() + "map total " + Game.data.map.toString());
        //remove everything not 32 tiles 
        if (Game.data.map != null && player != null)
        {
            var int:Int = 0;
            for (i in Game.data.map.x...Game.data.map.mx)
            {
                for (j in Game.data.map.y...Game.data.map.my)
                {
                    if (Math.abs(player.ix - i) > 36 || Math.abs(player.iy - j) > 36)
                    {
                        ground.remove(i,j);
                        objects.remove(i,j,true);
                        objects.remove(i,j);
                        int++;
                    }
                }
            }
            trace('cleaned $int tiles');
        }else{
            trace('can not clean because map data is null or player');
        }
        for (i in instance.x...instance.x + instance.width)
        {
            for (j in instance.y...instance.y + instance.height)
            {
                //remove overlap first
                /*ground.remove(i,j);
                objects.remove(i,j,true);
                objects.remove(i,j,true);*/
                //add new
                ground.add(Game.data.map.biome.get(i,j),i,j);
                objects.add([Game.data.map.floor.get(i,j)],i,j);
            }
        }
        for (j in instance.y...instance.y + instance.height)
        {
            for (i in instance.x...instance.x + instance.width)
            {
                objects.add(Game.data.map.object.get(i,j * -1),i,j * -1);
            }
        }
        var it = Game.data.playerMap.iterator();
        while (it.hasNext())
        {
            objects.player = it.next();
            objects.group.removeTile(objects.player);
            if (!objects.player.held) objects.group.addTile(objects.player);
        }
        //Game.data.map.chunks.push(instance);
        ground.render();
        //groundOverlay.render();
    }
}
#else
import ImportAll;
import game.Player;
//#if nativeGen @:nativeGen #end
class Main extends game.Game
{
    var player:Player;
    public function new()
    {
        directory();
        super();
        new resources.ObjectBake();
        cred();
        client.ip = "thinqbator.app";
        client.email = "set your email";
        client.key = "set your key";
        connect();
        while (true)
        {
            client.update();
            Sys.sleep(0.2);
            Sys.getChar(false);
        }
    }
    override function playerUpdate(instances:Array<PlayerInstance>) 
    {
        super.playerUpdate(instances);
        if (player == null)
        {
            player = new Player();
            player.set(instances[instances.length - 1]);
        }
    }
    public static function main()
    {
        new Main();
    }
}
#end