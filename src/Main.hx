import lime.app.Future;
import client.Router;
import sys.FileSystem;
import sys.io.Process;
import haxe.Timer;
import console.Program;
import data.GameData;
import data.ArrayDataInt;
import settings.Bind;
import sys.net.Socket;
import client.Client;
import console.Console;
import haxe.io.Path;
import settings.Settings;
import data.object.ObjectData;
import data.object.player.PlayerInstance;
import data.map.MapInstance;
import game.Player;
import data.object.player.PlayerInstance;
import data.object.player.PlayerMove;
import data.map.MapChange;
//visual client
import openfl.display.Bitmap;
import openfl.display.BitmapData;
import openfl.ui.Keyboard;
import openfl.display.TileContainer;
import openfl.display.DisplayObject;
import ui.Text;
import motion.easing.Quad;
import motion.Actuate;
import openfl.display.FPS;
import lime.system.System;
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
import game.*;
import openfl.display.Tile;
import openfl.geom.Rectangle;
import data.animation.AnimationPlayer;
import data.sound.SoundPlayer;
import ui.*;

class Main extends Game
{
    var food:Shape;
    var select:Shape;
    var chat:Text;
    var ground:Ground;
    //login
    var emailText:Text;
    var emailInput:InputText;
    var keyText:Text;
    var keyInput:InputText;
    var serverText:Text;
    var serverInput:InputText;
    var portText:Text;
    var portInput:InputText;
    var join:Button;
    var console:Console;
    var player:Player;
    var program:Program;
    //players
    public static var sounds:SoundPlayer = new SoundPlayer();
    public static var animations:AnimationPlayer = new AnimationPlayer();
    //use to grab group stage
    public static var objects:Objects;
    var state:DisplayObjectContainer;
    public var log:Text;
    var selectX:Int = 0;
    var selectY:Int = 0;
    var gameBool:Bool = false;
    var lerpInt:Int = 2;
    var renderTime:Timer = null;
    var foodPercent:Float = 0;
    public function new()
    {
        if (!directory()) stage.window.alert("Place in OpenLife folder","directory not found");
        super();
        events();
        //state
        state = new DisplayObjectContainer();
        state.mouseChildren = false;
        state.mouseEnabled = false;
        addChild(state);
        //client
        console = new console.Console();
        program = new Program(console,client);
        console.set("program",program);
        console.set("data",Game.data);
        login();
        //top layer
        var fps = new FPS();
        fps.textColor = 0xFF0000;//0xFFFFFF;
        addChild(fps);
        log = new Text();
        log.color = 0xFFFFFF;
        log.y = 100;
        log.cacheAsBitmap = false;
        addChild(log);
        addChild(console);
        resize(null);
    }
    //events
    private function events()
    {
        //events
        addEventListener(Event.ENTER_FRAME,update);
        stage.addEventListener(Event.RESIZE,resize);
        stage.addEventListener(KeyboardEvent.KEY_DOWN,keyDown);
        stage.addEventListener(KeyboardEvent.KEY_UP,keyUp);
        stage.addEventListener(MouseEvent.MOUSE_WHEEL,mouseWheel);
        stage.addEventListener(MouseEvent.MOUSE_DOWN,mouseDown);
        stage.addEventListener(MouseEvent.MOUSE_UP,mouseUp);  
        stage.addEventListener(MouseEvent.RIGHT_MOUSE_DOWN,mouseRightDown);
        stage.addEventListener(MouseEvent.RIGHT_MOUSE_UP,mouseRightUp);
    }
    private function login()
    {
        keyText = new Text("Key",LEFT,24,0xFFFFFF);
        keyText.y = 50;
        emailText = new Text("Email",LEFT,24,0xFFFFFF);
        emailText.y = 100;
        addChild(keyText);
        addChild(emailText);
        
        serverText = new Text("Address",LEFT,24,0xFFFFFF);
        portText = new Text("Port",LEFT,24,0xFFFFFF);
        serverText.y = 150;
        portText.y = 200;
        addChild(serverText);
        addChild(portText);

        keyInput = new InputText();
        keyInput.x = 100;
        keyInput.y = 50;
        addChild(keyInput);
        emailInput = new InputText();
        emailInput.x = 100;
        emailInput.y = 100;
        addChild(emailInput);

        serverInput = new InputText();
        serverInput.x = 100;
        serverInput.y = 150;
        addChild(serverInput);

        portInput = new InputText();
        portInput.x = 100;
        portInput.y = 200;
        addChild(portInput);
        //fill
        if (!settings.fail)
        {
            emailInput.text = settings.data.get("email");
            keyInput.text = settings.data.get("accountKey");
            if (valid(settings.data.get("customServerAddress"))) serverInput.text = string;
            if (valid(settings.data.get("customServerPort"))) portInput.text = string;
        }
        join = new Button();
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
    private function game()
    {
        ground = new Ground();
        objects = new Objects();
        //tile selector
        select = new Shape();
        //change to matrix if it's to much of a hinderace for intensive scaling systems
        select.cacheAsBitmap = true;
        select.graphics.lineStyle(4,0xB7B7B7);
        select.graphics.drawRect(Static.GRID/2,Static.GRID/2,Static.GRID,Static.GRID);
        state.addChild(ground);
        state.addChild(select);
        state.addChild(objects);
        console.set("ground",ground);
        console.set("objects",objects);
        console.set("connect",connect);
        //draw display
        food = new Shape();
        food.cacheAsBitmap = true;
        state.addChild(food);
        chat = new Text("",LEFT,30,0,200);
        chat.tabEnabled = false;
        chat.wordWrap = false;
        chat.multiline = false;
        chat.background = true;
        chat.height = 34;
        chat.cacheAsBitmap = false;
        chat.selectable = true;
        chat.mouseEnabled = true;
        chat.type = INPUT;
        state.addChild(chat);
    }
    public static var xs:Int = 0;
    public static var ys:Int = 0;
    var it:Iterator<Player>;
    override public function update(_)
    {
        super.update(null);
        //player movement
        if(gameBool)
        {
            if (stage.focus != console.input && stage.focus != chat)
            {
                xs = 0;
                ys = 0;
                if (Bind.playerUp.bool) ys += 1;
                if (Bind.playerDown.bool) ys += -1;
                if (Bind.playerLeft.bool) xs += -1;
                if (Bind.playerRight.bool) xs += 1;
                if (xs != 0 || ys != 0) 
                {
                    if (player.step(xs,ys)) 
                    {
                        client.send("MOVE " + player.ix + " " + player.iy + " @" + player.lastMove + " " + player.moves[0].x + " " + player.moves[0].y);
                    }
                }
            }
            if (player != null && player.follow)
            {
                //set camera to middle
                if (player.parent == objects.group)
                {
                    objects.group.x = Math.round(lerp(objects.group.x,-player.x * objects.scale + objects.width/2 ,0.20));
                    objects.group.y = Math.round(lerp(objects.group.y,-player.y * objects.scale + objects.height/2,0.20));
                }else{
                    if (player.parent != null)
                    {
                        //trace("player parent " + player.parent + " Player " + player);
                        objects.group.x = Math.round(lerp(objects.group.x,-player.parent.x * objects.scale + objects.width/2 ,0.20));
                        objects.group.y = Math.round(lerp(objects.group.y,-player.parent.y * objects.scale + objects.height/2 ,0.20));
                    }
                    
                }
            }
            //set ground
            ground.x = objects.group.x;
            ground.y = objects.group.y;
            ground.scaleX = objects.group.scaleX;
            ground.scaleY = objects.group.scaleY;
            select.scaleX = ground.scaleX;
            select.scaleY = ground.scaleY;
            //mostly global
            selectX = Math.floor((mouseX - ground.x - (Static.GRID/2) * objects.scale)/(Static.GRID * objects.scale)) + 1;
            selectY = Math.floor((mouseY - ground.y - (Static.GRID/2) * objects.scale)/(Static.GRID * objects.scale)) + 1;
            //set local for render
            select.x = (selectX - 1) * (Static.GRID * objects.scale) + ground.x;
            select.y = (selectY - 1) * (Static.GRID * objects.scale) + ground.y;
            //set y real global
            selectY = Static.tileHeight - selectY;
            //log
            if (player != null)
            {
                //log.text = "num " + objects.group.numTiles;
            }
        }
    }
    private inline function lerp(v0:Float,v1:Float,t:Float)
    {
        if (lerpInt > 0)
        {
            lerpInt--;
            return v1;
        }
        return v1 + t * (v1 - v0);
    }
    private function keyDown(e:KeyboardEvent)
    {
        if (console.keyDown(e.keyCode)) return;
        if (e.keyCode == Keyboard.F12)
        {
            var latest:Int = -1;
            var int:Int = 0;
            if (!FileSystem.exists(Game.dir + "screenShots")) FileSystem.createDirectory(Game.dir + "screenShots");
            for (file in FileSystem.readDirectory(Game.dir + "screenShots"))
            {
                //screen 6
                int = Std.parseInt(Path.withoutExtension(file).substring(0,file.length));
                if (int > latest) latest = int;
            }
            latest++;
            //generate screen shot
            /*
            var bmd = new BitmapData(Std.int(stage.stageHeight),Std.int(stage.stageWidth));
            bmd.draw(stage);
            for (i in 0...5 - name.length) name = "0" + name;
            var file = sys.io.File.write(Game.dir + "screenShots/screen" + name,true);

            file.write(bmd.encode(bmd.rect,new openfl.display.JPEGEncoderOptions(80)));
            file.close();
            */
            return;
        }
        Bind.keys(e,true);
        //chat
        if (stage.focus == chat)
        {
            //focused chat
            if (Bind.chat.bool) 
            {
                if (chat.text == "")
                {
                    stage.focus = null;
                    return;
                }
                program.say(chat.text);
                chat.text = "";
                stage.focus = null;
            }
            return;
        }else{
            //non focused chat
            if (Bind.chat.bool)
            {
                stage.focus = chat;
                chat.setSelection(chat.length,chat.length);
                trace("chat length");
                return;
            }
        }
        //zoom
        if (Bind.zoomIn.bool) zoom(1);
        if (Bind.zoomOut.bool) zoom(-1);
        //player
        if (player == null) return;
        if (Bind.playerSelf.bool)
        {
            program.self();
        }
        if (Bind.search.bool)
        {
            animations.clear(player.sprites());
        }
    }
    private function keyUp(e:KeyboardEvent)
    {
        Bind.keys(e,false);
    }
    private function mouseDown(_)
    {
        //fix crash
        if (player == null) return;
        if (Bind.command)
        {
            mouseRightDown(null);
            return;
        }
        if (Bind.playerMove.bool)
        {
            //not yet
        }else{
            if (Bind.playerKill.bool) 
            {
                trace("kill");
                //program.remove(selectX,selectY,-1);
                //program.kill(selectX,selectY);
            }else{
                if (player.instance.age < 3 && player.held)
                {
                    program.jump();
                }
                //use action if within range
                program.use(selectX,selectY);
            }
        }
    }
    private function mouseUp(_)
    {
        
    }
    private function mouseRightDown(_)
    {
        if (player != null)
        {
            if (player.instance.o_id.length > 0)
            {
                program.drop(selectX,selectY);
            }else{
                program.remove(selectX,selectY);
            }
        }
    }
    private function mouseRightUp(_)
    {

    }
    private function mouseWheel(e:MouseEvent)
    {
        zoom(e.delta);
    }
    var offsetX:Int = 0;
    var offsetY:Int = 0;
    public function render(cx:Int,cy:Int) 
    {
        //clear
        for (chunk in Game.data.map.chunks)
        {
            if (Math.abs(chunk.x + chunk.width/2 - cx) >= 24 || Math.abs(chunk.y + chunk.height/2 - cy) >= 24)
            {
                    for (j in chunk.y...chunk.y + chunk.height)
                    {
                        for (i in chunk.x...chunk.x + chunk.width)
                        {
                            objects.remove(i,j);
                            objects.remove(i,j,true);
                            ground.remove(i,j);
                        }
                    }
                    Game.data.map.chunks.remove(chunk);
            }
        }
        //new
        //new Future(function()
        //{
        //objects.tileset.bitmapData.lock();
        for (j in mapInstance.y...mapInstance.y + mapInstance.height)
        {
            for (i in mapInstance.x...mapInstance.x + mapInstance.width)
            {
                ground.add(Game.data.map.biome.get(i,j),i,j);
                objects.add(Game.data.map.object.get(i,j),i,j);
                objects.add([Game.data.map.floor.get(i,j)],i,j);
            }
        }
        objects.tileset.bitmapData.unlock();
        ground.render();
        Game.data.map.chunks.push(mapInstance);
    }
    private function clear()
    {
        //clear data
        Game.data.clear();
        //data = new GameData();
        //console.set("data",data);
        objects.clear();
        player = null;
        gameBool = false;
    }
    private function disconnect()
    {
        client.close();
        clear();
    }
    private function setPlayer(player:Player)
    {
        trace("set main player");
        this.player.program = program;
        this.player = player;
        //player.sort();
        console.set("player",player);
        //center instantly
        lerpInt = 2;
        @:privateAccess client.send("PING 0 0 " + client.pingInt);
    }
    private function resize(_)
    {
        if (console != null)  console.resize(stage.stageWidth);
        if (gameBool)
        {
            objects.width = stage.stageWidth;
            objects.height = stage.stageHeight;
            chat.y = stage.stageHeight - 30;
            food.y = stage.stageHeight - 30;
        }
    }
    private function zoom(i:Int)
    {
        if (!gameBool) return;
        if (objects.scale > 2 && i > 0 || objects.scale < 0.2 && i < 0) return;
        objects.scale += i * 0.08;
        lerpInt = 2;
    }
    override private function end()
    {
        super.end();
        switch(client.tag)
        {
            case PLAYER_UPDATE:
            if (player == null && objects.player != null) 
            {
                setPlayer(objects.player);
                //player.sort();
                gameBool = true;
                resize(null);
            }
            objects.player = null;
            client.tag = null;
            default:
        }
    }
    
}