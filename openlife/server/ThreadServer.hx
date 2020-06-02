package openlife.server;
#if ((target.threaded) && !cs)
import sys.net.Socket;
import sys.thread.Thread;
import sys.thread.Mutex;
import sys.net.Host;
class ThreadServer
{
    public var socket:Socket;
    public var port:Int = 8005;
    public var maxCount:Int = -1;
    public var listenCount:Int = 10;
    public var messageBreak:Int = 4;
    public static inline var setTimeout:Int = 30;
    public function new()
    {
        socket = new Socket();
    }
    public function create()
    {
        socket.bind(new Host("0.0.0.0"),port);
        socket.listen(listenCount);
        while (true) 
        {
            Thread.create(connection).sendMessage(socket.accept());
            //new connection function to run tasks
            Thread.create(newConnection);
        }
    }
    private function connection()
    {
        var socket:Socket = cast Thread.readMessage(true);
        socket.setBlocking(false);
        socket.setFastSend(true);
        socket.setTimeout(30);
        socket.custom = {bool:true,timeout:setTimeout};
        var message:String = "";
        if (connect != null) connect(socket);
        while (socket.custom.bool)
        {
            try {
                message = socket.input.readUntil(messageBreak);
            }catch(e:Dynamic)
            {
                if (e != haxe.io.Error.Blocked)
                {
                    trace("server e " + e);
                    //if (e == Eof || Std.is(e,Eof))
                    //close socket
                    break;
                }else{
                    Sys.sleep(1);
                    //trace("timeout " + socket.custom.timeout);
                    if (socket.custom.timeout-- <= 0)
                    {
                        socket.custom.timeout = setTimeout;
                        timeout(socket);
                    }
                    continue;
                }
            }
            if (!this.message(socket,message)) break;
        }
        //close client
        disconnect(socket);
        socket.close();
    }
    public function newConnection() {}
    public function timeout(socket:Socket) {}
    public function message(socket:Socket,input:String):Bool {return true;}
    public function connect(socket:Socket) {}
    public function disconnect(socket:Socket) {}
    public function send(socket:Socket,output:String)
    {
        try {
            Sys.println('send    : $output');
            socket.output.writeString(output + String.fromCharCode(messageBreak));
        }catch(e:Dynamic)
        {
            trace("e " + e);
            socket.custom.bool = false;
        }
    }
}
#end