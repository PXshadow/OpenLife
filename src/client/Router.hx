package client;

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
            message(socket.input.readLine());
        }catch(e:Dynamic)
        {
            
        }
    }
    public function send(string:String)
    {
        input.output.writeString(string + "\n");
    }
    public function sendCompress(bytes:Bytes)
    {
        input.output.writeBytes(bytes,0,bytes.length);
    }
}