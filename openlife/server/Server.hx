package openlife.server;
import openlife.data.Pos;
#if ((target.threaded) && !cs)
import openlife.client.ClientTag;
import sys.thread.Thread;
import haxe.Timer;
import sys.db.Sqlite;
import haxe.io.Bytes;
import sys.net.Socket;
import openlife.settings.Settings;
import sys.net.Host;
import sys.FileSystem;
import sys.io.File;
import haxe.io.Path;

class Server
{
    public var connections:Array<Connection> = [];
    var tick:Int = 0;
    public static function main()
    {
        new Server();
    }
    public function new()
    {
        var thread = new ThreadServer(this,8005);
        Thread.create(function()
        {
            thread.create();
        });
        while (true)
        {
            update();
            tick++;
            Sys.sleep(1/20);
        }
    }
    private function update()
    {
        for (connection in connections)
        {
            connection.update();
        }
    }
    public function process(connection:Connection,string:String)
    {
        var array = string.split(" ");
        if (array.length == 0) return;
        var tag = array[0];
        var input = array.slice(1);
        message(connection,tag,input);
    }
    private function message(header:ServerHeader,tag:ServerTag,input:Array<String>)
    {
        switch (tag)
        {
            case LOGIN:
            header.login();
            case RLOGIN:
            header.rlogin();
            case MOVE:
            var x = Std.parseInt(input[0]);
            var y = Std.parseInt(input[1]);
            var seq = Std.parseInt(input[2].substr(1));
            input = input.slice(3);
            var moves:Array<Pos> = [];
            for (i in 0...Std.int(input.length/2))
            {
                moves.push(new Pos(Std.parseInt(input[i * 2]),Std.parseInt(input[i * 2 + 1])));
            }
            header.move(x,y,seq,moves);
            case KA:
            header.keepAlive();
            default:
        }
    }
}
#end