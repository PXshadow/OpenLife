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
#if (cpp && debug)
import cpp.vm.Profiler;
#else
import debug.Profiler;
#end
//visual client
#if openfl
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
#end

class Main #if openfl extends Sprite #end
{
    //client
    public static var client:Client;
    //over top console
    var console:Console;
    //settings
    public static var settings:Settings;
    //game
    #if openfl
    var draw:Draw;
    var food:Shape;
    var select:Shape;
    var chat:Text;
    var ground:Ground;
    //players
    public static var sounds:SoundPlayer = new SoundPlayer();
    public static var animations:AnimationPlayer = new AnimationPlayer();
    //use to grab group stage
    public static var objects:Objects;
    var state:DisplayObjectContainer;
    public var log:Text;
    #end
    public static var player:Player;
    var selectX:Int = 0;
    var selectY:Int = 0;
    public static var data:GameData;
    var playerInstance:PlayerInstance;
    var mapInstance:MapInstance;
    var index:Int = 0;
    var compress:Bool = false;
    var inital:Bool = true;
    var program:Program;
    var string:String = "";
    var gameBool:Bool = false;
    var lerpInt:Int = 2;
    var renderTime:Timer = null;
    var foodPercent:Float = 0;
    #if !openfl
    public static function main()
    {
        new Main();
    }
    #end
    public function new()
    {
        dir();
        #if openfl
        super();
        events();
        //state
        state = new DisplayObjectContainer();
        state.mouseChildren = false;
        state.mouseEnabled = false;
        addChild(state);
        #end
        //client
        console = new console.Console();
        program = new Program(console);
        console.set("program",program);
        console.set("data",data);
        //settings
        settings = new Settings();
        //data
        data = new GameData();
        //complete
        #if openfl
        console.set("data",data);
        game();
        //top layer
        /*var fps = new FPS();
        fps.textColor = 0;//0xFFFFFF;
        addChild(fps);*/
        log = new Text();
        log.color = 0xFFFFFF;
        log.y = 100;
        log.cacheAsBitmap = false;
        addChild(log);
        addChild(console);
        #else
        //terminal application

        //update loop main
        while (true)
        {
            client.update();
            Sys.sleep(1/20);
        }
        #end
        //trace("connect " + Main.client.ip + " email " + Main.client.login.email);
        var instance = new PlayerInstance([]);
        instance.po_id = 19;
        objects.addPlayer(instance);
        player = objects.player;
        Timer.delay(function()
        {
            player.instance.clothing_set = "2940;1376;2886;2886;2937;198";
            player.cloths();
        },1000);
        player.x = 300;
        player.y = 300;
        gameBool = true;
        resize(null);
        //connect();
    }
    public function dir()
    {
        #if (windows || !openfl)
        Static.dir = "";
        #else
        Static.dir = Path.normalize(lime.system.System.applicationDirectory);
        Static.dir = Path.removeTrailingSlashes(Static.dir) + "/";
        #end
        #if mac
        Static.dir = Static.dir.substring(0,Static.dir.indexOf("/Contents/Resources/"));
        Static.dir = Static.dir.substring(0,Static.dir.lastIndexOf("/") + 1);
        #end
        //check to see if location is valid
        if (exist(["groundTileCache","objects","sprites","animations","transitions"]))
        {
            trace("valid location");
        }else{
            #if openfl
            stage.window.alert("Place in OpenLife folder","directory not found");
            #else
            throw "directory not found";
            #end
            Sys.exit(0);
        }
    }
    public function exist(folders:Array<String>):Bool
    {
        for (folder in folders)
        {
            if (!FileSystem.exists(Static.dir + folder)) return false;
        }
        return true;
    }
    public function cred()
    {
        //account default
        //Main.client.login.email = "test@test.co.uk";
        //Main.client.login.key = "WC2TM-KZ2FP-LW5A5-LKGLP";
        Main.client.login.email = "test@test.com";
        Main.client.login.key = "9UYQ3-PQKCT-NGQXH-YB93E";
        //server default (thanks so much Kryptic <3)
        Main.client.ip = "game.krypticmedia.co.uk";
        Main.client.port = 8007;
        //Main.client.ip = "bigserver2.onehouronelife.com";
        //Main.client.port = 8005;

        //settings to use infomation
        if (!Main.settings.fail)
        {
            //account
            if (valid(Main.settings.data.get("email"))) Main.client.login.email = string;
            if (valid(Main.settings.data.get("accountKey"))) Main.client.login.key = string;
            if (valid(Main.settings.data.get("useCustomServer")) && string == "1")
            {
                if (valid(Main.settings.data.get("customServerAddress"))) Main.client.ip = string;
                if (valid(Main.settings.data.get("customServerPort"))) Main.client.port = Std.parseInt(string);
            }
            //window
            #if openfl
            if (valid(Main.settings.data.get("borderless"))) stage.window.borderless = string == "1" ? true : false;
            if (valid(Main.settings.data.get("fullscreen"))) stage.window.fullscreen = string == "1" ? true : false;
            if (valid(Main.settings.data.get("screenWidth"))) stage.window.width = Std.parseInt(string);
            if (valid(Main.settings.data.get("screenHeight"))) stage.window.height = Std.parseInt(string);
            if (valid(Main.settings.data.get("targetFrameRate"))) stage.frameRate = Std.parseInt(string);
            #end
        }
        //by pass settings and force email and key if secret account
        #if secret
        trace("set secret");
        Main.client.login.email = Secret.email;
        Main.client.login.key = Secret.key;
        Main.client.ip = Secret.ip;
        Main.client.port = Secret.port;
        #end
    }
    public function valid(obj:Dynamic):Bool
    {
        if (obj == null || obj == "") return false;
        string = cast obj;
        return true;
    }
    //events
    #if openfl
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
    public function game()
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
        draw = new Draw(program);
        state.addChild(draw);
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
    private function update(_)
    {
        if (client != null) client.update();
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
                    program.clean();
                    player.step(xs,ys);
                }
            }
            //update draw
            draw.update();
            if (player.follow)
            {
                //set camera to middle
                if (player.parent == objects.group)
                {
                    objects.group.x = Math.round(lerp(objects.group.x,-player.x * objects.scale + objects.width/2 ,0.20));
                    objects.group.y = Math.round(lerp(objects.group.y,-player.y * objects.scale + objects.height/2,0.20));
                }else{
                    //if (player.parent != null) try "player.parent null";
                    //trace("player parent " + player.parent + " Player " + player);
                    //objects.group.x = Math.round(lerp(objects.group.x,-player.parent.x * objects.scale + objects.width/2 ,0.20));
                    //objects.group.y = Math.round(lerp(objects.group.y,-player.parent.y * objects.scale + objects.height/2 ,0.20));
                    
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
            if (!FileSystem.exists(Static.dir + "screenShots")) FileSystem.createDirectory(Static.dir + "screenShots");
            for (file in FileSystem.readDirectory(Static.dir + "screenShots"))
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
            var file = sys.io.File.write(Static.dir + "screenShots/screen" + name,true);

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
            //program.task("berryfarm");
            trace("sharpstone");
            //program.task("sharpstone");
            animations.clear(player.sprites());
        }
        if (Bind.playerDrop.bool)
        {
            program.drop(selectX,selectY);
        }
        if (Bind.playerUse.bool)
        {
            program.use(selectX,selectY);
            player.hold();
        }
        if (Bind.playerKill.bool)
        {
            trace("play animation");
            animations.play(player.instance.po_id,2,player.sprites(),0,Static.tileHeight,player.clothing);
            //new data.AnimationPlayer(player.instance.po_id,2,player,player.sprites(),0,Static.tileHeight);
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
            program.goal = new Pos();
            program.goal.x = selectX;
            program.goal.y = selectY;
            @:privateAccess program.path(false);
        }else{
            if (Bind.playerKill.bool) 
            {
                trace("kill");
                program.kill(selectX,selectY);
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
        //remove out of area
        for (chunk in data.map.chunks)
        {
            if (Math.abs(chunk.x + chunk.width/2 - cx) > 22 || Math.abs(chunk.y + chunk.height/2 - cy) > 22)
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
                data.map.chunks.remove(chunk);
            }
        }
        //new
        for (j in mapInstance.y...mapInstance.y + mapInstance.height)
        {
            for (i in mapInstance.x...mapInstance.x + mapInstance.width)
            {
                ground.add(data.map.biome.get(i,j),i,j);
                objects.add(data.map.object.get(i,j),i,j);
                objects.add([data.map.floor.get(i,j)],i,j);
            }
        }
        data.map.chunks.push(mapInstance);
        ground.render();
    }
    private function clear()
    {
        //clear data
        data.map = new data.map.MapData();
        data.tileData = new data.display.TileData();
        data.blocking = new Map<String,Bool>();
        data.playerMap = new Map<Int,Player>();
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
        player.program = program;
        Main.player = player;
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
        if (objects.scale > 2 && i > 0 || objects.scale < 0.2 && i < 0) return;
        objects.scale += i * 0.08;
        lerpInt = 2;
    }
    #end
    private function end()
    {
        switch(Main.client.tag)
        {
            case PONG:
            client.ping = UnitTest.stamp();
            case PLAYER_UPDATE:
            #if !openfl
            //terminal
            if (player == null && playerInstance != null)
            {
                player = data.playerMap.get(playerInstance.p_id);
                console.set("player",player);
            }
            #else
            //sys.io.File.saveContent(Static.dir + "playerUpdate.txt",playerUpdateString);
            //visual client
            if (player == null && objects.player != null) 
            {
                setPlayer(objects.player);
                //player.sort();
                gameBool = true;
                resize(null);
            }
            objects.player = null;
            Main.client.tag = null;
            #end
            default:
        }
    }
    private function connect()
    {
        client = new Client();
        console.set("client",client);
        client.login = new client.Login();
        cred();
        client.login.accept = function()
        {
            trace("accept");
            //set message reader function to game
            client.message = message;
            client.end = end;
            client.login.accept = null;
            client.login = null;
            client.tag = null;
            index = 0;
        }
        client.login.reject = function()
        {
            trace("reject");
            client.login.reject = null;
            //Main.client.login = null;
        }
        client.message = client.login.message;
        client.connect();
    }
    private function analyze()
    {
        //check if valley data has not been loaded in yet
        if (data.map.valleySpacing == 0) return;
        var iy:Int = 0;
        var ix:Int = 0;
        var dy:Int = 0;
        var dx:Int = 0;
        if (data.map.valleyBool)
        {
            iy = mapInstance.y - data.map.valleyOffsetY;
            trace("valley spacing " + data.map.valleySpacing);
            dy = iy % data.map.valleySpacing;
            iy += dy;
            trace("valley y check " + iy);
            trace("valleyOffset " + data.map.valleyOffsetY);
        }
        if (data.map.offsetBoolX)
        {

        }
        if (data.map.offsetBoolY)
        {
            
        }
    }
    private function message(input:String) 
    {
        switch(Main.client.tag)
        {
            case COMPRESSED_MESSAGE:
            var array = input.split(" ");
            Main.client.compress = Std.parseInt(array[1]);
            Main.client.tag = null;
            case PLAYER_EMOT:
            var array = input.split(" ");
            //p_id emot_index ttl_sec
            //ttl_sec is optional, and specifies how long the emote should be shown
            //-1 is permanent, -2 is permanent but not new so should be skipped
            case PLAYER_UPDATE:
            playerInstance = new PlayerInstance(input.split(" "));
            //always force non main player
            if (player != null && playerInstance.p_id != player.instance.p_id) playerInstance.forced = true;
            #if openfl
            objects.addPlayer(playerInstance);
            #else
            var player = data.playerMap.get(playerInstance.p_id);
            if (player == null) player = new Player();
            player.set(playerInstance);
            data.playerMap.set(playerInstance.p_id,player);
            #end
            case PLAYER_MOVES_START:
            var playerMove = new PlayerMove(input.split(" "));
            if (player == null || playerMove.id == player.instance.p_id) return;
            if (data.playerMap.exists(playerMove.id))
            {
                playerMove.movePlayer(data.playerMap.get(playerMove.id));
            }
            Main.client.tag = null;
            case MAP_CHUNK:
            if(compress)
            {
                Main.client.tag = null;
                data.map.setRect(mapInstance,input);
                //center point to determine range
                var cx:Int = mapInstance.x + Std.int(mapInstance.width/2);
                var cy:Int = mapInstance.y + Std.int(mapInstance.height/2);
                if (player != null)
                {
                    cx = player.ix;
                    cy = player.iy;
                    //cx += player.instance.x;
                    //cy += player.instance.y;
                    //cx += -player.ix;
                    //cy += -player.iy;
                }
                #if openfl
                render(cx,cy);
                #end
                analyze();
                //mapInstance = null;
                //toggle to go back to istance for next chunk
                compress = false;
            }else{
                var array = input.split(" ");
                //trace("map chunk array " + array);
                for(value in array)
                {
                    switch(index++)
                    {
                        case 0:
                        mapInstance = new MapInstance();
                        mapInstance.width = Std.parseInt(value);
                        case 1:
                        mapInstance.height = Std.parseInt(value);
                        case 2:
                        mapInstance.x = Std.parseInt(value);
                        case 3:
                        mapInstance.y = Std.parseInt(value);
                        case 4:
                        mapInstance.rawSize = Std.parseInt(value);
                        case 5:
                        mapInstance.compressedSize = Std.parseInt(value);
                        //set min
                        
                        trace("map chunk " + mapInstance.toString());
                        index = 0;
                        //set compressed size wanted
                        Main.client.compress = mapInstance.compressedSize;
                        compress = true;
                    }
                }
            }
            case MAP_CHANGE:
            var change = new MapChange(input.split(" "));
            //floor
            if (change.floor > 0)
            {
                //no floor changes yet
                return;
            }
            //clear
            #if openfl
            objects.remove(change.x,change.y);
            #end
            //set in case of no object
            data.map.object.set(change.x,change.y,change.id);
            //trace("x " + change.x + " y " + change.y);
            //add
            if (change.id.length > 0 && change.id[0] > 0)
            {
                var move:Bool = change.speed > 0 ? true : false;
                #if openfl
                if (move)
                {
                    objects.remove(change.oldX,change.oldY);
                    var container = new TileContainer();
                    objects.add(change.id,change.x,change.y,container);
                    objects.group.addTile(container);
                    Main.data.tileData.object.set(change.x,change.y,[container]);
                    //move back to previous postition
                    container.x = change.oldX * Static.GRID;
                    container.y = (Static.tileHeight - change.oldY) * Static.GRID;
                    //tween
                    var time = Std.int(Static.GRID/(Static.GRID * change.speed * 1) * 60 * 1);
                    Actuate.tween(container,time/60,{x:change.x * Static.GRID,y:(Static.tileHeight - change.y) * Static.GRID}).ease(Quad.easeInOut);
                }else{
                    objects.add(change.id,change.x,change.y);
                }
                #end
            }
            Main.client.tag = null;
            index = 0;
            case HEAT_CHANGE:
            //heat food_time indoor_bonus
            //trace("heat " + input);
            Main.client.tag = null;
            index = 0;
            case FOOD_CHANGE:
            //trace("food change " + input);
            var array = input.split(" ");
            foodPercent = Std.parseInt(array[0])/Std.parseInt(array[1]);
            //trace("food% " + foodPercent);
            #if openfl
            food.graphics.clear();
            food.graphics.beginFill(0);
            food.graphics.drawRect(200,0,100,20);
            food.graphics.beginFill(0xFF0000);
            food.graphics.drawRect(200,0,foodPercent * 100,20);
            #end
            //also need to set new movement move_speed: is floating point playerSpeed in grid square widths per second.
            if (player != null) 
            {
                if (foodPercent <= 0.3 && program.taskName != "food")
                {
                    if (player.instance.age >= 3)
                    {
                        //program.task("food");
                    }else{
                        client.send("SAY 0 0 F");
                    }
                }
                player.instance.move_speed = Std.parseFloat(array[4]);
            }
            case FRAME:
            Main.client.tag = null;
            index = 0;
            case PLAYER_SAYS:
            trace("player say " + input);
            #if openfl
            var array = input.split("/");
            //trace("id " + array[0]);
            var text = array[1].substring(2,array[1].length);
            if (text.length > 0)
            {
                draw.say(Std.parseInt(array[0]),text);
            }
            #end
            case PLAYER_OUT_OF_RANGE:
            //player is out of range
            trace("player out of range " + input);
            var id:Int = Std.parseInt(input);
            var player = data.playerMap.get(id);
            #if openfl
            if (player != null) objects.group.removeTile(player);
            #end
            case LINEAGE:
            //p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id

            case NAME:
            //p_id first_name last_name last_name may be ommitted.
            var array = input.split(" ");
            var name:String = array[1] + (array.length > 1 ? " " + array[2] : "");
            #if openfl
            draw.username(Std.parseInt(array[0]),name);
            #end
            case HEALED:
            //p_id player healed no longer dying.

            case MONUMENT_CALL:
            //MN x y o_id monument call has happened at location x,y with the creation object id

            case GRAVE:
            //x y p_id
            var array = input.split(" ");
            var x:Int = Std.parseInt(array[0]);
            var y:Int = Std.parseInt(array[1]);
            var id:Int = Std.parseInt(array[2]);
            if (player == null || player.instance.p_id != id)
            {

            }else{
                //main player died disconnect
                #if openfl
                disconnect();
                #end
                Sys.sleep(0.6);
                connect();
            }
            case DYING:
            //p_id isSick isSick is optional 1 flag to indicate that player is sick (client shouldn't show blood UI overlay for sick players)
            trace("dying " + input);
            case GRAVE_MOVE:
            //xs ys xd yd swap_dest optional swap_dest parameter is 1, it means that some other grave at  destination is in mid-air.  If 0, not

            case GRAVE_OLD:
            //x y p_id po_id death_age underscored_name mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id
            //Provides info about an old grave that wasn't created during your lifetime.
            //underscored_name is name with spaces replaced by _ If player has no name, this will be ~ character instead.

            case OWNER_LIST:
            //x y p_id p_id p_id ... p_id

            case VALLEY_SPACING:
            //y_spacing y_offset Offset is from client's birth position (0,0) of first valley.
            var array = input.split(" ");
            data.map.valleySpacing = Std.parseInt(array[0]);
            data.map.valleyOffsetY = Std.parseInt(array[1]);
            trace("valley spacing " + data.map.valleySpacing + " offset " + data.map.valleyOffsetY);
            analyze();
            case FLIGHT_DEST:
            //p_id dest_x dest_y
            trace("FLIGHT FLIGHT FLIGHT " + input.split(" "));
            default:
        }
    }
}