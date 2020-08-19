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
            Sys.sleep(1/15);
        }
    }
    var change:Int = 0;
    var ox:Int = 0;
    var oy:Int = 0;
    private function update()
    {
        for (connection in connections)
        {
            connection.send(FRAME);
            //x y new_floor_id new_id p_id old_x old_y speed
            if (tick % 15 == 0)
            {
                var rad = 3;
                var x = 0;
                var y = 0;
                if (change == -1) return;
                switch(change++)
                {
                    case 0:
                    x = rad;
                    y = rad;
                    case 1:
                    x = rad;
                    y = -rad;
                    case 2:
                    x = -rad;
                    y = -rad;
                    case 3:
                    x = -rad;
                    y = rad;
                    change = -1;
                }
                var id = 30;
                connection.send(MAP_CHANGE,['$x $y 0 $id 0 $ox $oy 10']);
                ox = x;
                oy = y;
            } 
            connection.send(FRAME);
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