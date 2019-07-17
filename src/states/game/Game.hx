package states.game;

import openfl.display.Bitmap;
import haxe.ds.Vector;
import data.PlayerData.PlayerType;
import console.Program;
import console.Console;
import data.MapData;
import data.MapData.MapInstance;
import data.PlayerData.PlayerInstance;
import data.PlayerData.PlayerMove;
import data.GameData;
import client.ClientTag;
import haxe.io.Bytes;

#if openfl
import openfl.display.FPS;
import openfl.display.DisplayObject;
import openfl.ui.Keyboard;
import ui.Text;
import settings.Bind;
#end

class Game #if openfl extends states.State #end
{
    #if openfl
    var dialog:Dialog;
    public var ground:Ground;
    public var objects:Objects;
    public var cameraSpeed:Float = 10;
    //camera
    public var cameraX:Int = 0;
    public var cameraY:Int = 0;
    //scale used for zoom in and out
    public var scale(get, set):Float;
    function get_scale():Float 
    {
        return scaleX;
    }
    
    function set_scale(scale:Float):Float 
    {
        scaleX = scale;
        scaleY = scale;
        center();
        return scale;
    }
    #end
    var playerInstance:PlayerInstance;
    public var mapInstance:MapInstance;
    var index:Int = 0;
    public var data:GameData;
    var compress:Bool = false;
    var inital:Bool = true;

    var program:Program;
    public function new()
    {
        //delelerative syntax for program console
        program = new Program(this);
        //set interp
        Console.interp.variables.set("game",this);
        Console.interp.variables.set("program",program);
        data = new GameData();

        #if openfl
        super();
        ground = new Ground(this);
        objects = new Objects(this);
        dialog = new Dialog(this);
        addChild(ground);
        addChild(objects);
        addChild(dialog);
        #end
        //background color of game
        stage.color = 0xFFFFFF;
        //connect
        if(!true)
        {
            Main.client.login.accept = function()
            {
                trace("accept");
                //set message reader function to game
                Main.client.message = message;
                Main.client.end = end;
                Main.client.login = null;
                Main.client.tag = null;
            }
            Main.client.login.reject = function()
            {
                trace("reject");
                Main.client.login = null;
            }
            Main.client.login.email = "test@test.co.uk";
            Main.client.login.key = "WC2TM-KZ2FP-LW5A5-LKGLP";
            Main.client.message = Main.client.login.message;
            Main.client.ip = "game.krypticmedia.co.uk";
            Main.client.port = 8007;
            Main.client.connect();
        }else{
            //playground
            objects.size(32,30);
            //player
            /*setPlayer(cast(objects.add(19,0,0,true),Player));
            Player.main.instance = new PlayerInstance([]);
            Player.main.instance.move_speed = 3;
            data.playerMap.set(0,Player.main);*/
            //bush
            objects.add(30,3,-1);
            //sheep
            objects.add(575,2,-2).animate(2);
            //trees
            objects.add(65,4,-4);
            objects.add(2454,5,-4);
            objects.add(49,6,-5);
            objects.add(530,3,-3);
            //tileset
            var bitmap = new Bitmap(objects.tileset.bitmapData);
            bitmap.alpha = 0.5;
            addChild(bitmap);

            objects.getFill();
        }
    }
    //client events
    #if openfl
    override function update()
    {
        super.update();
        //controls
        var cameraArray:Array<DisplayObject> = [ground,objects];
        if (Bind.cameraUp.bool) for (obj in cameraArray) obj.y += cameraSpeed;
        if (Bind.cameraDown.bool) for (obj in cameraArray) obj.y += -cameraSpeed;
        if (Bind.cameraLeft.bool) for (obj in cameraArray) obj.x += cameraSpeed;
        if (Bind.cameraRight.bool) for (obj in cameraArray) obj.x += -cameraSpeed;

        if(Player.main != null)
        {
            var xs:Int = 0;
            var ys:Int = 0;
            if (Bind.playerUp.bool) ys += 1;
            if (Bind.playerDown.bool) ys += -1;
            if (Bind.playerLeft.bool) xs += -1;
            if (Bind.playerRight.bool) xs += 1;
            if (xs != 0 || ys != 0) Player.main.step(xs,ys);
            //animations
            if (Bind.playerDrop.bool) Player.main.animate(2);
            //grabs object where standing
            //if (Bind.playerPick.bool) 
        }
        //updates
        objects.update();
        //players
        var it = data.playerMap.iterator();
        while(it.hasNext())
        {
            it.next().update();
        }
    }
    public function mapUpdate() 
    {
        trace("MAP UPDATE");
        //width = 32, height = 30
        objects.size(mapInstance.width,mapInstance.height);
        var x:Int = 0;
        var y:Int = 0;
        //scale = 1;
        for(j in mapInstance.y...mapInstance.y + mapInstance.height)
        {
            for (i in mapInstance.x...mapInstance.x + mapInstance.width)
            {
                //ground
                ground.graphics.clear();
                //objects get local
                y = j - data.map.y;
                x = i - data.map.x;
                //set global
                objects.addObject(data.map.object[y][x],i,j);
            }
        }
        var bitmap = new Bitmap(objects.tileset.bitmapData);
        bitmap.alpha = 0.5;
        bitmap.width = Main.setWidth;
        bitmap.height = Main.setHeight;
        addChild(bitmap);
        objects.getFill();
    }
    public function center()
    {
        x = (Main.setWidth - objects.width)/2 * scale;
        y = (Main.setHeight - objects.height)/2 * scale;
    }
    #end
    
    public function end()
    {
        switch(Main.client.tag)
        {
            case PLAYER_UPDATE:
            trace("set main");
            if (Player.main == null) 
            {
                setPlayer(objects.player);
            }
            objects.player = null;
            objects.sort();
            default:
        }
    }
    public function setPlayer(player:Player)
    {
        Player.main = player;
        Console.interp.variables.set("player",Player.main);
    }
    public function message(input:String) 
    {
        switch(Main.client.tag)
        {
            case PLAYER_UPDATE:
            playerInstance = new PlayerInstance(input.split(" "));
            objects.addPlayer(playerInstance);
            case PLAYER_MOVES_START:
            var playerMove = new PlayerMove(input.split(" "));
            if (data.playerMap.exists(playerMove.id))
            {
                playerMove.movePlayer(data.playerMap.get(playerMove.id));
            }
            case MAP_CHUNK:
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
                        if (data.map.height < mapInstance.y + mapInstance.height) data.map.height= mapInstance.y + mapInstance.height;
                        trace("map chunk " + mapInstance.toString());
                        index = 0;
                        //set compressed size wanted
                        Main.client.compress = mapInstance.compressedSize;
                        trace("set compress " + Main.client.compress);
                        compress = true;
                    }
                }
            }
            case MAP_CHANGE:
            //x y new_floor_id new_id p_id optional oldX oldY cameraSpeed
            var mapChange = new MapChange(input.split(" "));
            case HEAT_CHANGE:
            //trace("heat " + input);

            case FOOD_CHANGE:
            //trace("food change " + input);
            //also need to set new movement move_speed: is floating point cameraSpeed in grid square widths per second.
            case FRAME:
            Main.client.tag = "";
            case PLAYER_SAYS:
            trace("player say " + input);
            #if openfl
            dialog.say(input);
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
}