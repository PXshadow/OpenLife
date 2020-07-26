package openlife.server;

import openlife.client.ClientTag;
import sys.net.Socket;

class Connection
{
    public var running:Bool = false;
    var sock:Socket;
    var server:Server;
    var tag:ServerTag;
    public function new(sock:Socket,server:Server)
    {
        this.sock = sock;
        this.server = server;
        var challenge = "dsdjsiojdiasjiodsjiosd";
        var version = 350;
        send(SERVER_INFO,'0/0\n$challenge\n$version');
    }
    public function close()
    {

    }
    public function message(data:String)
    {
        trace('recieved: $data');
        var array = data.split("\n");
        if (array.length == 0) return;
        tag = array[0];
        var input = array.slice(1,array.length > 2 ? array.length - 1 : array.length);
        switch (tag)
        {
            case LOGIN:
            send(ACCEPTED,"");
            default:
            trace('tag not found $tag');
        }
    }
    private function send(tag:ClientTag,data:String)
    {
        sock.output.writeString('$tag $data#');
        trace('send $data');
    }
}