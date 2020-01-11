package client;
import haxe.io.Bytes;
#if (sys || nodejs)
import sys.net.Socket;
import sys.net.Host;
#end
import haxe.io.Error;
import haxe.crypto.Hmac;
import client.ClientTag;
import haxe.Timer;
/**
 * Socket Client
 */
class Client
{
    #if (sys || nodejs)
    var socket:Socket;
    #end
    //interact to be able to login to game
    var data:String = "";
    var aliveTimer:Timer;
    var connected:Bool = false;
    public var message:(tag:ClientTag,input:Array<String>)->Void;
    public var ip:String = "localhost";
    public var port:Int = 8005;
    //reconnect timer
    public var reconnect:Int = -1;
    //ping
    public var ping:Float = 0;
    var pingInt:Int = 0;
    //login info
    public var email:String = "";
    public var challenge:String = "";
    public var key:String = "";
    public var twin:String = "coding";
    public var tutorial:Bool = false;
    public var version:Int = 0;
    //functions
    public var accept:Void->Void;
    public var reject:Void->Void;


    var index:Int = 0;

    public function new()
    {

    }
    public function update()
    {
        if (!connected) 
        {
            trace("unconnected for update");
            return;
        }
        data = "";
        #if (sys || nodejs)
        if(socket == null) return;
		try {
            data = socket.input.readUntil("#".code);
            process();
		}catch(e:Dynamic)
		{
			if(e != Error.Blocked)
			{
                trace('e: $e');
                close();
			}
        }
        #end
    }
    private function process()
    {
        var array = data.split("\n");
        trace("array " + array);
        if (array.length == 0) return;
        trace("tag: " + array[0]);
        var tag:ClientTag = array[0];
        message(tag,array.slice(1,array.length));
    }
    public function alive()
    {
        send("KA 0 0");
        UnitTest.inital();
        send("PING 0 0 " + pingInt++);
    }
    public function login(tag:ClientTag,input:Array<String>) 
    {
        //login process
        switch(tag)
        {
            case SERVER_INFO:
            index = 0;
            for (data in input)
            {
                trace("data " + data);
                switch(index)
			    {
				    case 0:
				    //current
                    trace("amount " + data);
				    case 1:
				    //challenge
				    challenge = data;
				    case 2: 
				    //version
                    version = Std.parseInt(data);
                    trace("version " + version);
                    request();
                    return;
                }
                index++;
            }
            case ACCEPTED:
            trace("ACCEPTED LOGIN");
            if (accept != null) accept();
            case REJECTED:
            trace("REJECTED LOGIN");
            if (reject != null) reject();
            default:
            trace('$tag not registered');
            case null:
            trace('tag not found in data:\n$data');
        }
    }
    private function request()
    {
		key = StringTools.replace(key,"-","");
        //login
        var string = /*"client_openlife" +*/ email + " " +
		new Hmac(SHA1).make(Bytes.ofString("262f43f043031282c645d0eb352df723a3ddc88f")
		,Bytes.ofString(challenge,RawNative)).toHex() + " " +
		new Hmac(SHA1).make(Bytes.ofString(key)
		,Bytes.ofString(challenge)).toHex() +  " " +
        //tutorial 1 = true 0 = false
        (tutorial ? 1 : 0);
        //twin
        //login += " " + Sha1.make(Bytes.ofString(twin)).toHex() + " 1";

        send("LOGIN " + string);
    }
    public function send(data:String)
    {
        if (!connected) return;
        #if (sys || nodejs)
        try {
            #if nodejs
            @:privateAccess socket.s.write(data + "#");
            #else
            socket.output.writeString(data + "#");
            #end
        }catch(e:Dynamic)
        {
            trace("e " + e);
            close();
            return;
        }
        #end
        //alive timer
        if (aliveTimer != null) aliveTimer.stop();
        aliveTimer = new Timer(15 * 1000);
        aliveTimer.run = alive;
    }
    public function connect()
	{
        trace("attempt connect " + ip);
        connected = false;
        #if (sys || nodejs)
		var host:Host;
		try {
			host = new Host(ip);
		}catch(e:Dynamic)
		{
            trace("host e " + e);
			return;
		}
		socket = new Socket();
        //socket.setTimeout(99999999);
        #if !nodejs
		socket.setFastSend(true);
        #end
		try {
			socket.connect(host,port);
		}catch(e:Dynamic)
		{
            trace("e " + e);
            return;
		}
        #if !nodejs
        socket.setBlocking(false);
        #end
        connected = true;
        trace("connected");
        #end
	}
    public function close()
    {
        #if (sys || nodejs)
        try {
            socket.close();
        }catch(e:Dynamic) {trace("failure to close socket " + e);}
        //socket = null;
        //socket = new Socket();
        trace("socket disconnected");
        #end
        connected = false;
        if (aliveTimer != null) aliveTimer.stop();
    }
}