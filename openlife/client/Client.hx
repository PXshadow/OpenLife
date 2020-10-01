package openlife.client;
import haxe.Exception;
import openlife.settings.Settings.ConfigData;
import haxe.io.Bytes;
import openlife.client.ClientTag;

import sys.io.File;
import sys.net.Socket;
import sys.net.Host;

import haxe.io.Error;
import haxe.crypto.Hmac;
import haxe.Timer;
/**
 * Socket Client
 */
 @:expose
class Client
{
    var socket:Socket;
    //interact to be able to login to game
    var data:String = "";
    var aliveStamp:Float = 0;
    var connected:Bool = false;
    public var message:(tag:ClientTag,input:Array<String>)->Void;
    public var onClose:Void->Void;
    //ping
    public var ping:Int = 0;
    var pingInt:Int = 0;
    public var config:ConfigData;
    var challenge:String;
    public var version:String;
    public var reconnect:Bool = false;
    //functions
    public var accept:Void->Void;
    public var reject:Void->Void;
    public var relayIn:Socket;
    public var relayServer: #if sys Socket #else js.node.net.Server #end;
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
        if (relayIn != null)
        {
            //relay system embeded into client update
            try {
                var input = relayIn.input.readUntil("#".code);
                trace("client -> server: " + input);
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
                trace("data: " + data);
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
    }
    var listen:Int;
    public function relay(listen:Int)
    {
        this.listen = listen;
        Sys.println('waiting for connection on port $listen');
        #if nodejs
        relayServer = js.node.Net.createServer(function(c)
        {
            relayIn = new Socket();
            @:privateAccess relayIn.s = c;
            @:privateAccess relayIn.addBuffer();
            @:privateAccess relayIn.s.setNoDelay();
            relayIn.setBlocking(false);
            @:privateAccess relayIn.s.on(js.node.net.Socket.SocketEvent.End,function()
            {
                close();
            });
        });
        relayServer.listen(listen);
        Sys.println("node sync wait");
        sys.NodeSync.wait(function()
        {
            return relayIn != null;
        });
        #else
        relayServer = new Socket();
        relayServer.bind(new Host("localhost"),listen);
        relayServer.listen(1);

        relayIn = relayServer.accept();
        //here we are connected
        relayIn.setFastSend(true);
        relayIn.setBlocking(false);
        #end
    }
    var tag:ClientTag;
    private function process(wasCompressed:Bool)
    {
        //relay
        if (!wasCompressed && relayIn != null) 
        {
            relaySend(data);
        }
        //normal client
        var array = data.split("\n");
        if (array.length == 0) return;
        tag = array[0];
        message(tag,array.slice(1,array.length > 2 ? array.length - 1 : array.length));
    }
    private function compressProcess()
    {
        if (relayIn != null)
        {
            #if nodejs
            @:privateAccess relayIn.write(dataCompressed);
            #else
            relayIn.output.write(dataCompressed);
            #end
        }
    }
    public function alive()
    {
        send("KA 0 0");
        send("PING 0 0 " + pingInt++);
        aliveStamp = Timer.stamp();
    }
    public function login(tag:ClientTag,input:Array<String>) 
    {
        trace("login " + tag + " input " + input);
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
        var key = StringTools.replace(config.key,"-","");
        var email = config.email + (config.seed == "" ? "" : "|" + config.seed);
        var password = new Hmac(SHA1).make(Bytes.ofString("262f43f043031282c645d0eb352df723a3ddc88f"),Bytes.ofString(challenge)).toHex();
        var accountKey = new Hmac(SHA1).make(Bytes.ofString(key),Bytes.ofString(challenge)).toHex();
        var clientTag = " client_openlife";
        if (config.legacy) clientTag = "";
        var requestString = (reconnect ? "R" : "") + 'LOGIN$clientTag $email $password $accountKey ${(config.tutorial ? 1 : 0)}';
        send(requestString);
    }
    public function send(data:String)
    {
        if (!connected) return;
        try {
            #if nodejs
            @:privateAccess socket.write('$data#');
            #else
            socket.output.writeString('$data#');
            #end
        }catch(e:Dynamic) {
            trace("client send error: " + e);
            close();
            return;
        }
    }
    private function relaySend(data:String)
    {
        try {
            #if nodejs
            @:privateAccess relayIn.write('$data#');
            #else
            relayIn.output.writeString('$data#');
            #end
        }catch(e:Dynamic) {
            trace("client send error: " + e);
            close();
            return;
        }
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
        if (config == null)
        {
            trace("config is null");
            return;
        }
        this.reconnect = reconnect;
        if (config.port == null) config.port = 8005;
        if (config.tutorial == null) config.tutorial = false;
        if (config.legacy == null) config.legacy = false;
        if (config.seed == null) config.seed = "";
        if (config.twin == null) config.twin = "";
        if (config.email == null) config.email = "test@email.email";
        if (config.key == null) config.key = "8888-8888-8888-8888";
        trace("attempt connect " + config.ip + ":" + config.port);
        connected = false;
		var host:Host;
		try {
			host = new Host(config.ip);
		}catch(e:Dynamic)
		{
            trace("host error: " + e);
			return;
		}
        socket = new Socket();
        //socket.setTimeout(10);
		try {
			socket.connect(host,config.port);
		}catch(e:Dynamic)
		{
            trace("socket connect error: " + e);
            close();
            return;
        }
        #if nodejs
        @:privateAccess socket.s.setNoDelay();
        @:privateAccess socket.s.setEncoding('utf8');
        #end
        socket.setBlocking(false);
        connected = true;
        trace("connected");
	}
    public function close()
    {
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
        connected = false;
        if (onClose != null) onClose();
    }
}