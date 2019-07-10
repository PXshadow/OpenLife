import sys.net.Socket;
import sys.net.Host;

class Output
{
    static function main()
    {
        var socket:Socket = new Socket();
        socket.connect(new Host("localhost"),2000);
        while(true)
        {
            Sys.print(socket.input.readLine() + "\n");
        }
    }
}