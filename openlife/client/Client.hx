package openlife.client;
import haxe.io.Bytes;
import openlife.client.ClientTag;
#if (sys || nodejs)
import sys.io.File;
import sys.net.Socket;
import sys.net.Host;
#end
import haxe.io.Error;
import haxe.crypto.Hmac;
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
    var aliveStamp:Float = 0;
    var connected:Bool = false;
    public var message:(tag:ClientTag,input:Array<String>)->Void;
    public var ip:String = "localhost";
    public var port:Int = 8005;
    //ping
    public var ping:Int = 0;
    var pingInt:Int = 0;
    //login info
    public var email:String = "";
    public var challenge:String = "";
    public var key:String = "";
    public var twin:String = "";
    public var tutorial:Bool = false;
    public var version:Int = 0;
    public var reconnect:Bool = false;
    //functions
    public var accept:Void->Void;
    public var reject:Void->Void;
    public var legacy:Bool = false;
    public var relay:Socket;
    var wasCompressed:Bool = false;
    public function new()
    {
        aliveStamp = Timer.stamp();
    }
    public function update()
    {
        if (Timer.stamp() - aliveStamp >= 15) alive();
        if (!connected) 
        {
            //trace("unconnected for update");
            return;
        }
        data = "";
        #if (sys || nodejs)
        if(socket == null) 
        {
            trace('socket is null');
            return;
        }
		try {
            if (compressSize > 0)
            {
                var temp = socket.input.read(compressSize - compressIndex);
                dataCompressed.blit(compressIndex,temp,0,temp.length);
                compressIndex += temp.length;
                if (compressIndex >= compressSize)
                {
                    compressProcess();
                    compressIndex = 0;
                    compressSize = 0;
                    data = haxe.zip.Uncompress.run(dataCompressed).toString();
                    wasCompressed = true;
                    if (tag == MAP_CHUNK)
                    {
                        data = '$MAP_CHUNK\n$data';
                    }
                    //data = '$tag\n$data';
                }else{
                    return;
                }
            }else{
                data = socket.input.readUntil("#".code);
            }
		}catch(e:Dynamic)
		{
			if(e != Error.Blocked)
			{
                trace('e: $e');
                close();
            }
            return;
        }
        process(wasCompressed);
        wasCompressed = false;
        #end
    }
    var tag:ClientTag;
    private function process(wasCompressed:Bool)
    {
        var array = data.split("\n");
        if (array.length == 0) return;
        tag = array[0];
        message(tag,array.slice(1,array.length > 2 ? array.length - 1 : array.length));
    }
    private function compressProcess()
    {

    }
    public function alive()
    {
        send("KA 0 0");
        send("PING 0 0 " + pingInt++);
        aliveStamp = Timer.stamp();
    }
    public function login(tag:ClientTag,input:Array<String>) 
    {
        trace('login tag: $tag $input');
        //login process
        switch(tag)
        {
            case SERVER_INFO:
			
			//current
            trace("amount " + input[0]);
			//challenge
			challenge = input[1];
			//version
            version = Std.parseInt(input[2]);
            trace("version " + version);
            request();
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
        var password = new Hmac(SHA1).make(Bytes.ofString("262f43f043031282c645d0eb352df723a3ddc88f"),Bytes.ofString(challenge,RawNative)).toHex();
        var accountKey = new Hmac(SHA1).make(Bytes.ofString(key),Bytes.ofString(challenge)).toHex();
        var clientTag = " client_openlife";
        if (legacy) clientTag = "";
        trace("request!");
        send((reconnect ? "R" : "") + 'LOGIN$clientTag $email $password $accountKey ${(tutorial ? 1 : 0)}');
    }
    public function send(data:String)
    {
        if (!connected) return;
        #if (sys || nodejs)
        try {
            #if nodejs
            @:privateAccess socket.s.write('$data#');
            #else
            socket.output.writeString('$data#');
            #end
        }catch(e:Dynamic)
        {
            trace("client send error: " + e);
            close();
            return;
        }
        #end
    }
    var compressIndex:Int = 0;
    var dataCompressed:Bytes;
    var compressSize:Int = 0;
    var rawSize:Int = 0;
    public function compress(rawSize:Int,compressSize:Int)
    {
        this.rawSize = rawSize;
        this.compressSize = compressSize;
        dataCompressed = Bytes.alloc(compressSize);
        compressIndex = 0;
    }
    public function connect(reconnect:Bool=false)
	{
        this.reconnect = reconnect;
        trace("attempt connect " + ip);
        connected = false;
        #if (sys || nodejs)
		var host:Host;
		try {
			host = new Host(ip);
		}catch(e:Dynamic)
		{
            trace("host error: " + e);
			return;
		}
		socket = new Socket();
        //socket.setTimeout(10);
		try {
			socket.connect(host,port);
		}catch(e:Dynamic)
		{
            trace("socket connect error: " + e);
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
    }
}