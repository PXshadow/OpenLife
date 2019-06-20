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
    var router:Bool = true;
    var output:String = "";
    var data:String = "";
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
        trace("output " + output + " tag " + tag);
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
            var string = array[0] + " " + array[1];
            //floor
            if(Std.parseInt(array[2]) > 0) 
            {

            }
            //object
            if(Std.parseInt(array[3]) > 0)
            {

            }
            //p_id

            //optional speed

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
            case PLAYER_MOVES_START:
            trace("player move start data " + data);
            case PLAYER_OUT_OF_RANGE:
            
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
        if (data.substring(data.length - 1,data.length) == "#")
        {
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
    public function send(data:String)
    {
        #if sys
        socket.output.writeString(data + "#");
        #end
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