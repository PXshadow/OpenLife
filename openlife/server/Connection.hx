package openlife.server;

import sys.net.Socket;

class Connection
{
    public var running:Bool = false;
    var sock:Socket;
    var server:Server;
    public function new(sock:Socket,server:Server)
    {
        this.sock = sock;
        this.server = server;
    }
    public function close()
    {

    }
    public function message(input:String)
    {

    }
}