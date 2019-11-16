package client;
#if sys
import haxe.io.Bytes;
import sys.net.Socket;
import sys.net.Host;

//relay data
class Router
{
    public var socket:Socket;
    public var input:Socket;
    public var port:Int;
    public var message:String->Void;
    public function new(port:Int=8005) 
    {
        this.port = port;
    }
    public function bind()
    {
        socket = new Socket();
        socket.bind(new Host("localhost"),port);
        socket.listen(1);
    }
    public function update()
    {
        try {
            message(input.input.readLine());
            trace("relay in");
        }catch(e:Dynamic)
        {
            //trace("e " + e);
        }
    }
    public function send(string:String)
    {
        if (input == null) return;
        trace("relay out " + string);
        input.output.writeString(string + "\n");
    }
    public function sendCompress(bytes:Bytes)
    {
        if (input == null) return;
        trace("relay out compressed bytes");
        input.output.writeBytes(bytes,0,bytes.length);
    }
}
#end