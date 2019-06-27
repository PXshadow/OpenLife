import openfl.events.Event;
import MapData.MapChange;
import PlayerData.PlayerMove;
import MapData.MapInstance;
import haxe.io.Encoding;
import haxe.io.Bytes;
import Message.MessageTag;
#if sys
import sys.net.Socket;
import sys.net.Host;
#end
import haxe.io.Error;
import haxe.crypto.Hmac;
import PlayerData.PlayerInstance;
import haxe.Timer;

class Client
{
    #if sys
    var socket:Socket;
    var server:Socket;
    #end
    var tag:MessageTag;
    var index:Int = 0;
    var mapInstance:MapInstance;
    var playerInstance:PlayerInstance;
    public var map:MapData;
    public var player:PlayerData;
    var challenge:String = "";
    var login:Bool = true;
    var router:Bool = !true;
    var output:String = "";
    var data:String = "";
    var aliveTimer:Timer;
    @:isVar var compress(get,set):Bool = false;
    function get_compress():Bool
    {
        return compress;
    }
    function set_compress(value:Bool):Bool
    {
        if(value) tagRemove = true;
        return compress = value;
    }
    var tagRemove:Bool = false;
    public function new()
    {
        map = new MapData();
        player = new PlayerData();
    }
    public function update()
    {
        #if sys
        if(socket == null) return;
        output = "";
        data = "";
		try {
            if(!compress) data = socket.input.readLine();
		}catch(e:Dynamic)
		{
			if(e != "Blocking" && e != Error.Blocked && e != "Blocked")
			{
                //trace("e " + e);
			}
		}
        if(compress)
        {
            unCompress();
        }else{
            if (data.length > 0) process();
        }

        if(router)
        {
            if(server == null) return;
            //write
            //if(output.length > 0) server.output.writeString(output + "\n",RawNative);
            //read
            try {
                var client = server.accept();
                client.write("ACCEPT");
            }catch(e:Dynamic)
            {
                if(e != "Blocking" && e != Error.Blocked && e != "Blocked")
			    {
                    //trace("e " + e);
			    }
            }
            try {
                var data = server.read();
                socket.write(data);
                //trace("read " + data);
            }catch(e:Dynamic)
            {
                if(e != "Blocking" && e != Error.Blocked && e != "Blocked")
			    {
                    //trace("e " + e);
			    }
            }
        }
        #end
    }
    private function end()
    {
        //trace("end");
        //end message
        switch(tag)
        {
            case PLAYER_UPDATE:
            if (Main.client.player != null) Main.client.player.update();
            default:
        }
    }
    private function process()
    {
        //router
        output = data;
        //trace("data " + data);
        if(login)
        {
            if (tag == "")
            {
                tag = data;
                return;
            }
        }else{
            if(data.substring(0,1) == "#")
            {
                //behavior
                end();
                //new tag
                index = 0;
                tag = data.substring(1,data.length);
                return;
            }
            if(tag == "")
            {
                tag = data;
                return;
            }
        }
        //trace("output " + output + " tag " + tag);
        //data 
        switch(tag)
        {
            case PLAYER_UPDATE:
            var array = data.split(" ");
            playerInstance = new PlayerInstance(data.split(" "));
            case PLAYER_MOVES_START:
            var playerMove = new PlayerMove(data.split(" "));
            //p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0
            //264 0 -1 0.503 0.503 0 1 1
            case MAP_CHUNK:
            var array = data.split(" ");
            trace("map chunk array " + array);
            for(value in array)
            {
                switch(index++)
                {
                    case 0:
                    mapInstance = new MapInstance();
                    mapInstance.sizeX = Std.parseInt(value);
                    case 1:
                    mapInstance.sizeY = Std.parseInt(value);
                    case 2:
                    mapInstance.x = Std.parseInt(value);
                    case 3:
                    mapInstance.y = Std.parseInt(value);
                    case 4:
                    mapInstance.rawSize = Std.parseInt(value);
                    case 5:
                    mapInstance.compressedSize = Std.parseInt(value);
                    mapInstance.bytes = Bytes.alloc(mapInstance.compressedSize);
                    map.setX = mapInstance.x;
                    map.setY = mapInstance.y;
                    map.setWidth = mapInstance.sizeX;
                    map.setHeight = mapInstance.sizeY;
                    trace("map chunk " + mapInstance.toString());
                    index = 0;
                    compress = true;
                }
            }
            case MAP_CHANGE:
            //x y new_floor_id new_id p_id optional oldX oldY speed
            var array = data.split(" ");
            var mapChange = new MapChange();
            mapChange.x = Std.parseInt(array[0]);
            mapChange.y = Std.parseInt(array[1]);
            var string = mapChange.x + "." + mapChange.y;
            //floor
            mapChange.floor = Std.parseInt(array[2]);
            map.floor.set(string,mapChange.floor);
            
            //object
            mapChange.id = Std.parseInt(array[3]);
            map.object.set(string,Std.string(mapChange.id));
            
            //p_id 4
            mapChange.pid = Std.parseInt(array[4]);
            if(mapChange.pid == -1)
            {
                //change no triggered by player
            }else{
                //triggered by player
                if(mapChange.pid < -1)
                {
                    //object was not dropped
                }else{
                    //object was dropped

                }
            }
            //optional speed
            if(array.length > 4)
            {
                var old = array[5] + "." + array[6];
                var speed = array[7];
            }
            case HEAT_CHANGE:
            //trace("heat " + data);

            case FOOD_CHANGE:
            //trace("food change " + data);
            //also need to set new movement move_speed: is floating point speed in grid square widths per second.
            case FRAME:
            tag = "";
            case SERVER_INFO:
			switch(index)
			{
				case 0:
				//current
				case 1:
				//challenge
				challenge = data;
				case 2: 
				//version
				//version = data;
                trace("get version");
                loginRequest("test@test.co.uk","WC2TM-KZ2FP-LW5A5-LKGLP");
                tag = "";
			}
			index++;
            case PLAYER_SAYS:
            trace("player say " + data);
            Main.dialog.say(data);
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
            trace("FLIGHT FLIGHT FLIGHT " + data.split(" "));
            case ACCEPTED:
            trace("accept");
            tag = "";
            case REJECTED:
            trace("reject");
            default:
            //trace("type " + tag + " data " + data);
        }
        //FM
        if (data.substring(data.length - 2,data.length) == "FM")
        {
            //frame
            //trace("frame and remove tag");
            //remove tag
            tag = "";
            return;
        }
    }
    private function loginRequest(email:String,key:String)
    {
        #if sys
        login = false;
		key = StringTools.replace(key,"-","");
        socket.output.writeString("LOGIN\n" + email + "\n" +
		new Hmac(SHA1).make(Bytes.ofString("262f43f043031282c645d0eb352df723a3ddc88f")
		,Bytes.ofString(challenge,RawNative)).toHex() + "\n" +
		new Hmac(SHA1).make(Bytes.ofString(key)
		,Bytes.ofString(challenge)).toHex() + "#");
		tag = "";
		trace("send");
        #end
    }
    public function alive()
    {
        send("KA 0 0");
    }
    public function send(data:String)
    {
        #if sys
        socket.output.writeString(data + "#");
        @:privateAccess Main.console.output.appendText(data + "\n");
        #end
        aliveTimer = new Timer(15 * 1000);
        aliveTimer.run = alive;
    }
    public function connect(ip:String="",port:Int=0)
	{
        #if sys
		ip = "game.krypticmedia.co.uk";
		port = 8007;
        //ip = "bigserver2.onehouronelife.com";
        //port = 8005;
		var host:Host;
		try {
			host = new Host(ip);
		}catch(e:Dynamic)
		{
			return;
		}
		socket = new Socket();
		socket.setBlocking(false);
		socket.setFastSend(true);
		try {
			socket.connect(host,port);
		}catch(e:Dynamic)
		{
            trace("e " + e);
		}
        if(router)
        {
            trace("set router");
            try {
                host = new Host("localhost");
            }catch(e:Dynamic)
            {
                return;
            }
            server = new Socket();
            server.setBlocking(false);
            server.setFastSend(true);
            try {
                server.bind(host,port);
                server.listen(1);
                trace("connected");
            }catch(e:Dynamic)
            {
                trace("connect " + e);
            }
        }
        #end
	}
    public function close()
    {
        #if sys
        socket.close();
        trace("socket closed");
        #end
    }
    private function unCompress()
    {
        #if sys
        //remove #
        if(tagRemove)
        {
            //router
            output += socket.input.readString(1);
            tagRemove = false;
        }
        var data:Bytes;
        switch(tag)
        {
            case MAP_CHUNK:
            if(index >= mapInstance.compressedSize) 
            {
                trace("map issue");
                mapInstance = null;
                tag = null;
                compress = false;
                return;
            }
            data = socket.input.read(mapInstance.compressedSize - index);
            //add output router
            output += data;
            mapInstance.bytes.blit(index,data,0,data.length);
            index += data.length;
            if(index >= mapInstance.compressedSize)
            {
                tag = null;
                compress = false;
                map.setRect(mapInstance.x,mapInstance.y,mapInstance.sizeX,mapInstance.sizeY,haxe.zip.Uncompress.run(mapInstance.bytes,mapInstance.bytes.length).toString());
                trace("after rect");
                mapInstance = null;
            }
            default:
        }
        #end
    }
}