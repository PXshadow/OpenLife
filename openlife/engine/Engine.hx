package openlife.engine;
import openlife.data.object.player.PlayerMove;
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;
import openlife.data.map.MapChange;
import openlife.data.map.MapInstance;
import openlife.client.Client;
import openlife.settings.Settings;
import openlife.data.GameData;
import haxe.io.Path;
import openlife.client.ClientTag;
//
class Engine extends EngineHeader
{
    /**
     * static game data
     */
    public static var data:GameData;
    public var settings:Settings;
    public static var program:Program;
    public var client:Client;
    /**
     * Used for string tool functions
     */
    var string:String;
    public static var dir:String;
    var mapInstance:MapInstance;
    public function new()
    {
        data = new GameData();
        settings = new Settings();
        client = new Client();
        program = new Program(client);
    }

    //helper functions
    private function exist(folders:Array<String>):Bool
    {
        #if sys
        for (folder in folders)
        {
            if (!sys.FileSystem.exists(dir + folder)) return false;
        }
        #else
        return false;
        #end
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
            #if visual
            if (valid(settings.data.get("borderless"))) window.borderless = string == "1";
            //if (valid(settings.data.get("fullscreen"))) stage.window.fullscreen = string == "1";
            if (valid(settings.data.get("screenWidth"))) window.width = Std.parseInt(string);
            if (valid(settings.data.get("screenHeight"))) window.height = Std.parseInt(string);
            if (valid(settings.data.get("targetFrameRate"))) window.frameRate = Std.parseInt(string);
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
    public function connect(reconnect:Bool=false)
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
            var index:Int = 0;
            var index2:Int = 0;
            var secs:Int = 0;
            for (line in input)
            {
                index = line.indexOf(" ");
                index2 = line.indexOf(" ",index + 1);
                if (index2 == -1)
                {
                    //no ttl_sec
                    secs = 10;
                    index2 = line.length;
                }else{
                    //ttl_sec exists
                    secs = Std.parseInt(line.substr(index2 + 1));
                }
                emot(Std.parseInt(line.substring(0,index)),Std.parseInt(line.substring(index + 1,index2)),secs);
            }
            //p_id emot_index ttl_sec
            //ttl_sec is optional, and specifies how long the emote should be shown
            //-1 is permanent, -2 is permanent but not new so should be skipped
            case PLAYER_UPDATE:
            var list:Array<PlayerInstance> = [];
            for (data in input.slice(0,input.length - 1)) 
            {
                list.push(new PlayerInstance(data.split(" ")));
            }
            playerUpdate(list);
            case PLAYER_MOVES_START:
            var a:Array<String> = [];
            for (string in input)
            {
                a = string.split(" ");
                if (a.length < 8 || a.length % 2 != 0) continue;
                playerMoveStart(new PlayerMove(a));
            }
            case MAP_CHUNK:
            if (mapInstance == null)
            {
                var instance = input[0].split(" ");
                var compress = input[1].split(" ");
                mapInstance = new MapInstance();
                trace("instance " + instance);
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
            var change:MapChange;
            for (data in input)
            {
                change = new MapChange(data.split(" "));
                Engine.data.map.object.set(change.oldX,change.oldY,[0]);
                Engine.data.map.object.set(change.x,change.y,change.id);
                mapChange(change);
            }
            case HEAT_CHANGE:
            //heat food_time indoor_bonus
            var array = input[0].split(" ");
            heatChange(Std.parseFloat(array[0]),Std.parseFloat(array[1]),Std.parseFloat(array[2]));
            case FOOD_CHANGE:
            var array = input[0].split(" ");
            //foodPercent = Std.parseInt(input[0])/Std.parseInt(input[1]);
            //food_store food_capacity last_ate_id last_ate_fill_max move_speed responsible_id
            case FRAME:
            frame();
            case PLAYER_SAYS:
            var index:Int = 0;
            for (line in input)
            {
                index = line.indexOf("/");
                says(Std.parseInt(line.substring(0,index)),line.substr(index + 2),line.substr(index + 1,1) == "1");
            }
            /*array = input.split("/");
            //trace("id " + array[0]);
            var text = array[1].substring(2,array[1].length);*/
            //id = Std.parseInt(array[0]);
            case LOCATION_SAYS:
            var array:Array<String> = [];
            for (line in input)
            {
                array = line.split(" ");
                saysLocation(Std.parseInt(array[0]),Std.parseInt(array[1]),array[2]);
            }
            case BAD_BIOMES:
            var index:Int = 0;
            for (line in input)
            {
                index = line.indexOf(" ");
                badBiomes(Std.parseInt(line.substring(0,index)),line.substr(index + 1));
            }
            case PLAYER_OUT_OF_RANGE:
            //player is out of range
            var list:Array<Int> = [];
            for (string in input) list.push(Std.parseInt(string));
            playerOutOfRange(list);
            case BABY_WIGGLE:
            var list:Array<Int> = [];
            for (string in input) list.push(Std.parseInt(string));
            babyWiggle(list);
            case LINEAGE:
            //p_id mother_id grandmother_id great_grandmother_id ... eve_id eve=eve_id
            //included at the end with the eve= tag in front of it.
            var array:Array<String> = [];
            for (line in input)
            {
                lineage(line.split(" "));
            }
            case NAME:
            //p_id first_name last_name last_name may be ommitted.
            var array:Array<String> = [];
            var lastName:String = "";
            for (line in input)
            {
                array = line.split(" ");
                if (array.length > 2)
                {
                    //last name
                    lastName = array[2];
                }else{
                    //no last name
                    lastName = "";
                }
                playerName(Std.parseInt(array[0]),array[1],lastName);
            }
            case APOCALYPSE:
            //Indicates that an apocalypse is pending.  Gives client time to show a visual effect.
            apocalypse();
            case APOCALYPSE_DONE:
            //Indicates that an apocalypse is now over.  Client should go back to displaying world.
            case DYING:
            var index:Int = 0;
            var sick:Bool;
            for (line in input)
            {
               index = line.indexOf(" ");
               if (index == -1)
               {
                   index = line.length;
                   sick = false;
               }else{
                   sick = true;
               }
               dying(Std.parseInt(line.substring(0,index)),sick);
            }
            apocalypseDone();
            case HEALED:
            //p_id player healed no longer dying.
            for (line in input)
            {
                healed(Std.parseInt(line));
            }
            case POSSE_JOIN: //FINISH tommrow
            //Indicates that killer joined posse of target.
            //If target = 0, killer has left the posse.
            var array = input[0].split(" ");
            posse(Std.parseInt(array[0]),Std.parseInt(array[1]));
            case MONUMENT_CALL:
            //MN x y o_id monument call has happened at location x,y with the creation object id
            var array = input[0].split(" ");
            monument(Std.parseInt(array[0]),Std.parseInt(array[1]),Std.parseInt(array[2]));
            case GRAVE:
            //x y p_id
            var x:Int = Std.parseInt(input[0]);
            var y:Int = Std.parseInt(input[1]);
            var id:Int = Std.parseInt(input[2]);
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
            //data.map.valleySpacing = Std.parseInt(input[0]);
            //data.map.valleyOffsetY = Std.parseInt(input[1]);
            
            case FLIGHT_DEST:
            //p_id dest_x dest_y
            trace("FLIGHT FLIGHT FLIGHT " + input);
            case PONG:
            //client.ping = Std.int(UnitTest.stamp() * 100);
            //trace("ping: " + client.ping);
            case HOMELAND:
            var array = input[0].split(" ");
            homeland(Std.parseInt(array[0]),Std.parseInt(array[1]),array[2]);
            default:
        }
    }
}