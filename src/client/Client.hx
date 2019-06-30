package client;
import openfl.events.Event;
import haxe.io.Encoding;
import haxe.io.Bytes;
#if sys
import sys.net.Socket;
import sys.net.Host;
#end
import haxe.io.Error;
import haxe.crypto.Hmac;
import client.MessageTag;
import haxe.Timer;

class Client
{
    #if sys
    var socket:Socket;
    #end
    public var tag:MessageTag;
    //interact to be able to login to game
    public var login:Login;
    var data:String = "";
    var aliveTimer:Timer;
    var connected:Bool = false;
    public var compress:Int = 0;
    var tagRemove:Bool = false;
    var index:Int = 0;
    public var message:String->Void;
    public function new()
    {
        login = new Login();
    }
    public function update()
    {
        if (!connected) return;
        data = "";
        #if sys
        if(socket == null) return;
		try {
            if(compress == 0) data = socket.input.readLine();
		}catch(e:Dynamic)
		{
			if(e != "Blocking" && e != Error.Blocked && e != "Blocked")
			{
                //trace("e " + e);
			}
		}
        #end
        if(compress > 0)
        {
            processCompress();
        }else{
            if (data.length > 0) process();
        }
    }
    /*private function end()
    {
        switch(tag)
        {
            case PLAYER_UPDATE:
            if (Main.client.player != null) Main.client.player.update();
            default:
        }
    }*/
    private function processCompress()
    {
        #if sys
        if(tagRemove)
        {
            //remove #
            socket.input.readString(1);
            tagRemove = false;
        }
        #end
    }
    private function process()
    {
        //trace("data " + data);
        if(data.substring(0,1) == "#")
        {
            //behavior end #
            index = 0;
            tag = data.substring(1,data.length);
            return;
        }
        if(tag == "")
        {
            tag = data;
            return;
        }
        if(message != null) message(data);
    }
    public function alive()
    {
        send("KA 0 0");
    }
    public function send(data:String)
    {
        if (!connected) return;
        #if sys
        socket.output.writeString(data + "#");
        #end

        //alive timer
        if (aliveTimer != null) aliveTimer.stop();
        aliveTimer = new Timer(15 * 1000);
        aliveTimer.run = alive;
    }
    public function connect(ip:String="",port:Int=0)
	{
        connected = true;
        trace("attempt connect");
        #if sys
		var host:Host;
		try {
			host = new Host(ip);
		}catch(e:Dynamic)
		{
            trace("host e " + e);
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
        trace("connect sys");
        #end
	}
    public function close()
    {
        #if sys
        socket.close();
        trace("socket connected");
        #end
        connected = false;
    }
    /*private function unCompress()
    {
        #if sys
        if(tagRemove)
        {
            //remove #
            socket.input.readString(1);
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
    }*/
}