import openfl.display.TileContainer;
import openfl.display.DisplayObject;
import console.Program;
import data.GameData;
import openfl.display.FPS;
import data.MapData.ArrayData;
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
    public static var objects:Objects;
    var ground:Ground;
    var select:Shape;
    var selectX:Int = 0;
    var selectY:Int = 0;
    var data:GameData;
    var playerInstance:PlayerInstance;
    public var mapInstance:MapInstance;
    var index:Int = 0;
    var compress:Bool = false;
    var inital:Bool = true;
    var program:Program;
    var string:String = "";
    var gameBool:Bool = false;
    public var player:Player;

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
        //client
        client = new client.Client();
        console = new console.Console();
        addChild(console);
        //launch
        dir();
        cred();
        game();
        connect();
    }
    public function game()
    {
        ground = new Ground();
        objects = new Objects();
        //tile selector
        select = new Shape();
        select.cacheAsBitmap = true;
        select.graphics.lineStyle(2,0xB7B7B7);
        select.graphics.drawRect(0,0,Static.GRID,Static.GRID);
        draw = new Draw(program);
        addChild(ground);
        addChild(select);
        addChild(objects);
        addChild(draw);
        data = new GameData();
        ground.data = data;
        objects.data = data;
        program = new Program(data,console);
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
                player.step(xs,ys);
            }
            //update players
            if (data != null)
            {
                it = data.playerMap.iterator();
                while(it.hasNext())
                {
                    it.next().update();
                }
            }
            //set camera to middle
            objects.group.x = lerp(objects.group.x,-player.x * objects.scale + stage.stageWidth/2 ,0.06);
            objects.group.y = lerp(objects.group.y,-player.y * objects.scale + stage.stageHeight/2,0.06);
            //set ground
            ground.x = objects.group.x;
            ground.y = objects.group.y;
            ground.scaleX = objects.group.scaleX;
            ground.scaleY = objects.group.scaleY;
            //selector
            selectX = Math.floor((stage.mouseX - ground.x)/Static.GRID);
            selectY = Math.floor(stage.mouseY - ground.y/Static.GRID);
            //set local for render
            select.x = selectX * Static.GRID;
            select.y = selectY * Static.GRID;
            //set global
            selectX *= Static.GRID;
            selectY *= Static.GRID;
        }
    }
    public inline function lerp(v0:Float,v1:Float,t:Float)
    {
        return v0 + t * (v1 - v0);
    }
    private function keyDown(e:KeyboardEvent)
    {
        if (console.keyDown(e.keyCode)) return;
        Bind.keys(e,true);
        if (Bind.zoomIn.bool) zoom(1);
        if (Bind.zoomOut.bool) zoom(-1);
        if (Bind.playerSelf.bool)
        {
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
            trace("oid " + player.instance.o_id);
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
    public function mapUpdate() 
    {
        trace("MAP UPDATE");
        //inital set camera
        if (inital)
        {
            objects.group.x = -data.map.x * Static.GRID;
            objects.group.y = -data.map.y * Static.GRID;
            ground.x = objects.group.x;
            ground.y = objects.group.y;
            inital = false;
        }
        var id:Int = 0;
        var array:Array<Tile> = [];
        function remove()
        {
            if (array != null) for (object in array)
            {
                objects.group.removeTile(object);
            }
        }
        for(j in mapInstance.y...mapInstance.y + mapInstance.height)
        {
            //overlap checker
            for (i in mapInstance.x...mapInstance.x + mapInstance.width)
            {
                //remove overlapping
                //ground.indices[data.tileData.biome.get(i,j)] = 0;
                array = data.tileData.floor.get(i,j);
                remove();
                array = data.tileData.object.get(i,j);
                remove();
                //add floor
                if (!objects.add(data.map.floor.get(i,j),i,j))
                {
                    //add ground as there is no floor
                    ground.add(data.map.biome.get(i,j),i,j);
                }
                //object
                objects.add(data.map.object.get(i,j),i,j);
            }
        }
        trace("tilemap percent " + objects.getFill());
        ground.render();
    }
    public function setPlayer(player:Player)
    {
        this.player = player;
        console.set(player);
    }
    public function end()
    {
        switch(Main.client.tag)
        {
            case PLAYER_UPDATE:
            #if openfl
            if (player == null) 
            {
                setPlayer(objects.player);
                player.sort();
                gameBool = true;
                resize(null);
            }
            objects.player = null;
            #end
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

            ground.x = objects.x;
            ground.y = objects.y;
            ground.width = objects.width;
            ground.height = objects.height;
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
            case PLAYER_UPDATE:
            playerInstance = new PlayerInstance(input.split(" "));
            #if openfl
            objects.addPlayer(playerInstance);
            #end
            case PLAYER_MOVES_START:
            var playerMove = new PlayerMove(input.split(" "));
            if (playerMove.id == player.instance.p_id) return;
            if (data.playerMap.exists(playerMove.id))
            {
                playerMove.movePlayer(data.playerMap.get(playerMove.id));
            }
            Main.client.tag = null;
            case MAP_CHUNK:
            trace("MAP CHUNK");
            if(compress)
            {
                Main.client.tag = null;
                data.map.setRect(mapInstance.x,mapInstance.y,mapInstance.width,mapInstance.height,input);
                #if openfl
                mapUpdate();
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
                        trace("width " + mapInstance.width);
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
            var change = new MapChange(input.split(" "));
            #if openfl
            var tile:Tile;
            if (change.speed > 0)
            {
                //move object 
            }else{
                var type = change.floor > 0 ? 0 : 1;
                var id = type == 0 ? change.floor : change.id;
                //remove object regardless
                /*for (i in 0...objects.numTiles)
                {
                    tile = objects.group.getTileAt(i);
                    if (change.x == tile.data.x && change.y == tile.data.y && type == tile.data.type)
                    {
                        objects.group.removeTile(tile);
                        break;
                    }
                }
                if (id > 0)
                {
                    //add new object to map
                    objects.add(id,change.x,change.y,false);
                }*/
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
            //also need to set new movement move_speed: is floating point playerSpeed in grid square widths per second.
            case FRAME:
            Main.client.tag = null;
            index = 0;
            case PLAYER_SAYS:
            trace("player say " + input);
            #if openfl
            draw.say(input);
            #end
            case PLAYER_OUT_OF_RANGE:
            //player is out of range

            case LINEAGE:
            //p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id

            case NAME:
            //p_id first_name last_name last_name may be ommitted.

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