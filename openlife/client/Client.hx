package openlife.client;
import haxe.Exception;
import openlife.settings.Settings.CredData;
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
    public var onClose:Void->Void;
    //ping
    public var ping:Int = 0;
    var pingInt:Int = 0;
    public var cred:CredData;
    var challenge:String;
    public var version:String;
    public var reconnect:Bool = false;
    //functions
    public var accept:Void->Void;
    public var reject:Void->Void;
    public var relayIn:Socket;
    public var relayServer:Socket;
    var wasCompressed:Bool = false;
    public function new()
    {
        aliveStamp = Timer.stamp();
    }
    public function update()
    {
        @:privateAccess haxe.MainLoop.tick(); //for timers
        if (Timer.stamp() - aliveStamp >= 15) alive();
        if (!connected) 
        {
            //trace("unconnected for update");
            return;
        }
        data = "";
        #if (sys || nodejs)
        if (relayIn != null)
        {
            //relay system embeded into client update
            try {
                var input = relayIn.input.readUntil("#".code);
                send(input);
            }catch(e:Exception)
            {
                if (e.message != "Blocked") close();
            }
        }
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
                }else{
                    return;
                }
            }else{
                data = socket.input.readUntil("#".code);
                //trace("data: " + data);
            }
		}catch(e:haxe.Exception)
		{
			if(e.message != "Blocked")
			{
                if(e.details().indexOf('Eof')>-1){
                    connected=false;
                    data="";
                    close();
                }else{
                    trace('e: ${e.details()}');
                    close();
                }
            }
            return;
        }
        process(wasCompressed);
        wasCompressed = false;
        update();
        #end
    }
    var listen:Int;
    public function relay(listen:Int)
    {
        this.listen = listen;
        relayServer = new Socket();
        relayServer.bind(new Host("localhost"),listen);
        relayServer.listen(1);
        Sys.println('waiting for connection on port $listen');
        relayIn = relayServer.accept();
        //here we are connected
        relayIn.setFastSend(true);
        relayIn.setBlocking(false);
    }
    var tag:ClientTag;
    private function process(wasCompressed:Bool)
    {
        //relay
        if (!wasCompressed && relayIn != null) 
        {
            relayIn.output.writeString('$data#');
        }
        //normal client
        var array = data.split("\n");
        if (array.length == 0) return;
        tag = array[0];
        message(tag,array.slice(1,array.length > 2 ? array.length - 1 : array.length));
    }
    private function compressProcess()
    {
        if (relayIn != null) relayIn.output.write(dataCompressed);
    }
    public function alive()
    {
        send("KA 0 0");
        send("PING 0 0 " + pingInt++);
        aliveStamp = Timer.stamp();
    }
    public function login(tag:ClientTag,input:Array<String>) 
    {
        //login process
        switch(tag)
        {
            case SERVER_INFO:
			
			//current
            //trace("amount " + input[0]);
			//challenge
			challenge = input[1];
			//version
            version = input[2];
            //trace("version " + version);
            request();
            case ACCEPTED:
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
        var key = StringTools.replace(cred.key,"-","");
        var email = cred.email + (cred.seed == "" ? "" : "|" + cred.seed);
        var password = new Hmac(SHA1).make(Bytes.ofString("262f43f043031282c645d0eb352df723a3ddc88f"),Bytes.ofString(challenge)).toHex();
        var accountKey = new Hmac(SHA1).make(Bytes.ofString(key),Bytes.ofString(challenge)).toHex();
        var clientTag = " client_openlife";
        if (cred.legacy) clientTag = "";
        var requestString = (reconnect ? "R" : "") + 'LOGIN$clientTag $email $password $accountKey ${(cred.tutorial ? 1 : 0)}';
        send(requestString);
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
        trace("attempt connect " + cred.ip + ":" + cred.port);
        connected = false;
        #if (sys || nodejs)
		var host:Host;
		try {
			host = new Host(cred.ip);
		}catch(e:Dynamic)
		{
            trace("host error: " + e);
			return;
		}
		socket = new Socket();
        //socket.setTimeout(10);
		try {
			socket.connect(host,cred.port);
		}catch(e:Dynamic)
		{
            trace("socket connect error: " + e);
            close();
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
            if (relayIn != null)
            {
                relayServer.close();
                relayIn.close();
            }
        }catch(e:Dynamic) 
        {
            trace("failure to close socket " + e);
        }
        trace("socket disconnected");
        #end
        connected = false;
        if (onClose != null) onClose();
    }
}