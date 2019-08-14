package client;
import haxe.io.Bytes;
#if sys
import sys.net.Socket;
import sys.net.Host;
#end
import haxe.io.Error;
import haxe.crypto.Hmac;
import client.ClientTag;
import haxe.Timer;

class Client
{
    #if sys
    var socket:Socket;
    #end
    public var tag:ClientTag;
    //interact to be able to login to game
    public var login:Login;
    var data:String = "";
    var dataCompress:Bytes;
    var aliveTimer:Timer;
    var connected:Bool = false;
    public var compress:Int = 0;
    var tagRemove:Bool = false;
    var index:Int = 0;
    public var message:String->Void;
    public var end:Void->Void;
    public var ip:String = "";
    public var port:Int = 0;
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
            if(compress == 0) 
            {
                data = socket.input.readLine();
            }
		}catch(e:Dynamic)
		{
			if(e != "Blocking" && e != Error.Blocked && e != "Blocked")
			{
                
			}
		}
        #end
        if(compress > 0)
        {
            if(dataCompress == null) 
            {
                dataCompress = Bytes.alloc(compress);
                tagRemove = true;
                index = 0;
            }
            processCompress();
        }else{
            if (data.length > 0) process();
        }
    }
    private function process()
    {
        if(data.substring(0,1) == "#")
        {
            //behavior end #
            if(end != null) end();
            index = 0;
            tag = data.substring(1,data.length);
            if (tag != FRAME && tag != HEAT_CHANGE) return;
        }
        if(tag == "")
        {
            tag = data;
            //login
            if(login != null)
            {
                if (tag == ACCEPTED && login.accept != null) login.accept();
                if (tag == REJECTED && login.reject != null) login.reject();
            }
            data = "";
            return;
        }
        //message out to state or login
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
    public function connect()
	{
        trace("attempt connect");
        connected = false;
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
        socket.setTimeout(99999999);
		socket.setBlocking(false);
		socket.setFastSend(true);
		try {
			socket.connect(host,port);
		}catch(e:Dynamic)
		{
            trace("e " + e);
            return;
		}
        connected = true;
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
    private function processCompress()
    {
        trace("compress");
        var temp:Bytes;
        #if sys
        if(tagRemove)
        {
            //remove #
            socket.input.readString(1);
            tagRemove = false;
        }
        if(index >= compress)
        {
            throw("map issue");
            tag = null;
            compress = 0;
            return;
        }
        //length - index
        temp = socket.input.read(compress - index);
        #end
        //blit into main compress
        dataCompress.blit(index,temp,0,temp.length);
        index += temp.length;
        if(index >= compress)
        {
            trace("index " + index + " compress " + compress + " dataCompress " + dataCompress.length);
            //finish data
            compress = 0;
            //unzip and send as normal message
            message(haxe.zip.Uncompress.run(dataCompress,dataCompress.length).toString());
            dataCompress = null;
            tag = null;
        }
    }
}