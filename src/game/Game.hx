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
import client.ClientTag;

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
    public static var dir:String;
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
            //if (valid(settings.data.get("fullscreen"))) stage.window.fullscreen = string == "1" ? true : false;
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
    private function connect(reconnect:Bool=false)
    {
        client.accept = function()
        {
            trace("accept");
            client.message = message;
            client.accept = null;
        }
        client.reject = function()
        {
            trace("reject");
            client.reject = null;
        }
        client.message = client.login;
        client.connect(reconnect);
    }
    private function message(tag:ClientTag,input:Array<String>) 
    {
        switch(tag)
        {
            case COMPRESSED_MESSAGE:
            var array = input[0].split(" ");
            client.compress(Std.parseInt(array[0]),Std.parseInt(array[1]));
            case PLAYER_EMOT:
            //p_id emot_index ttl_sec
            //ttl_sec is optional, and specifies how long the emote should be shown
            //-1 is permanent, -2 is permanent but not new so should be skipped
            case PLAYER_UPDATE:
            var list:Array<PlayerInstance> = [];
            for (data in input) 
            {
                list.push(new PlayerInstance(data.split(" ")));
            }
            playerUpdate(list);
            case PLAYER_MOVES_START:
            var instance:PlayerMove;
            for (data in input)
            {
                instance = new PlayerMove(data.split(" "));
                playerMoveStart(instance);
            }
            case MAP_CHUNK:
            if (mapInstance == null)
            {
                var instance = input[0].split(" ");
                var compress = input[1].split(" ");
                mapInstance = new MapInstance();
                mapInstance.width = Std.parseInt(instance[0]);
                mapInstance.height = Std.parseInt(instance[1]);
                mapInstance.x = Std.parseInt(instance[2]);
                mapInstance.y = Std.parseInt(instance[3]);
                client.compress(Std.parseInt(compress[0]),Std.parseInt(compress[1]));
            }else{
                data.map.setRect(mapInstance,input[0]);
                mapChunk(mapInstance);
                mapInstance = null;
            }
            case MAP_CHANGE:
            //var change = new MapChange(input);
            var change:MapChange;
            for (data in input)
            {
                change = new MapChange(data.split(" "));
                mapChange(change);
            }
            case HEAT_CHANGE:
            //heat food_time indoor_bonus
            
            case FOOD_CHANGE:
            //trace("food change " + input);
            //foodPercent = Std.parseInt(input[0])/Std.parseInt(input[1]);
            
            case FRAME:
            case PLAYER_SAYS:
            /*array = input.split("/");
            //trace("id " + array[0]);
            var text = array[1].substring(2,array[1].length);*/
            //id = Std.parseInt(array[0]);
            case PLAYER_OUT_OF_RANGE:
            //player is out of range
            trace("player out of range " + input);
            var id:Int = Std.parseInt(input[0]);
            var player = data.playerMap.get(id);
            case LINEAGE:
            //p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id

            case NAME:
            //p_id first_name last_name last_name may be ommitted.
            var id:Int = Std.parseInt(input[0]);
            var name:String = input[1] + (input.length > 1 ? " " + input[2] : "");
            case HEALED:
            //p_id player healed no longer dying.

            case MONUMENT_CALL:
            //MN x y o_id monument call has happened at location x,y with the creation object id

            case GRAVE:
            //x y p_id
            var x:Int = Std.parseInt(input[0]);
            var y:Int = Std.parseInt(input[1]);
            var id:Int = Std.parseInt(input[2]);
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
            data.map.valleySpacing = Std.parseInt(input[0]);
            data.map.valleyOffsetY = Std.parseInt(input[1]);
            
            case FLIGHT_DEST:
            //p_id dest_x dest_y
            trace("FLIGHT FLIGHT FLIGHT " + input);
            default:
        }
    }
}