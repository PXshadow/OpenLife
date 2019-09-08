import lime.system.System;
import sys.FileSystem;
import sys.io.Process;
import ui.Text;
import haxe.Timer;
import motion.easing.Quad;
import motion.Actuate;
import openfl.display.TileContainer;
import openfl.display.DisplayObject;
import console.Program;
import data.GameData;
import openfl.display.FPS;
import data.MapData.ArrayDataArray;
import settings.Bind;
import sys.thread.Thread;
import sys.net.Socket;
import client.Client;
import console.Console;
import haxe.io.Path;
//visual client
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
import settings.Settings;
import game.*;
import data.PlayerData.PlayerInstance;
import data.MapData.MapInstance;
import openfl.display.Tile;
import data.PlayerData.PlayerMove;
import data.MapData.MapChange;
import openfl.geom.Rectangle;

class Main #if openfl extends Sprite #end
{
    //client
    public static var client:Client;
    //over top console
    var console:Console;
    //local data
    public static var so:SharedObject;
    //settings 
    public static var settings:Settings;
    //game
    var draw:Draw;
    var food:Shape;
    public static var objects:Objects;
    var chat:Text;
    var ground:Ground;
    var select:Shape;
    var selectX:Int = 0;
    var selectY:Int = 0;
    var data:GameData;
    var playerInstance:PlayerInstance;
    var mapInstance:MapInstance;
    var index:Int = 0;
    var compress:Bool = false;
    var inital:Bool = true;
    var program:Program;
    var string:String = "";
    var gameBool:Bool = false;
    var state:DisplayObjectContainer;
    public static var player:Player;

    public function new()
    {
        super();
        //stored appdata
        so = SharedObject.getLocal("client",null,true);
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
        //state
        state = new DisplayObjectContainer();
        state.mouseChildren = false;
        state.mouseEnabled = false;
        addChild(state);
        //client
        client = new client.Client();
        console = new console.Console();
        addChild(console);
        //launch
        dir();
        cred();
        game();
        connect();
        var fps = new FPS();
        addChild(fps);
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
        data = new GameData();
        ground.data = data;
        objects.data = data;
        program = new Program(data,console);
        console.set("program",program);
        console.set("ground",ground);
        console.set("objects",objects);
        //draw display
        draw = new Draw(data,program);
        state.addChild(draw);
        food = new Shape();
        food.cacheAsBitmap = true;
        addChild(food);
        chat = new Text("",LEFT,30,0,200);
        chat.wordWrap = false;
        chat.multiline = false;
        chat.background = true;
        chat.height = 34;
        chat.cacheAsBitmap = false;
        chat.selectable = true;
        chat.mouseEnabled = true;
        chat.type = INPUT;
        addChild(chat);
    }
    public function dir()
    {
        #if windows
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
        if (exist(["groundTileCache","objects","sprites","animations"]))//,"settings"]))
        {
            trace("valid location");
        }else{
            stage.window.alert("Place OpenLife in the OneLife folder","OneLife directory not found");
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

        //settings to grab infomation
        Main.settings = new settings.Settings();
        if (!Main.settings.fail)
        {
            //account
            if (valid(Main.settings.data.email)) Main.client.login.email = string;
            if (valid(Main.settings.data.accountKey)) Main.client.login.key = string;
            if (valid(Main.settings.data.useCustomServer) && string == "1")
            {
                if (valid(Main.settings.data.customServerAddress)) Main.client.ip = string;
                if (valid(Main.settings.data.customServerPort)) Main.client.port = Std.parseInt(string);
            }
            //window
            if (valid(Main.settings.data.borderless)) stage.window.borderless = Std.parseInt(string) == 1 ? true : false;
            if (valid(Main.settings.data.fullscreen)) stage.window.fullscreen = Std.parseInt(string) == 1 ? true : false;
            if (valid(Main.settings.data.screenWidth)) stage.window.width = Std.parseInt(string);
            if (valid(Main.settings.data.screenHeight)) stage.window.height = Std.parseInt(string);
            if (valid(Main.settings.data.targetFrameRate)) stage.frameRate = Std.parseInt(string);
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
    var xs:Int = 0;
    var ys:Int = 0;
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
                    player.goal = false;
                    program.setup = false;
                    player.move(xs,ys);
                }
            }
            //update draw
            draw.update();
            if (player.follow)
            {
                //set camera to middle
                objects.group.x = Math.round(lerp(objects.group.x,-player.x * objects.scale + objects.width/2 ,0.05));
                objects.group.y = Math.round(lerp(objects.group.y,-player.y * objects.scale + objects.height/2,0.05));
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
        }
    }
    private inline function lerp(v0:Float,v1:Float,t:Float)
    {
        return v0 + t * (v1 - v0);
    }
    private function keyDown(e:KeyboardEvent)
    {
        if (console.keyDown(e.keyCode)) return;
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
        if (Bind.playerSelf.bool)
        {
            trace("self");
            program.self();
        }
        if (Bind.playerDrop.bool)
        {
            program.drop(selectX,selectY);
        }
        if (Bind.playerUse.bool)
        {
            program.use(selectX,selectY);
            player.hold();
            //animation section
            /*var tile:Tile = null;
            var animation = objects.objectMap.get(player.instance.po_id).animation;
            trace(" map " + animation);
            if (animation != null)
            {
                var i:Int = 0;
                for (param in animation.record[2].params)
                {
                    if (player.sprites[i] != null)
                    {
                        player.sprites[i].x = param.offset.x;
                        player.sprites[i].y = param.offset.y;
                        i++;
                    }
                }
            }*/
            //var record = map.animation.record;
            /*for (i in 0...record.params.length)
            {
                var sprite = player.sprites[i];
                trace("sprite " + sprite);
            }*/
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
            program.path(selectX,selectY);
        }else{
            if (Bind.playerKill.bool) 
            {
                trace("kill");
                program.kill(selectX,selectY);
            }else{
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
            if (player.instance.o_id > 0)
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
    var timer:Timer = null;
    public function render() 
    {
        trace("MAP UPDATE");
        //inital set camera
        var delay:Int = 400;
        if (inital)
        {
            objects.group.x = -data.map.x * Static.GRID;
            objects.group.y = -data.map.y * Static.GRID;
            ground.x = objects.group.x;
            ground.y = objects.group.y;
            inital = false;
            delay = 0;
            //clear player
            //objects.addPlayer(playerInstance);
        }
        if (timer != null) return;
        timer = new Timer(delay);
        timer.run = function()
        {
            objects.tileset.bitmapData.lock();
            var time = Timer.stamp();
            ground.clear();
            objects.clear();
            trace("clear " + Std.string(Timer.stamp() - time));
            time = Timer.stamp();
            if (objects.getFill() > 0.90 || objects.clearBool)
            {
                objects.cacheMap = new Map<Int,Int>();
                objects.tileX = 0;
                objects.tileY = 0;
                objects.tileHeight = 0;
                //player
                it = data.playerMap.iterator();
                while (it.hasNext())
                {
                    objects.player = it.next();
                    objects.player.removeTiles();
                    objects.add(objects.player.instance.po_id,0,0,true,false);
                    objects.player.addTiles(cast(objects.object,Player).sprites);
                }
                objects.tileset.bitmapData.fillRect(objects.tileset.bitmapData.rect,0xFFFFFFFF);
            }
            //center point to determine range
            var cx:Int = mapInstance.x + Std.int(mapInstance.width/2);
            var cy:Int = mapInstance.y + Std.int(mapInstance.height/2);
            var int:Null<Int>;
            if (player != null)
            {
                cx = player.instance.x;
                cy = player.instance.y;
            }
            //trace("start object layer");
            //object layer
            var array:Array<Int> = [];
            for (j in cy - objects.range...cy + objects.range)
            {
                for (i in cx - objects.range...cx + objects.range)
                {
                    add(data.map.object.get(i,j),i,j);
                }
            }
            //trace("object " + Std.string(Timer.stamp() - time));
            time = Timer.stamp();
            //floor layer
            for (j in cy - objects.range...cy + objects.range)
            {
                for (i in cx - objects.range...cx + objects.range)
                {
                    //add floor
                    if (!objects.add(data.map.floor.get(i,j),i,j))
                    {
                        //add ground as there is no floor
                        int = data.map.biome.get(i,j);
                        if (int != null) ground.add(int,i,j);
                    }
                }
            }
            //trace("floor " + Std.string(Timer.stamp() - time));
            time = Timer.stamp();
            it = data.playerMap.iterator();
            while (it.hasNext())
            {
                objects.player = it.next();
                objects.player.sort();
                objects.group.addTile(objects.player);
            }
            //trace("player " + Std.string(Timer.stamp() - time));
            time = Timer.stamp();
            ground.render();
            //trace("ground " + Std.string(Timer.stamp() - time));
            objects.tileset.bitmapData.unlock();
            //trace("get fill " + objects.getFill());
            timer.stop();
            timer = null;
        }
    }
    //add object arraqy
    public function add(array:Array<Int>,x:Int,y:Int,container:Bool=false,push:Bool=true)
    {
        if (array != null) 
        {
            objects.add(array[0],x,y,array.length > 1 ? true : container,push);
            objects.containing = array.length > 1 ? array[0] : 0;
            var index:Int = 0;
            for (i in 1...array.length)
            {
                if (array[i] < 0)
                {
                    //sub container
                    objects.add(array[i] * -1,x,y,true,push,index);
                    index++;
                }else{
                    //container
                    objects.add(array[i],x,y,container,push);
                    index = 0;
                }
            }
        }
    }
    public function setPlayer(player:Player)
    {
        player.program = program;
        Main.player = player;
        player.sort();
        console.set("player",player);
        //center instantly
        objects.group.x = -player.x * objects.scale + objects.width/2;
        objects.group.y = -player.y * objects.scale + objects.height/2;
    }
    public function end()
    {
        switch(Main.client.tag)
        {
            case PLAYER_UPDATE:
            if (player == null && objects.player != null) 
            {
                setPlayer(objects.player);
                player.sort();
                gameBool = true;
                resize(null);
            }
            objects.player = null;
            Main.client.tag = null;
            default:
        }
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
    public function connect()
    {
        client.login.accept = function()
        {
            trace("accept");
            //set message reader function to game
            Main.client.message = message;
            Main.client.end = end;
            //Main.client.login = null;
            Main.client.tag = null;
            index = 0;
        }
        client.login.reject = function()
        {
            trace("reject");
            //Main.client.login = null;
        }
        client.message = Main.client.login.message;
        trace("connect " + Main.client.ip + " email " + Main.client.login.email);
        client.connect();
    }
    public function message(input:String) 
    {
        switch(Main.client.tag)
        {
            case COMPRESSED_MESSAGE:
            var array = input.split(" ");
            Main.client.compress = Std.parseInt(array[1]);
            Main.client.tag = null;
            case PLAYER_UPDATE:
            playerInstance = new PlayerInstance(input.split(" "));
            #if openfl
            objects.addPlayer(playerInstance);
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
                data.map.setRect(mapInstance.x,mapInstance.y,mapInstance.width,mapInstance.height,input);
                #if openfl
                render();
                #end
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
                        if (data.map.x > mapInstance.x) data.map.x = mapInstance.x;
                        if (data.map.y > mapInstance.y) data.map.y = mapInstance.y;
                        if (data.map.width < mapInstance.x + mapInstance.width) data.map.width = mapInstance.x + mapInstance.width;
                        if (data.map.height < mapInstance.y + mapInstance.height) data.map.height = mapInstance.y + mapInstance.height;
                        trace("map chunk " + mapInstance.toString());
                        index = 0;
                        //set compressed size wanted
                        Main.client.compress = mapInstance.compressedSize;
                        compress = true;
                    }
                }
            }
            case MAP_CHANGE:
            //x y new_floor_id new_id p_id optional oldX oldY playerSpeed
            //trace("change " + input.split(" "));
            var change = new MapChange(input.split(" "));
            #if openfl
            var tile:Tile;
            var id:Array<Int> = change.floor > 0 ? [change.floor] : change.id;
            trace("change id: " + id);
            var move:Bool = change.speed > 0 ? true : false;
            //removal location
            var rx:Int = change.speed > 0 ? change.oldX : change.x;
            var ry:Int = change.speed > 0 ? change.oldY : change.y;
            //removal
            var array:Array<Tile> = [];
            //remove data
            if (change.floor == 1 && !move)
            {
                array = data.tileData.floor.get(rx,ry);
                data.tileData.floor.set(rx,ry,null);
                data.map.floor.set(rx,ry,0);
            }else{
                array = data.tileData.object.get(rx,ry);
                data.tileData.object.set(rx,ry,null);
                data.map.object.set(rx,ry,[0]);
            }
            if (array != null) for (tile in array) objects.group.removeTile(tile);
            //add new
            add(id,rx,ry,move,!move);
            if (move)
            {
                //add to new location
                data.tileData.object.set(change.x,change.y,[objects.object]);
                data.map.object.set(change.x,change.y,id);
                //tween to location
                Actuate.tween(objects.object,1,{x:change.x * Static.GRID,y:(Static.tileHeight - change.y) * Static.GRID}).ease(Quad.easeInOut);
            }
            #end
            //change data todo:
            Main.client.tag = null;
            index = 0;
            case HEAT_CHANGE:
            //trace("heat " + input);
            Main.client.tag = null;
            index = 0;
            case FOOD_CHANGE:
            trace("food change " + input);
            var array = input.split(" ");
            food.graphics.clear();
            food.graphics.beginFill(0);
            food.graphics.drawRect(200,0,100,20);
            food.graphics.beginFill(0xFF0000);
            food.graphics.drawRect(200,0,Std.parseInt(array[0])/Std.parseInt(array[1]) * 100,20);
            //also need to set new movement move_speed: is floating point playerSpeed in grid square widths per second.
            if (player != null) player.instance.move_speed = Std.parseFloat(array[4]);
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
            data.playerMap.remove(id);
            objects.group.removeTile(player);
            player = null;
            case LINEAGE:
            //p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id

            case NAME:
            //p_id first_name last_name last_name may be ommitted.
            var array = input.split(" ");
            var name:String = array[1] + (array.length > 1 ? " " + array[2] : "");
            draw.username(Std.parseInt(array[0]),name);
            case DYING:
            //p_id isSick isSick is optional 1 flag to indicate that player is sick (client shouldn't show blood UI overlay for sick players)

            case HEALED:
            //p_id player healed no longer dying.

            case MONUMENT_CALL:
            //MN x y o_id monument call has happened at location x,y with the creation object id

            case GRAVE:
            //x y p_id

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

            case FLIGHT_DEST:
            //p_id dest_x dest_y
            trace("FLIGHT FLIGHT FLIGHT " + input.split(" "));
            default:
        }
    }
    public function zoom(i:Int)
    {
        if (objects.scale > 2 && i > 0 || objects.scale < 0.2 && i < 0) return;
        objects.scale += i * 0.08;
    }
}