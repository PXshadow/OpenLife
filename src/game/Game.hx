package game;
import data.object.player.PlayerInstance;
import data.object.player.PlayerMove;
import data.map.MapChange;
import data.map.MapInstance;
import console.Console;
import client.Client;
import settings.Settings;
import data.GameData;
import haxe.io.Path;

class Game extends GameHeader
{
    /**
     * static game data
     */
    public static var data:GameData;
    public var settings:Settings;
    public var client:Client;
    /**
     * Used for string tool functions
     */
    var string:String;
    /**
     * index multi message such as MapChunk
     */
    var index:Int = 0;
    public static var dir:String;
    var compress:Bool = false;
    var mapInstance:MapInstance;
    public function new()
    {
        #if openfl
        super();
        #end
        data = new GameData();
        settings = new Settings();
        client = new Client();
    }
    public function update(_)
    {
        client.update();
    }
    public function directory():Bool
    {
        #if (windows || !openfl)
        dir = "";
        #else
        dir = Path.normalize(lime.system.System.applicationDirectory);
        dir = Path.removeTrailingSlashes(Game.dir) + "/";
        #end
        #if mac
        dir = dir.substring(0,dir.indexOf("/Contents/Resources/"));
        dir = dir.substring(0,dir.lastIndexOf("/") + 1);
        #end
        //check to see if location is valid
        if (exist(["groundTileCache","objects","sprites","animations","transitions"])) return true;
        return false;
    }
    //helper functions
    private function exist(folders:Array<String>):Bool
    {
        for (folder in folders)
        {
            if (!sys.FileSystem.exists(dir + folder)) return false;
        }
        return true;
    }
    private inline function valid(obj:Dynamic):Bool
    {
        if (obj == null || obj == "") return false;
        string = cast obj;
        return true;
    }
    public function cred()
    {
        //settings to use infomation
        if (!settings.fail)
        {
            //account
            if (valid(settings.data.get("email"))) client.email = string;
            if (valid(settings.data.get("accountKey"))) client.key = string;
            if (valid(settings.data.get("useCustomServer")) && string == "1")
            {
                if (valid(settings.data.get("customServerAddress"))) client.ip = string;
                if (valid(settings.data.get("customServerPort"))) client.port = Std.parseInt(string);
            }
            //window
            #if openfl
            if (valid(settings.data.get("borderless"))) stage.window.borderless = string == "1" ? true : false;
            if (valid(settings.data.get("fullscreen"))) stage.window.fullscreen = string == "1" ? true : false;
            if (valid(settings.data.get("screenWidth"))) stage.window.width = Std.parseInt(string);
            if (valid(settings.data.get("screenHeight"))) stage.window.height = Std.parseInt(string);
            if (valid(settings.data.get("targetFrameRate"))) stage.frameRate = Std.parseInt(string);
            #end
        }
        //by pass settings and force email and key if secret account
        #if secret
        trace("set secret");
        client.email = Secret.email;
        client.key = Secret.key;
        client.ip = Secret.ip;
        client.port = Secret.port;
        #end
    }
    private function end()
    {
        switch(client.tag)
        {
            case PONG:
            client.ping = UnitTest.stamp();
            default:
        }
    }
    private function connect()
    {
        client.accept = function()
        {
            trace("accept");
            client.message = message;
            client.end = end;
            client.accept = null;
            client.tag = null;
            index = 0;
        }
        client.reject = function()
        {
            trace("reject");
            client.reject = null;
        }
        client.message = client.login;
        client.connect();
    }
    var array:Array<String>;

    private function message(input:String) 
    {
        switch(client.tag)
        {
            case COMPRESSED_MESSAGE:
            array = input.split(" ");
            client.compress = Std.parseInt(array[1]);
            client.tag = null;
            case PLAYER_EMOT:
            array = input.split(" ");
            //p_id emot_index ttl_sec
            //ttl_sec is optional, and specifies how long the emote should be shown
            //-1 is permanent, -2 is permanent but not new so should be skipped
            case PLAYER_UPDATE:
            //playerUpdate(new PlayerInstance(input.split(" ")));
            case PLAYER_MOVES_START:
            //playerMoveStart(new PlayerMove(input.split(" ")));
            client.tag = null;
            case MAP_CHUNK:
            if(compress)
            {
                client.tag = null;
                data.map.setRect(mapInstance,input);
                //mapChunk(mapInstance);
                //mapInstance = null;
                //toggle to go back to istance for next chunk
                compress = false;
            }else{
                array = input.split(" ");
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
                        client.compress = mapInstance.compressedSize;
                        compress = true;
                    }
                }
            }
            case MAP_CHANGE:
            var change = new MapChange(input.split(" "));
            
            client.tag = null;
            index = 0;
            case HEAT_CHANGE:
            //heat food_time indoor_bonus
            //trace("heat " + input);
            client.tag = null;
            index = 0;
            case FOOD_CHANGE:
            //trace("food change " + input);
            array = input.split(" ");
            //foodPercent = Std.parseInt(array[0])/Std.parseInt(array[1]);
            case FRAME:
            client.tag = null;
            index = 0;
            case PLAYER_SAYS:
            array = input.split("/");
            //trace("id " + array[0]);
            var text = array[1].substring(2,array[1].length);
            //id = Std.parseInt(array[0]);
            case PLAYER_OUT_OF_RANGE:
            //player is out of range
            trace("player out of range " + input);
            var id:Int = Std.parseInt(input);
            var player = data.playerMap.get(id);
            case LINEAGE:
            //p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id

            case NAME:
            //p_id first_name last_name last_name may be ommitted.
            array = input.split(" ");
            var name:String = array[1] + (array.length > 1 ? " " + array[2] : "");
            case HEALED:
            //p_id player healed no longer dying.

            case MONUMENT_CALL:
            //MN x y o_id monument call has happened at location x,y with the creation object id

            case GRAVE:
            //x y p_id
            array = input.split(" ");
            var x:Int = Std.parseInt(array[0]);
            var y:Int = Std.parseInt(array[1]);
            var id:Int = Std.parseInt(array[2]);
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
            array = input.split(" ");
            data.map.valleySpacing = Std.parseInt(array[0]);
            data.map.valleyOffsetY = Std.parseInt(array[1]);
            
            case FLIGHT_DEST:
            //p_id dest_x dest_y
            trace("FLIGHT FLIGHT FLIGHT " + input.split(" "));
            default:
        }
    }
}